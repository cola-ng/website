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
  onPlayingStateChange,
  stopAudio
} from '../lib/audio'

interface ScoreDetail {
  label: string
  score: number
  color: string
}

function ScoreBar({ label, score, color }: ScoreDetail) {
  return (
    <div className="flex items-center gap-3">
      <span className="text-sm text-gray-600 w-20">{label}</span>
      <div className="flex-1 h-2 bg-gray-200 rounded-full">
        <div
          className={cn('h-2 rounded-full transition-all', color)}
          style={{ width: `${score}%` }}
        />
      </div>
      <span className={cn('text-sm font-semibold w-12 text-right',
        score >= 80 ? 'text-green-600' : score >= 60 ? 'text-amber-600' : 'text-red-600'
      )}>
        {score}%
      </span>
    </div>
  )
}

export function ReadingPage() {
  const { token } = useAuth()
  const [subjects, setSubjects] = useState<ReadSubject[]>([])
  const [sentences, setSentences] = useState<ReadSentence[]>([])
  const [selectedSubject, setSelectedSubject] = useState<ReadSubject | null>(null)
  const [currentIndex, setCurrentIndex] = useState(0)
  const [isRecording, setIsRecording] = useState(false)
  const [isPlaying, setIsPlaying] = useState(false)
  const [loading, setLoading] = useState(true)
  const [ttsLoading, setTtsLoading] = useState(false)
  const [evaluating, setEvaluating] = useState(false)
  const [evaluationResult, setEvaluationResult] = useState<EvaluatePronunciationResponse | null>(null)
  const [recordingDuration, setRecordingDuration] = useState(0)

  // Recording refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const streamRef = useRef<MediaStream | null>(null)

  // Subscribe to global audio playing state
  useEffect(() => {
    return onPlayingStateChange((id) => {
      setIsPlaying(id === 'tts-playback')
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
      } catch (err) {
        console.error('Failed to fetch sentences:', err)
      }
    }
    fetchSentences()
  }, [selectedSubject])

  const currentSentence = sentences[currentIndex]
  const progress = sentences.length > 0 ? ((currentIndex + 1) / sentences.length) * 100 : 0

  const handleNext = () => {
    if (currentIndex < sentences.length - 1) {
      setCurrentIndex(currentIndex + 1)
      setEvaluationResult(null)
    }
  }

  const handlePrev = () => {
    if (currentIndex > 0) {
      setCurrentIndex(currentIndex - 1)
      setEvaluationResult(null)
    }
  }

  // Play TTS for current sentence
  const handlePlayTts = useCallback(async () => {
    if (!currentSentence || !token || ttsLoading) return

    // If already playing, stop
    if (isPlaying) {
      stopAudio()
      return
    }

    // Activate AudioContext on user interaction
    await ensureAudioContextRunning()

    setTtsLoading(true)
    try {
      const response = await textToSpeech(token, currentSentence.content_en)
      queueAudioFromBase64(response.audio_base64, 'tts-playback')
    } catch (err) {
      console.error('TTS error:', err)
    } finally {
      setTtsLoading(false)
    }
  }, [currentSentence, token, ttsLoading, isPlaying])

  // Convert Blob to base64
  const blobToBase64 = (blob: Blob): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onloadend = () => {
        const base64 = reader.result as string
        // Remove data URL prefix (e.g., "data:audio/webm;base64,")
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

      // Create WAV file
      const numberOfChannels = audioBuffer.numberOfChannels
      const sampleRate = audioBuffer.sampleRate
      const length = audioBuffer.length

      // Create buffer for WAV file
      const wavBuffer = new ArrayBuffer(44 + length * numberOfChannels * 2)
      const view = new DataView(wavBuffer)

      // Write WAV header
      // "RIFF"
      view.setUint8(0, 0x52); view.setUint8(1, 0x49); view.setUint8(2, 0x46); view.setUint8(3, 0x46)
      // File size - 8
      view.setUint32(4, 36 + length * numberOfChannels * 2, true)
      // "WAVE"
      view.setUint8(8, 0x57); view.setUint8(9, 0x41); view.setUint8(10, 0x56); view.setUint8(11, 0x45)
      // "fmt "
      view.setUint8(12, 0x66); view.setUint8(13, 0x6d); view.setUint8(14, 0x74); view.setUint8(15, 0x20)
      // Subchunk1Size (16 for PCM)
      view.setUint32(16, 16, true)
      // AudioFormat (1 for PCM)
      view.setUint16(20, 1, true)
      // NumChannels
      view.setUint16(22, numberOfChannels, true)
      // SampleRate
      view.setUint32(24, sampleRate, true)
      // ByteRate
      view.setUint32(28, sampleRate * numberOfChannels * 2, true)
      // BlockAlign
      view.setUint16(32, numberOfChannels * 2, true)
      // BitsPerSample
      view.setUint16(34, 16, true)
      // "data"
      view.setUint8(36, 0x64); view.setUint8(37, 0x61); view.setUint8(38, 0x74); view.setUint8(39, 0x61)
      // Subchunk2Size
      view.setUint32(40, length * numberOfChannels * 2, true)

      // Write audio data
      let offset = 44
      for (let i = 0; i < length; i++) {
        for (let channel = 0; channel < numberOfChannels; channel++) {
          const sample = audioBuffer.getChannelData(channel)[i]
          // Convert to 16-bit PCM
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
      // Stop recording
      if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
        mediaRecorderRef.current.stop()
      }
      setIsRecording(false)
    } else {
      // Start recording
      // Activate AudioContext on user interaction
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
          // Stop all tracks
          stream.getTracks().forEach(track => track.stop())
          streamRef.current = null

          // Clear recording timer
          if (recordingTimerRef.current) {
            clearInterval(recordingTimerRef.current)
            recordingTimerRef.current = null
          }
          setRecordingDuration(0)

          // Process audio
          const audioBlob = new Blob(audioChunksRef.current, { type: mediaRecorder.mimeType })
          if (audioBlob.size === 0) {
            console.error('No audio recorded')
            return
          }

          // Evaluate pronunciation
          if (!token || !currentSentence) return

          setEvaluating(true)
          try {
            // Convert to WAV for better ASR compatibility
            const wavBlob = await convertToWav(audioBlob)
            const base64Audio = await blobToBase64(wavBlob)

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

        // Start recording
        mediaRecorder.start()
        setIsRecording(true)

        // Start recording timer
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

  // Format recording duration
  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">🎤</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">大声跟读</h1>
            <p className="text-gray-600 mb-6">
              AI 智能评分，纠正发音，提升口语流利度
            </p>
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
        {/* Header */}
        <div className="bg-white rounded-xl shadow-lg p-6 mb-4">
          <div className="flex items-baseline gap-3 mb-4">
            <h1 className="text-2xl font-bold text-gray-900">
              <span className="mr-2">🎤</span>
              跟读练习
            </h1>
            <p className="text-gray-500">发音纠正 · 音波对比 · AI 智能评分</p>
          </div>

          {/* Subject selector */}
          {subjects.length > 0 && (
            <div className="flex gap-2 mb-4 flex-wrap">
              {subjects.map((subject) => (
                <button
                  key={subject.id}
                  onClick={() => setSelectedSubject(subject)}
                  className={cn(
                    'px-3 py-1.5 rounded-lg text-sm font-medium transition-colors',
                    selectedSubject?.id === subject.id
                      ? 'bg-orange-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-orange-50'
                  )}
                >
                  {subject.title_zh}
                </button>
              ))}
            </div>
          )}

          {/* Progress */}
          <div className="flex items-center gap-3">
            <span className="text-sm text-gray-500">练习进度</span>
            <div className="flex-1 h-2 bg-gray-200 rounded-full">
              <div
                className="h-2 bg-green-500 rounded-full transition-all"
                style={{ width: `${progress}%` }}
              />
            </div>
            <span className="text-sm font-semibold text-gray-700">
              {sentences.length > 0 ? `${currentIndex + 1}/${sentences.length} 句` : '0/0 句'}
            </span>
          </div>
        </div>

        {/* Sentence Display */}
        {currentSentence ? (
          <>
            <div className="bg-white rounded-xl shadow-lg p-6 mb-4 text-center">
              <div className="text-xs text-gray-400 mb-2">
                {selectedSubject?.title_zh || '今日练习'}
              </div>
              <h2 className="text-2xl font-semibold text-gray-900 mb-2">
                {currentSentence.content_en}
              </h2>
              <p className="text-gray-500">{currentSentence.content_zh}</p>
              {currentSentence.phonetic_transcription && (
                <p className="text-sm text-orange-600 mt-2">🔊 {currentSentence.phonetic_transcription}</p>
              )}
            </div>

            {/* Audio Controls */}
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
              {/* Native Audio */}
              <div className="bg-white rounded-xl shadow-lg p-4">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-sm font-medium text-gray-700">
                    <Volume2 className="h-4 w-4 inline mr-2" />
                    标准发音
                  </span>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={handlePlayTts}
                    disabled={ttsLoading}
                  >
                    {ttsLoading ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : isPlaying ? (
                      <Pause className="h-4 w-4" />
                    ) : (
                      <Play className="h-4 w-4" />
                    )}
                  </Button>
                </div>
                <div className="h-20 bg-gray-100 rounded-lg flex items-center justify-center">
                  <div className="flex items-end gap-1 h-12">
                    {Array.from({ length: 30 }).map((_, i) => (
                      <div
                        key={i}
                        className={cn(
                          'w-1 rounded-full transition-all',
                          isPlaying ? 'bg-green-500' : 'bg-green-300'
                        )}
                        style={{
                          height: `${isPlaying ? 20 + Math.sin(i * 0.5 + Date.now() / 200) * 40 + Math.random() * 20 : 20 + Math.random() * 30}%`,
                          transition: isPlaying ? 'height 0.1s' : 'none'
                        }}
                      />
                    ))}
                  </div>
                </div>
              </div>

              {/* User Audio */}
              <div className="bg-white rounded-xl shadow-lg p-4">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-sm font-medium text-gray-700">
                    <Mic className="h-4 w-4 inline mr-2" />
                    你的发音
                    {isRecording && (
                      <span className="ml-2 text-red-500 text-xs">
                        {formatDuration(recordingDuration)}
                      </span>
                    )}
                  </span>
                </div>
                <div className="h-20 bg-gray-100 rounded-lg flex items-center justify-center">
                  {evaluating ? (
                    <div className="flex items-center gap-2 text-gray-500">
                      <Loader2 className="h-5 w-5 animate-spin" />
                      <span className="text-sm">正在评估...</span>
                    </div>
                  ) : isRecording ? (
                    <div className="flex items-end gap-1 h-12">
                      {Array.from({ length: 30 }).map((_, i) => (
                        <div
                          key={i}
                          className="w-1 bg-red-400 rounded-full animate-pulse"
                          style={{
                            height: `${20 + Math.random() * 60}%`,
                            animationDelay: `${i * 50}ms`
                          }}
                        />
                      ))}
                    </div>
                  ) : evaluationResult ? (
                    <div className="flex items-end gap-1 h-12">
                      {Array.from({ length: 30 }).map((_, i) => (
                        <div
                          key={i}
                          className="w-1 bg-orange-400 rounded-full"
                          style={{ height: `${20 + Math.random() * 60}%` }}
                        />
                      ))}
                    </div>
                  ) : (
                    <span className="text-sm text-gray-400">点击录音开始</span>
                  )}
                </div>
              </div>
            </div>

            {/* Score Card (shown after evaluation) */}
            {evaluationResult && (
              <div className="bg-white rounded-xl shadow-lg p-6 mb-4">
                <h3 className="font-semibold text-gray-900 mb-4">
                  <span className="mr-2">🧠</span>
                  AI 评分与建议
                </h3>
                <div className="flex items-start gap-6">
                  {/* Overall Score */}
                  <div className="text-center">
                    <div className={cn(
                      'w-20 h-20 rounded-full border-4 flex items-center justify-center',
                      evaluationResult.overall_score >= 80
                        ? 'border-green-500 bg-green-50'
                        : evaluationResult.overall_score >= 60
                          ? 'border-amber-500 bg-amber-50'
                          : 'border-red-500 bg-red-50'
                    )}>
                      <span className={cn(
                        'text-3xl font-bold',
                        evaluationResult.overall_score >= 80
                          ? 'text-green-600'
                          : evaluationResult.overall_score >= 60
                            ? 'text-amber-600'
                            : 'text-red-600'
                      )}>
                        {evaluationResult.overall_score}
                      </span>
                    </div>
                    <span className="text-sm text-gray-500 mt-2 block">总分</span>
                  </div>

                  {/* Detailed Scores */}
                  <div className="flex-1 space-y-3">
                    <ScoreBar
                      label="发音准确度"
                      score={evaluationResult.pronunciation_score}
                      color={evaluationResult.pronunciation_score >= 80 ? 'bg-green-500' : evaluationResult.pronunciation_score >= 60 ? 'bg-amber-500' : 'bg-red-500'}
                    />
                    <ScoreBar
                      label="流畅度"
                      score={evaluationResult.fluency_score}
                      color={evaluationResult.fluency_score >= 80 ? 'bg-green-500' : evaluationResult.fluency_score >= 60 ? 'bg-amber-500' : 'bg-red-500'}
                    />
                    <ScoreBar
                      label="语调"
                      score={evaluationResult.intonation_score}
                      color={evaluationResult.intonation_score >= 80 ? 'bg-green-500' : evaluationResult.intonation_score >= 60 ? 'bg-amber-500' : 'bg-red-500'}
                    />
                  </div>
                </div>

                {/* Transcribed Text */}
                {evaluationResult.transcribed_text && (
                  <div className="mt-4 pt-4 border-t">
                    <div className="text-sm text-gray-500 mb-1">识别结果:</div>
                    <div className="text-gray-700 bg-gray-50 rounded-lg p-3">
                      {evaluationResult.transcribed_text}
                    </div>
                  </div>
                )}

                {/* Feedback */}
                {evaluationResult.feedback.length > 0 && (
                  <div className="mt-4 pt-4 border-t space-y-2">
                    {evaluationResult.feedback.map((item, index) => (
                      <div key={index} className="flex items-start gap-2 text-sm">
                        {item.type === 'good' ? (
                          <CheckCircle className="h-4 w-4 text-green-500 shrink-0 mt-0.5" />
                        ) : (
                          <AlertCircle className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                        )}
                        <span className={item.type === 'good' ? 'text-green-700' : 'text-gray-700'}>
                          {item.message}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex items-center justify-center gap-3">
              <Button
                variant="outline"
                onClick={handlePrev}
                disabled={currentIndex === 0}
              >
                <SkipBack className="h-4 w-4 mr-2" />
                上一句
              </Button>

              <Button
                size="lg"
                onClick={handleRecord}
                disabled={evaluating}
                className={cn(
                  'px-8',
                  isRecording && 'bg-red-500 hover:bg-red-600'
                )}
              >
                {evaluating ? (
                  <Loader2 className="h-5 w-5 mr-2 animate-spin" />
                ) : (
                  <Mic className={cn('h-5 w-5 mr-2', isRecording && 'animate-pulse')} />
                )}
                {evaluating ? '评估中...' : isRecording ? '停止录音' : evaluationResult ? '重新录音' : '开始录音'}
              </Button>

              <Button
                variant="outline"
                onClick={handleNext}
                disabled={currentIndex === sentences.length - 1}
              >
                下一句
                <SkipForward className="h-4 w-4 ml-2" />
              </Button>
            </div>
          </>
        ) : (
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">📝</div>
            <h3 className="text-lg font-medium text-gray-900 mb-2">
              暂无练习内容
            </h3>
            <p className="text-gray-500">
              请稍后再试或选择其他练习
            </p>
          </div>
        )}

        {/* Tips */}
        <div className="mt-6 bg-white rounded-xl shadow-lg p-6">
          <h3 className="font-semibold text-gray-900 mb-3">
            <span className="mr-2">💡</span>
            跟读技巧
          </h3>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="font-medium text-gray-800 mb-1">🎯 模仿语调</div>
              <p className="text-sm text-gray-600">注意句子的升降调</p>
            </div>
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="font-medium text-gray-800 mb-1">🔗 注意连读</div>
              <p className="text-sm text-gray-600">单词之间的自然衔接</p>
            </div>
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="font-medium text-gray-800 mb-1">⏱️ 控制节奏</div>
              <p className="text-sm text-gray-600">不要太快或太慢</p>
            </div>
          </div>
        </div>
      </main>
      <Footer />
    </div>
  )
}
