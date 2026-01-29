import { useState, useEffect, useRef, useCallback } from 'react'
import { Mic, Play, Pause, SkipBack, SkipForward, Volume2, CheckCircle, AlertCircle, Loader2 } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'
import {
  listReadSubjects,
  getReadSentences,
  textToSpeech,
  evaluatePronunciation,
  type ReadSubject,
  type ReadSentence,
  type EvaluatePronunciationResponse
} from '../lib/api'
import {
  ensureAudioContextRunning,
  queueAudioFromBase64,
  queueAudio,
  fetchAndQueueAudio,
  onPlayingStateChange,
  stopAudio
} from '../lib/audio'

export function ReadingPage() {
  const { token } = useAuth()
  const [subjects, setSubjects] = useState<ReadSubject[]>([])
  const [sentences, setSentences] = useState<ReadSentence[]>([])
  const [selectedSubject, setSelectedSubject] = useState<ReadSubject | null>(null)
  const [currentIndex, setCurrentIndex] = useState(0)
  const [isRecording, setIsRecording] = useState(false)
  const [playingType, setPlayingType] = useState<'standard' | 'user' | null>(null)
  const [loading, setLoading] = useState(true)
  const [audioLoading, setAudioLoading] = useState(false)
  const [evaluating, setEvaluating] = useState(false)
  const [evaluationResult, setEvaluationResult] = useState<EvaluatePronunciationResponse | null>(null)
  const [recordingDuration, setRecordingDuration] = useState(0)
  const [userAudioData, setUserAudioData] = useState<ArrayBuffer | null>(null)

  // Recording refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const streamRef = useRef<MediaStream | null>(null)

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

  // Fetch subjects from API
  useEffect(() => {
    async function fetchSubjects() {
      try {
        setLoading(true)
        const response = await listReadSubjects({ per_page: 100 })
        setSubjects(response.items)
        if (response.items.length > 0) {
          setSelectedSubject(response.items[0])
        }
      } catch (err) {
        console.error('Failed to fetch subjects:', err)
      } finally {
        setLoading(false)
      }
    }
    fetchSubjects()
  }, [])

  // Fetch sentences for selected subject
  useEffect(() => {
    async function fetchSentences() {
      if (!selectedSubject) return
      try {
        const response = await getReadSentences(selectedSubject.id, { per_page: 200 })
        setSentences(response.items)
        setCurrentIndex(0)
        setEvaluationResult(null)
        setUserAudioData(null)
      } catch (err) {
        console.error('Failed to fetch sentences:', err)
      }
    }
    fetchSentences()
  }, [selectedSubject])

  const currentSentence = sentences[currentIndex]

  // Stop recording and discard results
  const stopRecordingAndDiscard = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      // Remove the onstop handler to prevent evaluation
      mediaRecorderRef.current.onstop = null
      mediaRecorderRef.current.stop()
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop())
      streamRef.current = null
    }
    if (recordingTimerRef.current) {
      clearInterval(recordingTimerRef.current)
      recordingTimerRef.current = null
    }
    setIsRecording(false)
    setRecordingDuration(0)
  }

  const handleNext = () => {
    if (currentIndex < sentences.length - 1) {
      if (isRecording) {
        stopRecordingAndDiscard()
      }
      setCurrentIndex(currentIndex + 1)
      setEvaluationResult(null)
      setUserAudioData(null)
    }
  }

  const handlePrev = () => {
    if (currentIndex > 0) {
      if (isRecording) {
        stopRecordingAndDiscard()
      }
      setCurrentIndex(currentIndex - 1)
      setEvaluationResult(null)
      setUserAudioData(null)
    }
  }

  // Play standard audio for current sentence
  const handlePlayStandard = useCallback(async () => {
    if (!currentSentence || !token || audioLoading) return

    if (playingType === 'standard') {
      stopAudio()
      return
    }

    await ensureAudioContextRunning()
    setAudioLoading(true)

    try {
      const audioUrl = `/api/asset/read/sentences/${currentSentence.id}/audio`
      await fetchAndQueueAudio(audioUrl, 'standard-audio')
    } catch {
      console.log('Pre-recorded audio not available, falling back to TTS')
      try {
        const response = await textToSpeech(token, currentSentence.content_en)
        queueAudioFromBase64(response.audio_base64, 'standard-audio')
      } catch (err) {
        console.error('TTS error:', err)
      }
    } finally {
      setAudioLoading(false)
    }
  }, [currentSentence, token, audioLoading, playingType])

  // Play user's recorded audio
  const handlePlayUserAudio = useCallback(async () => {
    if (!userAudioData) return

    if (playingType === 'user') {
      stopAudio()
      return
    }

    await ensureAudioContextRunning()
    queueAudio(userAudioData, 'user-audio')
  }, [userAudioData, playingType])

  // Convert Blob to base64
  const blobToBase64 = (blob: Blob): Promise<string> => {
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

  // Convert audio blob to WAV format using Web Audio API
  const convertToWav = async (audioBlob: Blob): Promise<Blob> => {
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

  // Start/stop recording
  const handleRecord = async () => {
    if (isRecording) {
      if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
        mediaRecorderRef.current.stop()
      }
      setIsRecording(false)
    } else {
      await ensureAudioContextRunning()

      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
        streamRef.current = stream

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

        mediaRecorder.onstop = async () => {
          stream.getTracks().forEach(track => track.stop())
          streamRef.current = null

          if (recordingTimerRef.current) {
            clearInterval(recordingTimerRef.current)
            recordingTimerRef.current = null
          }
          setRecordingDuration(0)

          const audioBlob = new Blob(audioChunksRef.current, { type: mediaRecorder.mimeType })
          if (audioBlob.size === 0) {
            console.error('No audio recorded')
            return
          }

          if (!token || !currentSentence) return

          setEvaluating(true)
          try {
            const wavBlob = await convertToWav(audioBlob)
            const base64Audio = await blobToBase64(wavBlob)

            // Store WAV data for playback
            const wavArrayBuffer = await wavBlob.arrayBuffer()
            setUserAudioData(wavArrayBuffer)

            const result = await evaluatePronunciation(
              token,
              base64Audio,
              currentSentence.content_en
            )
            setEvaluationResult(result)
          } catch (err) {
            console.error('Evaluation error:', err)
          } finally {
            setEvaluating(false)
          }
        }

        mediaRecorder.start()
        setIsRecording(true)
        setRecordingDuration(0)
        recordingTimerRef.current = setInterval(() => {
          setRecordingDuration(d => d + 1)
        }, 1000)

      } catch (err) {
        console.error('Failed to start recording:', err)
      }
    }
  }

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (recordingTimerRef.current) {
        clearInterval(recordingTimerRef.current)
      }
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop())
      }
    }
  }, [])

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  const getScoreColor = (score: number) => {
    if (score >= 80) return 'text-green-600'
    if (score >= 60) return 'text-amber-600'
    return 'text-red-600'
  }

  const getScoreBg = (score: number) => {
    if (score >= 80) return 'bg-green-500'
    if (score >= 60) return 'bg-amber-500'
    return 'bg-red-500'
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">🎤</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">大声跟读</h1>
            <p className="text-gray-600 mb-6">AI 智能评分，纠正发音，提升口语流利度</p>
            <Button asChild>
              <a href="/login?redirectTo=/read">登录开始练习</a>
            </Button>
          </div>
        </div>
        <Footer />
      </div>
    )
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <main className="mx-auto max-w-6xl p-4">
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-orange-500"></div>
          </div>
        </main>
        <Footer />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-6xl p-4">
        {/* Header with horizontal scrollable tabs */}
        <div className="bg-white rounded-xl shadow-lg mb-4 overflow-hidden">
          {/* Title bar */}
          <div className="flex items-center justify-between px-4 py-2 border-b">
            <h1 className="text-lg font-bold text-gray-900">🎤 跟读练习</h1>
            <span className="text-sm text-gray-500">
              {sentences.length > 0 ? `${currentIndex + 1}/${sentences.length}` : ''}
            </span>
          </div>

          {/* Subject tabs grid */}
          {subjects.length > 0 && (
            <div className="flex flex-wrap gap-1.5 px-3 py-2">
              {subjects.map((subject) => (
                <button
                  key={subject.id}
                  onClick={() => setSelectedSubject(subject)}
                  className={cn(
                    'w-20 py-1.5 rounded-lg text-xs text-center transition-all truncate',
                    selectedSubject?.id === subject.id
                      ? 'bg-orange-500 text-white font-medium shadow-sm'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  )}
                  title={subject.title_zh}
                >
                  {subject.title_zh}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Main content */}
        {currentSentence ? (
          <div className="bg-white rounded-xl shadow-lg p-5 mb-4">
            {/* Sentence */}
            <div className="text-center mb-4">
              <h2 className="text-xl font-semibold text-gray-900 mb-1">
                {currentSentence.content_en}
              </h2>
              <p className="text-gray-500 text-sm">{currentSentence.content_zh}</p>
            </div>

            {/* Audio controls with navigation */}
            <div className="flex items-center justify-center gap-3 mb-4">
              {/* Prev button */}
              <Button
                variant="ghost"
                size="sm"
                onClick={handlePrev}
                disabled={currentIndex === 0}
              >
                <SkipBack className="h-4 w-4 mr-1" />
                上一句
              </Button>

              {/* Standard audio */}
              <Button
                variant="outline"
                size="sm"
                onClick={handlePlayStandard}
                disabled={audioLoading}
                className="gap-2"
              >
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
                className={cn(
                  'px-6',
                  isRecording && 'bg-red-500 hover:bg-red-600'
                )}
              >
                {evaluating ? (
                  <Loader2 className="h-5 w-5 animate-spin" />
                ) : (
                  <Mic className={cn('h-5 w-5', isRecording && 'animate-pulse')} />
                )}
                <span className="ml-2">
                  {evaluating ? '评估中' : isRecording ? formatDuration(recordingDuration) : '录音'}
                </span>
              </Button>

              {/* User audio playback */}
              <Button
                variant="outline"
                size="sm"
                onClick={handlePlayUserAudio}
                disabled={!userAudioData}
                className="gap-2"
              >
                {playingType === 'user' ? (
                  <Pause className="h-4 w-4" />
                ) : (
                  <Play className="h-4 w-4" />
                )}
                回放
              </Button>

              {/* Next button */}
              <Button
                variant="ghost"
                size="sm"
                onClick={handleNext}
                disabled={currentIndex === sentences.length - 1}
              >
                下一句
                <SkipForward className="h-4 w-4 ml-1" />
              </Button>
            </div>

            {/* Evaluation result - compact */}
            {evaluationResult && (
              <div className="border-t pt-4">
                <div className="flex items-center gap-4">
                  {/* Score circle */}
                  <div className={cn(
                    'w-14 h-14 rounded-full border-3 flex items-center justify-center shrink-0',
                    evaluationResult.overall_score >= 80 ? 'border-green-500 bg-green-50' :
                    evaluationResult.overall_score >= 60 ? 'border-amber-500 bg-amber-50' :
                    'border-red-500 bg-red-50'
                  )}>
                    <span className={cn('text-xl font-bold', getScoreColor(evaluationResult.overall_score))}>
                      {evaluationResult.overall_score}
                    </span>
                  </div>

                  {/* Score bars */}
                  <div className="flex-1 space-y-1.5">
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-gray-500 w-16">发音</span>
                      <div className="flex-1 h-1.5 bg-gray-200 rounded-full">
                        <div className={cn('h-1.5 rounded-full', getScoreBg(evaluationResult.pronunciation_score))}
                          style={{ width: `${evaluationResult.pronunciation_score}%` }} />
                      </div>
                      <span className={cn('w-8 text-right font-medium', getScoreColor(evaluationResult.pronunciation_score))}>
                        {evaluationResult.pronunciation_score}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-gray-500 w-16">流畅</span>
                      <div className="flex-1 h-1.5 bg-gray-200 rounded-full">
                        <div className={cn('h-1.5 rounded-full', getScoreBg(evaluationResult.fluency_score))}
                          style={{ width: `${evaluationResult.fluency_score}%` }} />
                      </div>
                      <span className={cn('w-8 text-right font-medium', getScoreColor(evaluationResult.fluency_score))}>
                        {evaluationResult.fluency_score}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-gray-500 w-16">语调</span>
                      <div className="flex-1 h-1.5 bg-gray-200 rounded-full">
                        <div className={cn('h-1.5 rounded-full', getScoreBg(evaluationResult.intonation_score))}
                          style={{ width: `${evaluationResult.intonation_score}%` }} />
                      </div>
                      <span className={cn('w-8 text-right font-medium', getScoreColor(evaluationResult.intonation_score))}>
                        {evaluationResult.intonation_score}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Transcription & feedback */}
                <div className="mt-3 text-xs">
                  {evaluationResult.transcribed_text && (
                    <div className="text-gray-600 bg-gray-50 rounded px-2 py-1.5 mb-2">
                      <span className="text-gray-400">识别: </span>
                      {evaluationResult.transcribed_text}
                    </div>
                  )}
                  {evaluationResult.feedback.length > 0 && (
                    <div className="flex flex-wrap gap-2">
                      {evaluationResult.feedback.map((item, index) => (
                        <span
                          key={index}
                          className={cn(
                            'inline-flex items-center gap-1 px-2 py-0.5 rounded-full',
                            item.type === 'good' ? 'bg-green-100 text-green-700' : 'bg-amber-100 text-amber-700'
                          )}
                        >
                          {item.type === 'good' ? (
                            <CheckCircle className="h-3 w-3" />
                          ) : (
                            <AlertCircle className="h-3 w-3" />
                          )}
                          {item.message}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}

          </div>
        ) : (
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-5xl mb-3">📝</div>
            <h3 className="text-lg font-medium text-gray-900 mb-1">暂无练习内容</h3>
            <p className="text-gray-500 text-sm">请稍后再试或选择其他练习</p>
          </div>
        )}

        {/* Tips - more compact */}
        <div className="bg-white rounded-xl shadow-lg p-4">
          <div className="flex items-center gap-4 text-sm">
            <span className="text-gray-400">💡 技巧:</span>
            <span className="text-gray-600">模仿语调</span>
            <span className="text-gray-300">·</span>
            <span className="text-gray-600">注意连读</span>
            <span className="text-gray-300">·</span>
            <span className="text-gray-600">控制节奏</span>
          </div>
        </div>
      </main>
      <Footer />
    </div>
  )
}
