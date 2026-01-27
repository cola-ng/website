/**
 * Global AudioContext manager
 *
 * Creates a single AudioContext instance that is reused across the app.
 * The context is created on first user interaction and persists for the session.
 */

// Global singleton AudioContext
let globalAudioContext: AudioContext | null = null
let currentSource: AudioBufferSourceNode | null = null
let audioQueue: { data: ArrayBuffer; id: string; onStart?: () => void; onEnd?: () => void }[] = []
let isProcessingQueue = false
let currentPlayingId: string | null = null

// Callbacks for state changes
type PlayingStateCallback = (id: string | null) => void
const playingStateCallbacks: Set<PlayingStateCallback> = new Set()

/**
 * Get or create the global AudioContext
 */
export function getAudioContext(): AudioContext {
  if (!globalAudioContext) {
    globalAudioContext = new AudioContext()
  }
  return globalAudioContext
}

/**
 * Ensure AudioContext is running (call on user interaction)
 * Returns the AudioContext instance
 */
export async function ensureAudioContextRunning(): Promise<AudioContext> {
  const ctx = getAudioContext()
  if (ctx.state === 'suspended') {
    await ctx.resume()
  }
  return ctx
}

/**
 * Check if AudioContext exists and is running
 */
export function isAudioContextReady(): boolean {
  return globalAudioContext !== null && globalAudioContext.state === 'running'
}

/**
 * Subscribe to playing state changes
 */
export function onPlayingStateChange(callback: PlayingStateCallback): () => void {
  playingStateCallbacks.add(callback)
  return () => playingStateCallbacks.delete(callback)
}

/**
 * Get current playing audio ID
 */
export function getCurrentPlayingId(): string | null {
  return currentPlayingId
}

/**
 * Set current playing ID and notify subscribers
 */
function setCurrentPlayingId(id: string | null) {
  currentPlayingId = id
  playingStateCallbacks.forEach(cb => cb(id))
}

/**
 * Stop current audio playback
 */
export function stopAudio() {
  if (currentSource) {
    try {
      currentSource.stop()
    } catch {
      // Already stopped
    }
    currentSource = null
  }
  audioQueue = []
  isProcessingQueue = false
  setCurrentPlayingId(null)
}

/**
 * Process the audio queue
 */
async function processAudioQueue() {
  if (isProcessingQueue || audioQueue.length === 0) {
    return
  }

  isProcessingQueue = true
  const item = audioQueue.shift()

  if (!item) {
    isProcessingQueue = false
    return
  }

  try {
    const ctx = await ensureAudioContextRunning()
    const audioBuffer = await ctx.decodeAudioData(item.data.slice(0))

    // Stop previous source if playing
    if (currentSource) {
      try {
        currentSource.stop()
      } catch {
        // Already stopped
      }
    }

    // Create and play source
    const source = ctx.createBufferSource()
    source.buffer = audioBuffer
    source.connect(ctx.destination)
    currentSource = source
    setCurrentPlayingId(item.id)
    item.onStart?.()

    source.onended = () => {
      setCurrentPlayingId(null)
      currentSource = null
      item.onEnd?.()
      isProcessingQueue = false
      // Process next item
      processAudioQueue()
    }

    source.start(0)
  } catch (err) {
    console.error('Failed to play audio:', err)
    setCurrentPlayingId(null)
    isProcessingQueue = false
    // Try next item
    processAudioQueue()
  }
}

/**
 * Queue audio for playback
 */
export function queueAudio(
  arrayBuffer: ArrayBuffer,
  id: string,
  options?: { onStart?: () => void; onEnd?: () => void }
) {
  audioQueue.push({
    data: arrayBuffer,
    id,
    onStart: options?.onStart,
    onEnd: options?.onEnd
  })
  processAudioQueue()
}

/**
 * Play audio immediately (stops current and clears queue)
 */
export function playAudioNow(
  arrayBuffer: ArrayBuffer,
  id: string,
  options?: { onStart?: () => void; onEnd?: () => void }
) {
  stopAudio()
  queueAudio(arrayBuffer, id, options)
}

/**
 * Fetch audio from URL and queue for playback
 */
export async function fetchAndQueueAudio(
  url: string,
  id: string,
  token?: string,
  options?: { onStart?: () => void; onEnd?: () => void }
): Promise<void> {
  const headers: HeadersInit = {}
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  const response = await fetch(url, { headers })
  if (!response.ok) {
    throw new Error(`Failed to fetch audio: ${response.status}`)
  }

  const arrayBuffer = await response.arrayBuffer()
  queueAudio(arrayBuffer, id, options)
}

/**
 * Convert base64 to ArrayBuffer and queue for playback
 */
export function queueAudioFromBase64(
  base64: string,
  id: string,
  options?: { onStart?: () => void; onEnd?: () => void }
) {
  const binaryString = atob(base64)
  const bytes = new Uint8Array(binaryString.length)
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i)
  }
  queueAudio(bytes.buffer, id, options)
}
