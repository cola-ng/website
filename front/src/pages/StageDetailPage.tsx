import { useState, useEffect, useRef, useCallback } from 'react'
import { useParams, useNavigate, useSearchParams, Link } from 'react-router-dom'
import {
  ArrowLeft,
  RotateCcw,
  Volume2,
  Mic,
  MicOff,
  Play,
  BookOpen,
  Users,
  Loader2,
  ArrowLeftRight,
} from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { cn } from '../lib/utils'
import { useAuth } from '../lib/auth'
import { ensureAudioContextRunning, stopAudio as globalStopAudio, onPlayingStateChange } from '../lib/audio'
import { evaluatePronunciation } from '../lib/api'

// Stage matches backend asset_stages table
interface Stage {
  id: number
  name_en: string
  name_zh: string
  description_en: string | null
  description_zh: string | null
  icon_emoji: string | null
  display_order: number | null
  difficulty: number | null
  is_active: boolean | null
  created_at: string
}

// Script matches backend asset_scripts table (dialogues)
interface Script {
  id: number
  stage_id: number
  title_en: string
  title_zh: string
  description_en: string | null
  description_zh: string | null
  total_turns: number | null
  estimated_duration_seconds: number | null
  difficulty: number | null
  created_at: string
}

// ScriptTurn matches backend asset_script_turns table
interface ScriptTurn {
  id: number
  script_id: number
  turn_number: number
  speaker_role: string
  speaker_name: string | null
  content_en: string
  content_zh: string
  phonetic_transcription: string | null
  asset_phrases: unknown | null
  notes: string | null
}

// User's recording result (in memory)
interface RecordingResult {
  turnId: number
  audioBlob: Blob
  audioUrl: string
  score?: number
  feedback?: string
}

// Stored recording in localStorage (base64 encoded)
interface StoredRecording {
  turnId: number
  audioBase64: string
  mimeType: string
  score?: number
  feedback?: string
}

// Helper: Convert Blob to base64
async function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onloadend = () => {
      const base64 = (reader.result as string).split(',')[1]
      resolve(base64)
    }
    reader.onerror = reject
    reader.readAsDataURL(blob)
  })
}

// Helper: Convert base64 to Blob
function base64ToBlob(base64: string, mimeType: string): Blob {
  const binaryString = atob(base64)
  const bytes = new Uint8Array(binaryString.length)
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i)
  }
  return new Blob([bytes], { type: mimeType })
}

// Helper: Get localStorage key for recordings
function getRecordingsKey(scriptId: number): string {
  return `stage_recordings_${scriptId}`
}

// Helper: Load recordings from localStorage
function loadRecordingsFromStorage(scriptId: number): Map<number, RecordingResult> {
  const map = new Map<number, RecordingResult>()
  try {
    const stored = localStorage.getItem(getRecordingsKey(scriptId))
    if (stored) {
      const recordings: StoredRecording[] = JSON.parse(stored)
      for (const rec of recordings) {
        const blob = base64ToBlob(rec.audioBase64, rec.mimeType)
        map.set(rec.turnId, {
          turnId: rec.turnId,
          audioBlob: blob,
          audioUrl: URL.createObjectURL(blob),
          score: rec.score,
          feedback: rec.feedback,
        })
      }
    }
  } catch (e) {
    console.error('Failed to load recordings from localStorage:', e)
  }
  return map
}

// Helper: Save recordings to localStorage
async function saveRecordingsToStorage(scriptId: number, recordings: Map<number, RecordingResult>) {
  try {
    const stored: StoredRecording[] = []
    for (const [, rec] of recordings) {
      const base64 = await blobToBase64(rec.audioBlob)
      stored.push({
        turnId: rec.turnId,
        audioBase64: base64,
        mimeType: rec.audioBlob.type,
        score: rec.score,
        feedback: rec.feedback,
      })
    }
    localStorage.setItem(getRecordingsKey(scriptId), JSON.stringify(stored))
  } catch (e) {
    console.error('Failed to save recordings to localStorage:', e)
  }
}

// Helper: Convert audio blob to WAV format using Web Audio API
async function convertToWav(audioBlob: Blob): Promise<Blob> {
  const audioContext = new AudioContext()
  const arrayBuffer = await audioBlob.arrayBuffer()

  try {
    const audioBuffer = await audioContext.decodeAudioData(arrayBuffer)
    const numberOfChannels = audioBuffer.numberOfChannels
    const sampleRate = audioBuffer.sampleRate
    const length = audioBuffer.length

    const wavBuffer = new ArrayBuffer(44 + length * numberOfChannels * 2)
    const view = new DataView(wavBuffer)

    // WAV header
    view.setUint8(0, 0x52); view.setUint8(1, 0x49); view.setUint8(2, 0x46); view.setUint8(3, 0x46)
    view.setUint32(4, 36 + length * numberOfChannels * 2, true)
    view.setUint8(8, 0x57); view.setUint8(9, 0x41); view.setUint8(10, 0x56); view.setUint8(11, 0x45)
    view.setUint8(12, 0x66); view.setUint8(13, 0x6d); view.setUint8(14, 0x74); view.setUint8(15, 0x20)
    view.setUint32(16, 16, true)
    view.setUint16(20, 1, true)
    view.setUint16(22, numberOfChannels, true)
    view.setUint32(24, sampleRate, true)
    view.setUint32(28, sampleRate * numberOfChannels * 2, true)
    view.setUint16(32, numberOfChannels * 2, true)
    view.setUint16(34, 16, true)
    view.setUint8(36, 0x64); view.setUint8(37, 0x61); view.setUint8(38, 0x74); view.setUint8(39, 0x61)
    view.setUint32(40, length * numberOfChannels * 2, true)

    let offset = 44
    for (let i = 0; i < length; i++) {
      for (let channel = 0; channel < numberOfChannels; channel++) {
        const sample = audioBuffer.getChannelData(channel)[i]
        const s = Math.max(-1, Math.min(1, sample))
        view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7FFF, true)
        offset += 2
      }
    }

    await audioContext.close()
    return new Blob([wavBuffer], { type: 'audio/wav' })
  } catch (err) {
    await audioContext.close()
    throw err
  }
}

export function StageDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const { token } = useAuth() // Ensure user is authenticated

  const [stage, setStage] = useState<Stage | null>(null)
  const [scripts, setScripts] = useState<Script[]>([])
  const [selectedScript, setSelectedScript] = useState<Script | null>(null)
  const [turns, setTurns] = useState<ScriptTurn[]>([])
  const [activeTurnIndex, setActiveTurnIndex] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Role selection - which role the user is playing
  const [selectedRole, setSelectedRole] = useState<string | null>(null)

  // Recording state
  const [isRecording, setIsRecording] = useState(false)
  const [recordingDuration, setRecordingDuration] = useState(0)
  const [recordings, setRecordings] = useState<Map<number, RecordingResult>>(new Map())
  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // Audio playback state
  const [playingAudioId, setPlayingAudioId] = useState<string | null>(null)
  const audioRef = useRef<HTMLAudioElement | null>(null)

  // Practice session state
  const [isStarted, setIsStarted] = useState(false)
  // Current turn index - tracks dialogue progress (which turn we're at)
  const [currentTurnIndex, setCurrentTurnIndex] = useState<number>(0)
  // Whether we're waiting for AI audio to finish
  const [isPlayingAiTurn, setIsPlayingAiTurn] = useState(false)
  // Whether we're evaluating pronunciation
  const [isEvaluating, setIsEvaluating] = useState(false)

  // Subscribe to global audio playing state
  useEffect(() => {
    return onPlayingStateChange((id) => {
      setPlayingAudioId(id)
    })
  }, [])

  // Display settings (persisted to localStorage)
  const [showEn, setShowEn] = useState(() => {
    const saved = localStorage.getItem('stage_showEn')
    return saved !== null ? saved === 'true' : true
  })
  const [showZh, setShowZh] = useState(() => {
    const saved = localStorage.getItem('stage_showZh')
    return saved !== null ? saved === 'true' : true
  })

  // Persist display settings
  useEffect(() => {
    localStorage.setItem('stage_showEn', String(showEn))
    localStorage.setItem('stage_showZh', String(showZh))
  }, [showEn, showZh])

  // Fetch stage data
  useEffect(() => {
    async function fetchStage() {
      if (!id) return
      try {
        setLoading(true)
        const response = await fetch(`/api/asset/stages/${id}`)
        if (!response.ok) throw new Error('Failed to fetch stage')
        const data = await response.json()
        setStage(data)
      } catch (err) {
        setError('Failed to load stage')
        console.error(err)
      } finally {
        setLoading(false)
      }
    }
    fetchStage()
  }, [id])

  // Fetch scripts for this stage
  useEffect(() => {
    async function fetchScripts() {
      if (!id) return
      try {
        const response = await fetch(`/api/asset/stages/${id}/scripts`)
        if (!response.ok) throw new Error('Failed to fetch scripts')
        const data = await response.json()
        setScripts(data)
        if (data.length > 0) {
          // Check URL for script parameter on initial load
          const scriptIdFromUrl = searchParams.get('script')
          const targetScript = scriptIdFromUrl
            ? data.find((s: Script) => s.id === Number(scriptIdFromUrl))
            : null
          setSelectedScript(targetScript || data[0])
        }
      } catch (err) {
        console.error('Failed to fetch scripts:', err)
      }
    }
    fetchScripts()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id])

  // Fetch turns for selected script
  useEffect(() => {
    async function fetchTurns() {
      if (!selectedScript) return
      try {
        const response = await fetch(`/api/asset/scripts/${selectedScript.id}/turns`)
        if (!response.ok) throw new Error('Failed to fetch turns')
        const data: ScriptTurn[] = await response.json()
        setTurns(data)
        // Load recordings from localStorage for this script
        setRecordings(loadRecordingsFromStorage(selectedScript.id))
        setIsStarted(false) // Reset start state when script changes
        setCurrentTurnIndex(0)
        setIsPlayingAiTurn(false)
        // Auto-select first role as the user's role
        const uniqueRoles = [...new Set(data.map(t => t.speaker_role))]
        const defaultRole = uniqueRoles.find(r => r === 'user') || uniqueRoles[0] || null
        setSelectedRole(defaultRole)
        // Auto-select first turn of that role
        const firstUserTurnIndex = data.findIndex(t => t.speaker_role === defaultRole)
        setActiveTurnIndex(firstUserTurnIndex >= 0 ? firstUserTurnIndex : null)
      } catch (err) {
        console.error('Failed to fetch turns:', err)
      }
    }
    fetchTurns()
  }, [selectedScript])

  // Get unique roles from turns
  const roles = turns.reduce((acc, turn) => {
    if (!acc.find(r => r.role === turn.speaker_role)) {
      acc.push({
        role: turn.speaker_role,
        name: turn.speaker_name || (turn.speaker_role === 'user' ? '你' : 'AI'),
      })
    }
    return acc
  }, [] as { role: string; name: string }[])

  // Play audio (for manual playback)
  const playAudio = useCallback(async (audioId: string, audioSource: string | Blob) => {
    // Stop any currently playing audio
    if (audioRef.current) {
      audioRef.current.pause()
      audioRef.current = null
    }

    if (playingAudioId === audioId) {
      setPlayingAudioId(null)
      return
    }

    try {
      const audio = new Audio(
        typeof audioSource === 'string'
          ? audioSource
          : URL.createObjectURL(audioSource)
      )
      audioRef.current = audio
      setPlayingAudioId(audioId)

      audio.onended = () => {
        setPlayingAudioId(null)
        if (typeof audioSource !== 'string') {
          URL.revokeObjectURL(audio.src)
        }
      }
      audio.onerror = () => {
        setPlayingAudioId(null)
        console.error('Audio playback failed')
      }
      await audio.play()
    } catch (err) {
      console.error('Failed to play audio:', err)
      setPlayingAudioId(null)
    }
  }, [playingAudioId])

  // Play AI turn audio and advance to next turn when finished
  const playAiTurnAudio = useCallback(async (turnIndex: number, onComplete?: () => void) => {
    if (turnIndex >= turns.length) {
      onComplete?.()
      return
    }

    const turn = turns[turnIndex]
    if (turn.speaker_role === selectedRole) {
      // This is user's turn, don't play audio
      onComplete?.()
      return
    }

    setIsPlayingAiTurn(true)
    try {
      const audioUrl = `/api/asset/scripts/turns/${turn.id}/audio`
      const response = await fetch(audioUrl)
      if (response.ok) {
        const arrayBuffer = await response.arrayBuffer()
        // Create audio element for playback with onended callback
        const audioContext = new AudioContext()
        const audioBuffer = await audioContext.decodeAudioData(arrayBuffer)
        const source = audioContext.createBufferSource()
        source.buffer = audioBuffer
        source.connect(audioContext.destination)

        source.onended = () => {
          setIsPlayingAiTurn(false)
          setPlayingAudioId(null)
          audioContext.close()
          onComplete?.()
        }

        setPlayingAudioId(`original-${turn.id}`)
        source.start()
      } else {
        setIsPlayingAiTurn(false)
        onComplete?.()
      }
    } catch (err) {
      console.error('Failed to play AI turn audio:', err)
      setIsPlayingAiTurn(false)
      onComplete?.()
    }
  }, [turns, selectedRole])

  // Advance dialogue to next turn, playing AI audio if needed
  const advanceToNextTurn = useCallback((fromIndex: number) => {
    const nextIndex = fromIndex + 1
    if (nextIndex >= turns.length) {
      // Dialogue complete
      setCurrentTurnIndex(turns.length)
      setActiveTurnIndex(null)
      return
    }

    setCurrentTurnIndex(nextIndex)
    const nextTurn = turns[nextIndex]

    if (nextTurn.speaker_role === selectedRole) {
      // Next turn is user's turn - set active and wait for recording
      setActiveTurnIndex(nextIndex)
    } else {
      // Next turn is AI's turn - play audio then advance further
      setActiveTurnIndex(null) // No active turn while AI is speaking
      playAiTurnAudio(nextIndex, () => {
        // After AI finishes, advance to the next turn
        advanceToNextTurn(nextIndex)
      })
    }
  }, [turns, selectedRole, playAiTurnAudio])

  // Handle start practice - activates AudioContext and starts dialogue from turn 0
  const handleStart = useCallback(async () => {
    // Activate AudioContext on user interaction
    await ensureAudioContextRunning()

    // Clear all previous recordings for this script
    if (selectedScript) {
      localStorage.removeItem(getRecordingsKey(selectedScript.id))
      setRecordings(new Map())
    }

    setIsStarted(true)
    setCurrentTurnIndex(0)

    if (turns.length === 0) return

    const firstTurn = turns[0]
    if (firstTurn.speaker_role === selectedRole) {
      // First turn is user's turn - set active and wait for recording
      setActiveTurnIndex(0)
    } else {
      // First turn is AI's turn - play audio then advance
      setActiveTurnIndex(null)
      playAiTurnAudio(0, () => {
        advanceToNextTurn(0)
      })
    }
  }, [turns, selectedRole, selectedScript, playAiTurnAudio, advanceToNextTurn])

  // Start recording
  const startRecording = useCallback(async (turnIndex: number) => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/mp4'
      })

      mediaRecorderRef.current = mediaRecorder
      audioChunksRef.current = []

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data)
        }
      }

      mediaRecorder.onstop = () => {
        stream.getTracks().forEach(track => track.stop())
        if (recordingTimerRef.current) {
          clearInterval(recordingTimerRef.current)
          recordingTimerRef.current = null
        }
        setRecordingDuration(0)

        const audioBlob = new Blob(audioChunksRef.current, { type: mediaRecorder.mimeType })
        if (audioBlob.size > 0) {
          const turn = turns[turnIndex]

          // Evaluate pronunciation using backend API
          const evaluateAndSave = async () => {
            setIsEvaluating(true)
            let score: number | undefined
            let feedback: string | undefined

            try {
              // Convert to WAV for API
              const wavBlob = await convertToWav(audioBlob)
              const audioBase64 = await blobToBase64(wavBlob)

              if (token) {
                const response = await evaluatePronunciation(token, audioBase64, turn.content_en)
                score = response.overall_score
                // Combine feedback items into a string
                feedback = response.feedback?.map(f => f.message).join(' ') || '发音不错！'
              }
            } catch (err) {
              console.error('Failed to evaluate pronunciation:', err)
              // Fallback to no score if evaluation fails
            } finally {
              setIsEvaluating(false)
            }

            const result: RecordingResult = {
              turnId: turn.id,
              audioBlob,
              audioUrl: URL.createObjectURL(audioBlob),
              score,
              feedback,
            }
            setRecordings(prev => {
              const next = new Map(prev).set(turn.id, result)
              // Save to localStorage
              if (selectedScript) {
                saveRecordingsToStorage(selectedScript.id, next)
              }
              return next
            })
            // Advance to next turn in dialogue (may play AI audio)
            advanceToNextTurn(turnIndex)
          }

          evaluateAndSave()
        }
      }

      mediaRecorder.start()
      setIsRecording(true)
      setActiveTurnIndex(turnIndex)
      setRecordingDuration(0)

      recordingTimerRef.current = setInterval(() => {
        setRecordingDuration(prev => prev + 1)
      }, 1000)
    } catch (err) {
      console.error('Failed to start recording:', err)
      alert('无法访问麦克风。请确保已授予麦克风权限。')
    }
  }, [turns, selectedScript, token, advanceToNextTurn])

  // Stop recording
  const stopRecording = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop()
    }
    setIsRecording(false)
  }, [])

  // Keyboard shortcut: Space to toggle recording
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if user is typing in an input field
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return
      // Don't trigger if AI is playing
      if (isPlayingAiTurn) return
      if (e.code === 'Space' && activeTurnIndex !== null) {
        e.preventDefault()
        if (isRecording) {
          stopRecording()
        } else {
          startRecording(activeTurnIndex)
        }
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [activeTurnIndex, isRecording, isPlayingAiTurn, startRecording, stopRecording])

  // Handle restart
  const handleRestart = () => {
    setRecordings(new Map())
    setActiveTurnIndex(null)
    setCurrentTurnIndex(0)
    setIsStarted(false)
    setIsPlayingAiTurn(false)
    // Clear localStorage recordings for this script
    if (selectedScript) {
      localStorage.removeItem(getRecordingsKey(selectedScript.id))
    }
    // Stop any playing audio using global audio manager
    globalStopAudio()
  }

  // Calculate progress
  // User turns are the turns for the role the user is playing
  const userTurns = turns.filter(t => t.speaker_role === selectedRole)
  const completedUserTurns = userTurns.filter(t => recordings.has(t.id))
  const progress = userTurns.length > 0 ? (completedUserTurns.length / userTurns.length) * 100 : 0

  // Handle role switch
  const handleSwitchRole = () => {
    if (roles.length < 2) return
    const currentRoleIndex = roles.findIndex(r => r.role === selectedRole)
    const nextRoleIndex = (currentRoleIndex + 1) % roles.length
    const newRole = roles[nextRoleIndex].role
    setSelectedRole(newRole)
    setRecordings(new Map()) // Clear recordings when switching roles
    setCurrentTurnIndex(0)
    setIsStarted(false)
    setIsPlayingAiTurn(false)
    // Clear localStorage recordings for this script
    if (selectedScript) {
      localStorage.removeItem(getRecordingsKey(selectedScript.id))
    }
    // Reset active turn
    setActiveTurnIndex(null)
    // Stop any playing audio
    globalStopAudio()
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <main className="mx-auto max-w-6xl p-4">
          <div className="flex items-center justify-center h-64">
            <Loader2 className="h-12 w-12 animate-spin text-orange-500" />
          </div>
        </main>
      </div>
    )
  }

  if (error || !stage) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <main className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">😕</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">场景未找到</h1>
            <p className="text-gray-600 mb-6">{error || '无法加载场景内容'}</p>
            <Button asChild>
              <Link to="/stages">返回角色扮演</Link>
            </Button>
          </div>
        </main>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50 flex flex-col">
      <Header />

      <main className="flex-1 mx-auto w-full max-w-6xl px-4 py-4">
        {/* Compact header */}
        <div className="flex items-center justify-between mb-4">
          <button
            onClick={() => navigate('/stages')}
            className="flex items-center gap-2 text-gray-600 hover:text-gray-900"
          >
            <ArrowLeft className="h-4 w-4" />
            <span>返回角色扮演</span>
          </button>
          <div className="flex items-center gap-3">
            <span className="text-2xl">{stage.icon_emoji || '📚'}</span>
            <div>
              <h1 className="font-semibold text-gray-900">{stage.name_zh}</h1>
              <p className="text-sm text-gray-500">{stage.name_en}</p>
            </div>
          </div>
          {/* Language toggles */}
          <div className="flex items-center gap-2 text-sm">
            <span className="text-gray-500">显示:</span>
            <button
              onClick={() => setShowEn(!showEn)}
              className={cn(
                'px-2 py-0.5 text-xs font-medium rounded-full transition-all',
                showEn
                  ? 'bg-orange-500 text-white shadow-sm'
                  : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
              )}
            >
              英
            </button>
            <button
              onClick={() => setShowZh(!showZh)}
              className={cn(
                'px-2 py-0.5 text-xs font-medium rounded-full transition-all',
                showZh
                  ? 'bg-orange-500 text-white shadow-sm'
                  : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
              )}
            >
              中
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-4 gap-4 h-[calc(100vh-180px)]">
          {/* Left: Script list */}
          <div className="lg:col-span-1 bg-white rounded-xl shadow-lg overflow-hidden flex flex-col">
            <div className="p-3 border-b bg-gray-50">
              <h2 className="font-semibold text-gray-900 flex items-center gap-2">
                <BookOpen className="h-4 w-4 text-orange-500" />
                对话列表
              </h2>
            </div>
            <div className="flex-1 overflow-y-auto p-2 space-y-1">
              {scripts.map((script) => (
                <button
                  key={script.id}
                  onClick={() => {
                    setSelectedScript(script)
                    setSearchParams({ script: String(script.id) })
                  }}
                  className={cn(
                    'w-full text-left p-2 rounded-lg transition-colors text-sm',
                    selectedScript?.id === script.id
                      ? 'bg-orange-100 border border-orange-300'
                      : 'hover:bg-gray-50'
                  )}
                >
                  <div className="font-medium text-gray-900">{script.title_zh}</div>
                  <div className="text-xs text-gray-500">{script.title_en}</div>
                  <div className="text-xs text-gray-400 mt-1">
                    {script.total_turns || 0} 轮 • {Math.ceil((script.estimated_duration_seconds || 60) / 60)} 分钟
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Right: Dialogue content */}
          <div className="lg:col-span-3 bg-white rounded-xl shadow-lg overflow-hidden flex flex-col">
            {selectedScript && turns.length > 0 ? (
              <>
                {/* Script info and roles */}
                <div className="p-4 border-b bg-gray-50">
                  <div className="flex items-start justify-between">
                    <div>
                      <h2 className="font-semibold text-gray-900">{selectedScript.title_zh}</h2>
                      <p className="text-sm text-gray-500">{selectedScript.description_zh || selectedScript.description_en}</p>
                    </div>
                    <div className="flex items-center gap-4">
                      {/* Roles with switch button - selected role always on right */}
                      <div className="flex items-center gap-1">
                        <Users className="h-4 w-4 text-gray-400 mr-1" />
                        {/* Sort roles: non-selected first, selected (你) last */}
                        {[...roles]
                          .sort((a, _b) => (a.role === selectedRole ? 1 : -1))
                          .map((r, idx, arr) => (
                            <div key={r.role} className="flex items-center">
                              <span
                                className={cn(
                                  'px-2 py-0.5 rounded-full text-xs font-medium',
                                  r.role === selectedRole
                                    ? 'bg-blue-100 text-blue-700'
                                    : 'bg-orange-100 text-orange-700'
                                )}
                              >
                                {r.name} {r.role === selectedRole && '(你)'}
                              </span>
                              {/* Switch button between roles */}
                              {idx < arr.length - 1 && (
                                <button
                                  onClick={handleSwitchRole}
                                  className="mx-1 p-1 rounded-full bg-gray-100 hover:bg-gray-200 transition-colors"
                                  title="切换角色"
                                >
                                  <ArrowLeftRight className="h-3 w-3 text-gray-500" />
                                </button>
                              )}
                            </div>
                          ))}
                      </div>
                      {/* Progress */}
                      <div className="text-sm text-gray-500">
                        进度: {completedUserTurns.length}/{userTurns.length}
                      </div>
                      {/* Restart button */}
                      <Button variant="outline" size="sm" onClick={handleRestart}>
                        <RotateCcw className="h-3 w-3 mr-1" />
                        重新开始
                      </Button>
                    </div>
                  </div>
                  {/* Progress bar */}
                  <div className="mt-3 h-1.5 bg-gray-200 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-orange-500 to-amber-500 transition-all duration-300"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                </div>

                {/* Dialogue transcript */}
                <div className="flex-1 overflow-y-auto p-4 space-y-3 relative">
                  {turns.map((turn, index) => {
                    const isUserRole = turn.speaker_role === selectedRole
                    const recording = recordings.get(turn.id)
                    const isActive = activeTurnIndex === index
                    const bothOff = !showEn && !showZh

                    return (
                      <div
                        key={turn.id}
                        className={cn(
                          'flex gap-3',
                          isUserRole ? 'flex-row-reverse' : ''
                        )}
                      >
                        {/* Avatar column with speaker buttons below */}
                        <div className="flex flex-col items-center gap-1 flex-shrink-0">
                          {/* Avatar */}
                          <div
                            className={cn(
                              'w-8 h-8 rounded-full flex items-center justify-center text-white text-xs font-bold',
                              isUserRole ? 'bg-blue-500' : 'bg-orange-500'
                            )}
                          >
                            {isUserRole ? '你' : turn.speaker_name?.charAt(0) || 'AI'}
                          </div>
                          {/* Speaker buttons below avatar */}
                          <div className="flex flex-col gap-0.5 items-center">
                            {/* Original audio button (orange) */}
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                playAudio(`original-${turn.id}`, `/api/asset/scripts/turns/${turn.id}/audio`)
                              }}
                              className={cn(
                                'p-1 rounded-full transition-colors',
                                playingAudioId === `original-${turn.id}`
                                  ? 'bg-orange-500 text-white'
                                  : 'bg-orange-100 text-orange-500 hover:bg-orange-200'
                              )}
                              title="播放原声"
                            >
                              <Volume2 className="h-3 w-3" />
                            </button>
                            {/* User recording button (green) - only for user role with recording */}
                            {isUserRole && recording && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation()
                                  playAudio(`user-${turn.id}`, recording.audioBlob)
                                }}
                                className={cn(
                                  'p-1 rounded-full transition-colors',
                                  playingAudioId === `user-${turn.id}`
                                    ? 'bg-green-500 text-white'
                                    : 'bg-green-100 text-green-600 hover:bg-green-200'
                                )}
                                title="播放我的录音"
                              >
                                <Play className="h-3 w-3" />
                              </button>
                            )}
                            {/* Score below buttons */}
                            {isUserRole && recording && recording.score !== undefined && (
                              <span className={cn(
                                'text-xs font-medium',
                                recording.score >= 80 ? 'text-green-600' : recording.score >= 60 ? 'text-amber-600' : 'text-red-500'
                              )}>
                                {recording.score}
                              </span>
                            )}
                          </div>
                        </div>

                        {/* Content */}
                        <div className={cn('flex-1 max-w-[75%]', isUserRole ? 'text-right' : '')}>
                          {/* Speaker name */}
                          <div className="text-xs text-gray-400 mb-1">
                            {turn.speaker_name || (isUserRole ? '你的台词' : '对方')} · 第 {turn.turn_number} 句
                          </div>

                          {/* Message bubble - clickable for user turns */}
                          {/* User role: light blue when unrecorded, deep blue when recorded */}
                          <div
                            onClick={() => isUserRole && !isRecording && setActiveTurnIndex(index)}
                            className={cn(
                              'inline-block rounded-2xl px-4 py-2 text-left',
                              isUserRole
                                ? recording
                                  ? 'bg-blue-500 text-white'  // recorded: deep blue
                                  : 'bg-blue-100 text-blue-800'  // unrecorded: light blue
                                : 'bg-gray-100 text-gray-900',
                              isUserRole && !isRecording && 'cursor-pointer hover:opacity-90',
                              isActive && 'ring-2 ring-orange-400'
                            )}
                          >
                            {(showEn || bothOff) && (
                              <p className={cn('text-sm', bothOff && 'blur-sm select-none')}>
                                {turn.content_en}
                              </p>
                            )}
                            {showEn && showZh && (
                              <div className={cn(
                                'my-1.5 border-t',
                                isUserRole
                                  ? recording ? 'border-blue-400/30' : 'border-blue-200'
                                  : 'border-gray-200'
                              )} />
                            )}
                            {(showZh || bothOff) && (
                              <p className={cn(
                                'text-sm',
                                isUserRole
                                  ? recording ? 'text-blue-100' : 'text-blue-600'  // adjust Chinese text color
                                  : 'text-gray-600',
                                bothOff && 'blur-sm select-none'
                              )}>
                                {turn.content_zh}
                              </p>
                            )}
                          </div>
                        </div>
                      </div>
                    )
                  })}
                </div>

                {/* Bottom controls - Start button or Recording controls */}
                <div className="border-t px-4 py-3 bg-gray-50">
                  {!isStarted ? (
                    /* Start practice button */
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <p className="text-sm text-gray-600">
                          👆 先熟悉上方台词，准备好后点击开始
                          {turns.length > 0 && turns[0].speaker_role !== selectedRole && (
                            <span className="text-orange-600 ml-1">（AI将先开口说第一句）</span>
                          )}
                        </p>
                      </div>
                      <Button
                        onClick={handleStart}
                        className="bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600 text-white px-6"
                      >
                        <Play className="h-4 w-4 mr-2" />
                        开始练习
                      </Button>
                    </div>
                  ) : (
                    /* Recording controls - shown after practice starts */
                    <div className="flex items-center gap-4">
                      {/* Left: Sentence info - full text with wrapping */}
                      <div className="flex-1 min-w-0">
                        {isPlayingAiTurn ? (
                          /* AI is speaking */
                          <div className="space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="text-sm font-medium text-orange-600">🔊 对方正在说话...</span>
                              <span className="text-sm text-gray-500">第 {currentTurnIndex + 1}/{turns.length} 句</span>
                            </div>
                            <p className="text-sm text-gray-500 break-words">
                              {turns[currentTurnIndex]?.content_en}
                            </p>
                          </div>
                        ) : activeTurnIndex !== null ? (
                          <div className="space-y-1">
                            <div className="flex items-center gap-2 flex-wrap">
                              <span className="text-sm font-medium text-gray-700">第 {turns[activeTurnIndex].turn_number} 句</span>
                              <span className="text-xs text-gray-400">({currentTurnIndex + 1}/{turns.length})</span>
                              {/* Recording timer */}
                              {isRecording && (
                                <span className="text-sm font-mono text-red-500">
                                  {Math.floor(recordingDuration / 60)}:{(recordingDuration % 60).toString().padStart(2, '0')}
                                </span>
                              )}
                              {/* Re-record button */}
                              {!isRecording && recordings.has(turns[activeTurnIndex].id) && (
                                <button
                                  onClick={() => {
                                    const turnId = turns[activeTurnIndex].id
                                    setRecordings(prev => {
                                      const next = new Map(prev)
                                      next.delete(turnId)
                                      // Update localStorage
                                      if (selectedScript) {
                                        saveRecordingsToStorage(selectedScript.id, next)
                                      }
                                      return next
                                    })
                                  }}
                                  className="text-xs text-gray-500 hover:text-gray-700 flex items-center gap-1"
                                  title="重新录音"
                                >
                                  <RotateCcw className="h-3 w-3" />
                                  重录
                                </button>
                              )}
                            </div>
                            <p className="text-sm text-gray-500 break-words">
                              {turns[activeTurnIndex].content_en}
                            </p>
                          </div>
                        ) : isEvaluating ? (
                          /* Evaluating pronunciation */
                          <div className="flex items-center gap-2">
                            <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
                            <span className="text-sm text-blue-600 font-medium">正在评估发音...</span>
                          </div>
                        ) : currentTurnIndex >= turns.length ? (
                          /* Dialogue complete */
                          <div className="text-sm text-green-600 font-medium">
                            🎉 对话完成！点击"重新开始"再练一次
                          </div>
                        ) : (
                          <div className="text-sm text-gray-500">
                            等待中...
                          </div>
                        )}
                      </div>

                    {/* Right: Audio controls and recording button */}
                    {activeTurnIndex !== null && !isPlayingAiTurn && !isEvaluating && (
                      <div className="flex items-center gap-3 flex-shrink-0">
                        {/* Play original audio */}
                        <button
                          onClick={() => playAudio(`original-${turns[activeTurnIndex].id}`, `/api/asset/scripts/turns/${turns[activeTurnIndex].id}/audio`)}
                          className={cn(
                            'p-2 rounded-full transition-colors',
                            playingAudioId === `original-${turns[activeTurnIndex].id}`
                              ? 'bg-orange-100 text-orange-600'
                              : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
                          )}
                          title="播放原声"
                        >
                          <Volume2 className="h-5 w-5" />
                        </button>

                        {/* Play user recording if exists */}
                        {recordings.has(turns[activeTurnIndex].id) && (
                          <button
                            onClick={() => {
                              const rec = recordings.get(turns[activeTurnIndex].id)
                              if (rec) playAudio(`user-${turns[activeTurnIndex].id}`, rec.audioBlob)
                            }}
                            className={cn(
                              'p-2 rounded-full transition-colors',
                              playingAudioId === `user-${turns[activeTurnIndex].id}`
                                ? 'bg-green-500 text-white'
                                : 'bg-green-100 text-green-600 hover:bg-green-200'
                            )}
                            title="播放我的录音"
                          >
                            <Play className="h-5 w-5" />
                          </button>
                        )}

                        {/* Recording button with hint */}
                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => isRecording ? stopRecording() : startRecording(activeTurnIndex)}
                            className={cn(
                              'w-12 h-12 rounded-full flex items-center justify-center transition-all shadow-lg',
                              isRecording
                                ? 'bg-red-500 text-white animate-pulse scale-110'
                                : 'bg-blue-500 text-white hover:bg-blue-600 hover:scale-105'
                            )}
                            title={isRecording ? '停止录音' : '开始录音'}
                          >
                            {isRecording ? (
                              <MicOff className="h-5 w-5" />
                            ) : (
                              <Mic className="h-5 w-5" />
                            )}
                          </button>
                          <span className="text-xs text-gray-400 w-16 text-center">
                            {isRecording ? '点击停止' : '按空格录音'}
                          </span>
                        </div>
                      </div>
                    )}
                  </div>
                  )}
                </div>
              </>
            ) : (
              <div className="flex-1 flex items-center justify-center">
                <div className="text-center">
                  <div className="text-6xl mb-4">📝</div>
                  <h3 className="text-lg font-medium text-gray-900 mb-2">
                    选择一个对话开始练习
                  </h3>
                  <p className="text-gray-500">
                    从左侧列表选择对话，开始你的场景练习
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  )
}
