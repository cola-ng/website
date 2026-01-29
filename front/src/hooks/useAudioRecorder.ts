import { useState, useRef, useCallback } from 'react'
import { ensureAudioContextRunning } from '../lib/audio'

export interface AudioRecorderState {
  isRecording: boolean
  recordingDuration: number
  userAudioData: ArrayBuffer | null
}

export interface AudioRecorderActions {
  startRecording: () => Promise<void>
  stopRecording: () => void
  stopAndDiscard: () => void
  clearAudio: () => void
}

export interface UseAudioRecorderOptions {
  onRecordingComplete?: (wavBlob: Blob, wavArrayBuffer: ArrayBuffer, base64: string) => void
  onError?: (error: Error) => void
}

/** Convert Blob to base64 string */
export async function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onloadend = () => {
      const base64 = reader.result as string
      const base64Data = base64.split(',')[1]
      resolve(base64Data)
    }
    reader.onerror = reject
    reader.readAsDataURL(blob)
  })
}

/** Convert audio blob to WAV format using Web Audio API */
export async function convertToWav(audioBlob: Blob): Promise<Blob> {
  const audioContext = new AudioContext()
  const arrayBuffer = await audioBlob.arrayBuffer()

  try {
    const audioBuffer = await audioContext.decodeAudioData(arrayBuffer)
    const numberOfChannels = audioBuffer.numberOfChannels
    const sampleRate = audioBuffer.sampleRate
    const length = audioBuffer.length

    const wavBuffer = new ArrayBuffer(44 + length * numberOfChannels * 2)
    const view = new DataView(wavBuffer)

    // WAV header - "RIFF"
    view.setUint8(0, 0x52)
    view.setUint8(1, 0x49)
    view.setUint8(2, 0x46)
    view.setUint8(3, 0x46)
    view.setUint32(4, 36 + length * numberOfChannels * 2, true)
    // "WAVE"
    view.setUint8(8, 0x57)
    view.setUint8(9, 0x41)
    view.setUint8(10, 0x56)
    view.setUint8(11, 0x45)
    // "fmt "
    view.setUint8(12, 0x66)
    view.setUint8(13, 0x6d)
    view.setUint8(14, 0x74)
    view.setUint8(15, 0x20)
    view.setUint32(16, 16, true) // Subchunk1Size
    view.setUint16(20, 1, true) // AudioFormat (PCM)
    view.setUint16(22, numberOfChannels, true)
    view.setUint32(24, sampleRate, true)
    view.setUint32(28, sampleRate * numberOfChannels * 2, true) // ByteRate
    view.setUint16(32, numberOfChannels * 2, true) // BlockAlign
    view.setUint16(34, 16, true) // BitsPerSample
    // "data"
    view.setUint8(36, 0x64)
    view.setUint8(37, 0x61)
    view.setUint8(38, 0x74)
    view.setUint8(39, 0x61)
    view.setUint32(40, length * numberOfChannels * 2, true)

    // Write audio samples
    let offset = 44
    for (let i = 0; i < length; i++) {
      for (let channel = 0; channel < numberOfChannels; channel++) {
        const sample = audioBuffer.getChannelData(channel)[i]
        const s = Math.max(-1, Math.min(1, sample))
        view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7fff, true)
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

/** Format seconds to MM:SS */
export function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

/**
 * Hook for audio recording with MediaRecorder API
 * Handles recording, WAV conversion, and base64 encoding
 */
export function useAudioRecorder(options: UseAudioRecorderOptions = {}): AudioRecorderState & AudioRecorderActions {
  const { onRecordingComplete, onError } = options

  const [isRecording, setIsRecording] = useState(false)
  const [recordingDuration, setRecordingDuration] = useState(0)
  const [userAudioData, setUserAudioData] = useState<ArrayBuffer | null>(null)

  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const streamRef = useRef<MediaStream | null>(null)

  const cleanup = useCallback(() => {
    if (recordingTimerRef.current) {
      clearInterval(recordingTimerRef.current)
      recordingTimerRef.current = null
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop())
      streamRef.current = null
    }
    setRecordingDuration(0)
  }, [])

  const stopAndDiscard = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.onstop = null
      mediaRecorderRef.current.stop()
    }
    cleanup()
    setIsRecording(false)
  }, [cleanup])

  const stopRecording = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop()
    }
    setIsRecording(false)
  }, [])

  const startRecording = useCallback(async () => {
    await ensureAudioContextRunning()

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      streamRef.current = stream

      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/mp4',
      })

      mediaRecorderRef.current = mediaRecorder
      audioChunksRef.current = []

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data)
        }
      }

      mediaRecorder.onstop = async () => {
        cleanup()

        const audioBlob = new Blob(audioChunksRef.current, { type: mediaRecorder.mimeType })
        if (audioBlob.size === 0) {
          console.error('No audio recorded')
          return
        }

        try {
          const wavBlob = await convertToWav(audioBlob)
          const wavArrayBuffer = await wavBlob.arrayBuffer()
          const base64Audio = await blobToBase64(wavBlob)

          setUserAudioData(wavArrayBuffer)
          onRecordingComplete?.(wavBlob, wavArrayBuffer, base64Audio)
        } catch (err) {
          console.error('Error processing recording:', err)
          onError?.(err instanceof Error ? err : new Error('Recording processing failed'))
        }
      }

      mediaRecorder.start()
      setIsRecording(true)
      setRecordingDuration(0)
      recordingTimerRef.current = setInterval(() => {
        setRecordingDuration((d) => d + 1)
      }, 1000)
    } catch (err) {
      console.error('Failed to start recording:', err)
      onError?.(err instanceof Error ? err : new Error('Failed to access microphone'))
    }
  }, [cleanup, onRecordingComplete, onError])

  const clearAudio = useCallback(() => {
    setUserAudioData(null)
  }, [])

  return {
    isRecording,
    recordingDuration,
    userAudioData,
    startRecording,
    stopRecording,
    stopAndDiscard,
    clearAudio,
  }
}
