import { useState, useCallback } from 'react'
import { Mic, Volume2, Play, Pause, Loader2, SkipForward } from 'lucide-react'
import { Button } from '../ui/button'
import { cn } from '../../lib/utils'
import { useAudioRecorder, formatDuration } from '../../hooks/useAudioRecorder'
import { PronunciationResult } from './ScoreDisplay'
import { evaluatePronunciation, textToSpeech, type EvaluatePronunciationResponse } from '../../lib/api'
import {
  ensureAudioContextRunning,
  queueAudioFromBase64,
  queueAudio,
  fetchAndQueueAudio,
  onPlayingStateChange,
  stopAudio,
} from '../../lib/audio'
import type { ReviewItem, ReviewResult } from '../../types/review'
import { useEffect } from 'react'

interface PronunciationPracticeProps {
  item: ReviewItem
  token: string
  onComplete: (result: ReviewResult) => void
  onSkip?: () => void
}

export function PronunciationPractice({ item, token, onComplete, onSkip }: PronunciationPracticeProps) {
  const [playingType, setPlayingType] = useState<'standard' | 'user' | null>(null)
  const [audioLoading, setAudioLoading] = useState(false)
  const [evaluating, setEvaluating] = useState(false)
  const [evaluationResult, setEvaluationResult] = useState<EvaluatePronunciationResponse | null>(null)
  const [startTime] = useState(() => Date.now())

  // Subscribe to global audio playing state
  useEffect(() => {
    return onPlayingStateChange((id) => {
      if (id === 'standard-audio') {
        setPlayingType('standard')
      } else if (id === 'user-audio') {
        setPlayingType('user')
      } else {
        setPlayingType(null)
      }
    })
  }, [])

  const handleRecordingComplete = useCallback(
    async (_wavBlob: Blob, wavArrayBuffer: ArrayBuffer, base64Audio: string) => {
      setEvaluating(true)
      try {
        const result = await evaluatePronunciation(token, base64Audio, item.word)
        setEvaluationResult(result)

        // Store WAV for playback
        recorder.userAudioData // Already set by the hook
      } catch (err) {
        console.error('Evaluation error:', err)
      } finally {
        setEvaluating(false)
      }
    },
    [token, item.word]
  )

  const recorder = useAudioRecorder({
    onRecordingComplete: handleRecordingComplete,
    onError: (err) => console.error('Recording error:', err),
  })

  // Play standard audio (TTS)
  const handlePlayStandard = useCallback(async () => {
    if (audioLoading) return

    if (playingType === 'standard') {
      stopAudio()
      return
    }

    await ensureAudioContextRunning()
    setAudioLoading(true)

    try {
      // Try to use pre-recorded audio if available
      if (item.audio_url) {
        await fetchAndQueueAudio(item.audio_url, 'standard-audio')
      } else {
        // Fall back to TTS
        const response = await textToSpeech(token, item.word)
        queueAudioFromBase64(response.audio_base64, 'standard-audio')
      }
    } catch (err) {
      console.error('Audio playback error:', err)
    } finally {
      setAudioLoading(false)
    }
  }, [audioLoading, playingType, token, item.word, item.audio_url])

  // Play user's recorded audio
  const handlePlayUserAudio = useCallback(async () => {
    if (!recorder.userAudioData) return

    if (playingType === 'user') {
      stopAudio()
      return
    }

    await ensureAudioContextRunning()
    queueAudio(recorder.userAudioData, 'user-audio')
  }, [recorder.userAudioData, playingType])

  // Handle recording toggle
  const handleRecord = () => {
    if (recorder.isRecording) {
      recorder.stopRecording()
    } else {
      setEvaluationResult(null)
      recorder.startRecording()
    }
  }

  // Complete and move to next
  const handleNext = () => {
    const result: ReviewResult = {
      item_id: item.id,
      result: evaluationResult
        ? evaluationResult.overall_score >= 80
          ? 'correct'
          : evaluationResult.overall_score >= 60
            ? 'partial'
            : 'incorrect'
        : 'incorrect',
      score: evaluationResult?.overall_score,
      time_spent_ms: Date.now() - startTime,
      attempts: 1,
    }
    onComplete(result)
  }

  return (
    <div className="flex flex-col h-full">
      {/* Word display */}
      <div className="text-center mb-6">
        <h2 className="text-3xl font-bold text-gray-900 mb-2">{item.word}</h2>
        {item.phonetic && <p className="text-gray-500 text-lg mb-1">{item.phonetic}</p>}
        {item.word_zh && <p className="text-gray-600">{item.word_zh}</p>}
      </div>

      {/* Audio controls */}
      <div className="flex items-center justify-center gap-4 mb-6">
        {/* Standard audio */}
        <Button variant="outline" size="sm" onClick={handlePlayStandard} disabled={audioLoading} className="gap-2">
          {audioLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : playingType === 'standard' ? (
            <Pause className="h-4 w-4" />
          ) : (
            <Volume2 className="h-4 w-4" />
          )}
          原声
        </Button>

        {/* Record button */}
        <Button
          size="lg"
          onClick={handleRecord}
          disabled={evaluating}
          className={cn('px-8', recorder.isRecording && 'bg-red-500 hover:bg-red-600')}
        >
          {evaluating ? (
            <Loader2 className="h-5 w-5 animate-spin" />
          ) : (
            <Mic className={cn('h-5 w-5', recorder.isRecording && 'animate-pulse')} />
          )}
          <span className="ml-2">
            {evaluating ? '评估中' : recorder.isRecording ? formatDuration(recorder.recordingDuration) : '录音'}
          </span>
        </Button>

        {/* User audio playback */}
        <Button
          variant="outline"
          size="sm"
          onClick={handlePlayUserAudio}
          disabled={!recorder.userAudioData}
          className="gap-2"
        >
          {playingType === 'user' ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
          回放
        </Button>
      </div>

      {/* Evaluation result */}
      {evaluationResult && (
        <PronunciationResult
          overallScore={evaluationResult.overall_score}
          pronunciationScore={evaluationResult.pronunciation_score}
          fluencyScore={evaluationResult.fluency_score}
          intonationScore={evaluationResult.intonation_score}
          transcribedText={evaluationResult.transcribed_text}
          feedback={evaluationResult.feedback}
          className="mb-6"
        />
      )}

      {/* Action buttons */}
      <div className="flex justify-center gap-4 mt-auto">
        {onSkip && (
          <Button variant="ghost" onClick={onSkip}>
            跳过
          </Button>
        )}
        <Button onClick={handleNext} disabled={!evaluationResult} className="gap-2">
          下一个
          <SkipForward className="h-4 w-4" />
        </Button>
      </div>
    </div>
  )
}
