import { useState } from 'react'
import { Mic, Play, Pause, SkipBack, SkipForward, Volume2, CheckCircle, AlertCircle } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'

interface Sentence {
  id: string
  english: string
  chinese: string
}

const sentences: Sentence[] = [
  { id: '1', english: 'Could you please help me with this?', chinese: '你能帮我一下吗？' },
  { id: '2', english: 'I would like to make a reservation.', chinese: '我想预订一下。' },
  { id: '3', english: 'What time does the meeting start?', chinese: '会议几点开始？' },
  { id: '4', english: 'Thank you for your patience.', chinese: '感谢您的耐心等待。' },
  { id: '5', english: 'Could you repeat that more slowly?', chinese: '你能说慢一点吗？' },
  { id: '6', english: 'I completely agree with you.', chinese: '我完全同意你的看法。' },
]

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
  const [currentIndex, setCurrentIndex] = useState(0)
  const [isRecording, setIsRecording] = useState(false)
  const [isPlaying, setIsPlaying] = useState(false)
  const [hasRecorded, setHasRecorded] = useState(false)

  const currentSentence = sentences[currentIndex]
  const progress = ((currentIndex + 1) / sentences.length) * 100

  const handleNext = () => {
    if (currentIndex < sentences.length - 1) {
      setCurrentIndex(currentIndex + 1)
      setHasRecorded(false)
    }
  }

  const handlePrev = () => {
    if (currentIndex > 0) {
      setCurrentIndex(currentIndex - 1)
      setHasRecorded(false)
    }
  }

  const handleRecord = () => {
    setIsRecording(!isRecording)
    if (isRecording) {
      setHasRecorded(true)
    }
  }

  const handlePlay = () => {
    setIsPlaying(!isPlaying)
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
              <a href="/login?redirectTo=/reading">登录开始练习</a>
            </Button>
          </div>
        </div>
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
              {currentIndex + 1}/{sentences.length} 句
            </span>
          </div>
        </div>

        {/* Sentence Display */}
        <div className="bg-white rounded-xl shadow-lg p-6 mb-4 text-center">
          <div className="text-xs text-gray-400 mb-2">今日练习</div>
          <h2 className="text-2xl font-semibold text-gray-900 mb-2">
            {currentSentence.english}
          </h2>
          <p className="text-gray-500">{currentSentence.chinese}</p>
        </div>

        {/* Waveforms */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
          {/* Native Audio */}
          <div className="bg-white rounded-xl shadow-lg p-4">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-gray-700">
                <Volume2 className="h-4 w-4 inline mr-2" />
                标准发音
              </span>
              <Button size="sm" variant="ghost" onClick={handlePlay}>
                {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
              </Button>
            </div>
            <div className="h-20 bg-gray-100 rounded-lg flex items-center justify-center">
              <div className="flex items-end gap-1 h-12">
                {Array.from({ length: 30 }).map((_, i) => (
                  <div
                    key={i}
                    className="w-1 bg-green-400 rounded-full"
                    style={{ height: `${Math.random() * 100}%` }}
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
              </span>
              {hasRecorded && (
                <Button size="sm" variant="ghost">
                  <Play className="h-4 w-4" />
                </Button>
              )}
            </div>
            <div className="h-20 bg-gray-100 rounded-lg flex items-center justify-center">
              {hasRecorded ? (
                <div className="flex items-end gap-1 h-12">
                  {Array.from({ length: 30 }).map((_, i) => (
                    <div
                      key={i}
                      className="w-1 bg-orange-400 rounded-full"
                      style={{ height: `${Math.random() * 100}%` }}
                    />
                  ))}
                </div>
              ) : (
                <span className="text-sm text-gray-400">点击录音开始</span>
              )}
            </div>
          </div>
        </div>

        {/* Score Card (shown after recording) */}
        {hasRecorded && (
          <div className="bg-white rounded-xl shadow-lg p-6 mb-4">
            <h3 className="font-semibold text-gray-900 mb-4">
              <span className="mr-2">🧠</span>
              AI 评分与建议
            </h3>
            <div className="flex items-start gap-6">
              {/* Overall Score */}
              <div className="text-center">
                <div className="w-20 h-20 rounded-full border-4 border-green-500 flex items-center justify-center bg-green-50">
                  <span className="text-3xl font-bold text-green-600">85</span>
                </div>
                <span className="text-sm text-gray-500 mt-2 block">总分</span>
              </div>

              {/* Detailed Scores */}
              <div className="flex-1 space-y-3">
                <ScoreBar label="发音准确度" score={90} color="bg-green-500" />
                <ScoreBar label="流畅度" score={80} color="bg-amber-500" />
                <ScoreBar label="语调" score={85} color="bg-green-500" />
              </div>
            </div>

            {/* Feedback */}
            <div className="mt-4 pt-4 border-t space-y-2">
              <div className="flex items-start gap-2 text-sm">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                <span className="text-gray-700">
                  需要注意: "help" 的发音稍重，注意轻读
                </span>
              </div>
              <div className="flex items-start gap-2 text-sm">
                <CheckCircle className="h-4 w-4 text-green-500 shrink-0 mt-0.5" />
                <span className="text-green-700">
                  做得好: "Could you" 的连读非常自然！
                </span>
              </div>
            </div>
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
            className={cn(
              'px-8',
              isRecording && 'bg-red-500 hover:bg-red-600'
            )}
          >
            <Mic className={cn('h-5 w-5 mr-2', isRecording && 'animate-pulse')} />
            {isRecording ? '停止录音' : hasRecorded ? '重新录音' : '开始录音'}
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
