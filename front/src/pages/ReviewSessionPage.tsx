import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  X,
  Volume2,
  RotateCcw,
  ThumbsUp,
  ThumbsDown,
  Zap,
  Clock,
  Trophy,
  Flame,
  ArrowLeft,
} from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'

interface ReviewWord {
  id: string
  word: string
  phonetic: string
  meaning: string
  example: string
  exampleZh: string
  mastery: number
  reviewCount: number
}

// Mock data for review session
const reviewWords: ReviewWord[] = [
  {
    id: '1',
    word: 'accommodation',
    phonetic: '/əˌkɒməˈdeɪʃn/',
    meaning: '住所，住宿；适应，调节',
    example: 'We need to find accommodation for the night.',
    exampleZh: '我们需要找个地方过夜。',
    mastery: 60,
    reviewCount: 3,
  },
  {
    id: '2',
    word: 'itinerary',
    phonetic: '/aɪˈtɪnərəri/',
    meaning: '行程表，旅行路线',
    example: "What's our itinerary for tomorrow?",
    exampleZh: '我们明天的行程是什么？',
    mastery: 45,
    reviewCount: 5,
  },
  {
    id: '3',
    word: 'reservation',
    phonetic: '/ˌrezəˈveɪʃn/',
    meaning: '预订；保留意见',
    example: 'I have a reservation under the name Smith.',
    exampleZh: '我有一个史密斯名下的预订。',
    mastery: 70,
    reviewCount: 2,
  },
  {
    id: '4',
    word: 'procrastinate',
    phonetic: '/prəˈkræstɪneɪt/',
    meaning: '拖延，耽搁',
    example: "Stop procrastinating and start working!",
    exampleZh: '别再拖延了，开始工作吧！',
    mastery: 30,
    reviewCount: 8,
  },
  {
    id: '5',
    word: 'meticulous',
    phonetic: '/məˈtɪkjələs/',
    meaning: '一丝不苟的，细致的',
    example: 'She is meticulous about her work.',
    exampleZh: '她对工作一丝不苟。',
    mastery: 55,
    reviewCount: 4,
  },
]

type ReviewResult = 'forgot' | 'hard' | 'good' | 'easy'

interface ReviewRecord {
  wordId: string
  result: ReviewResult
  timeSpent: number
}

export function ReviewSessionPage() {
  const { token } = useAuth()
  const navigate = useNavigate()

  const [currentIndex, setCurrentIndex] = useState(0)
  const [isFlipped, setIsFlipped] = useState(false)
  const [showAnswer, setShowAnswer] = useState(false)
  const [reviewRecords, setReviewRecords] = useState<ReviewRecord[]>([])
  const [sessionComplete, setSessionComplete] = useState(false)
  const [startTime, setStartTime] = useState<number>(Date.now())
  const [cardStartTime, setCardStartTime] = useState<number>(Date.now())

  const currentWord = reviewWords[currentIndex]
  const progress = ((currentIndex) / reviewWords.length) * 100
  const totalWords = reviewWords.length

  // Reset card start time when moving to new card
  useEffect(() => {
    setCardStartTime(Date.now())
  }, [currentIndex])

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (sessionComplete) return

      switch (e.key) {
        case ' ':
        case 'Enter':
          e.preventDefault()
          if (!showAnswer) {
            setShowAnswer(true)
            setIsFlipped(true)
          }
          break
        case '1':
          if (showAnswer) handleResult('forgot')
          break
        case '2':
          if (showAnswer) handleResult('hard')
          break
        case '3':
          if (showAnswer) handleResult('good')
          break
        case '4':
          if (showAnswer) handleResult('easy')
          break
        case 'ArrowLeft':
          if (currentIndex > 0 && !showAnswer) {
            setCurrentIndex(currentIndex - 1)
            setIsFlipped(false)
            setShowAnswer(false)
          }
          break
        case 'Escape':
          navigate('/review')
          break
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [showAnswer, currentIndex, sessionComplete, navigate])

  const handleResult = useCallback((result: ReviewResult) => {
    const timeSpent = Date.now() - cardStartTime

    setReviewRecords((prev) => [
      ...prev,
      { wordId: currentWord.id, result, timeSpent },
    ])

    if (currentIndex < totalWords - 1) {
      setCurrentIndex(currentIndex + 1)
      setIsFlipped(false)
      setShowAnswer(false)
    } else {
      setSessionComplete(true)
    }
  }, [cardStartTime, currentIndex, currentWord?.id, totalWords])

  const handleFlip = () => {
    setShowAnswer(true)
    setIsFlipped(true)
  }

  const playAudio = () => {
    // In real app, this would play TTS
    console.log('Playing audio for:', currentWord.word)
  }

  const restartSession = () => {
    setCurrentIndex(0)
    setIsFlipped(false)
    setShowAnswer(false)
    setReviewRecords([])
    setSessionComplete(false)
    setStartTime(Date.now())
    setCardStartTime(Date.now())
  }

  // Calculate session stats
  const calculateStats = () => {
    const forgot = reviewRecords.filter((r) => r.result === 'forgot').length
    const hard = reviewRecords.filter((r) => r.result === 'hard').length
    const good = reviewRecords.filter((r) => r.result === 'good').length
    const easy = reviewRecords.filter((r) => r.result === 'easy').length
    const totalTime = Date.now() - startTime
    const avgTime = reviewRecords.length > 0
      ? reviewRecords.reduce((acc, r) => acc + r.timeSpent, 0) / reviewRecords.length
      : 0

    return { forgot, hard, good, easy, totalTime, avgTime }
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">📚</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">开始复习</h1>
            <p className="text-gray-600 mb-6">登录后开始你的复习之旅</p>
            <Button asChild>
              <a href="/login?redirectTo=/review/session">登录开始复习</a>
            </Button>
          </div>
        </div>
        <Footer />
      </div>
    )
  }

  // Session complete screen
  if (sessionComplete) {
    const stats = calculateStats()
    const accuracy = ((stats.good + stats.easy) / totalWords) * 100

    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <main className="mx-auto max-w-2xl p-4">
          <div className="bg-white rounded-2xl shadow-lg overflow-hidden">
            {/* Header */}
            <div className="bg-gradient-to-r from-orange-500 to-amber-500 p-8 text-center text-white">
              <div className="inline-flex items-center justify-center w-20 h-20 bg-white/20 rounded-full mb-4">
                <Trophy className="h-10 w-10" />
              </div>
              <h1 className="text-3xl font-bold mb-2">复习完成!</h1>
              <p className="text-orange-100">太棒了，你完成了今天的复习任务</p>
            </div>

            {/* Stats */}
            <div className="p-6">
              {/* Main Stats */}
              <div className="grid grid-cols-3 gap-4 mb-6">
                <div className="text-center p-4 bg-gray-50 rounded-xl">
                  <div className="text-3xl font-bold text-gray-900">{totalWords}</div>
                  <div className="text-sm text-gray-500">复习单词</div>
                </div>
                <div className="text-center p-4 bg-green-50 rounded-xl">
                  <div className="text-3xl font-bold text-green-600">{accuracy.toFixed(0)}%</div>
                  <div className="text-sm text-gray-500">正确率</div>
                </div>
                <div className="text-center p-4 bg-blue-50 rounded-xl">
                  <div className="text-3xl font-bold text-blue-600">
                    {Math.floor(stats.totalTime / 60000)}:{String(Math.floor((stats.totalTime % 60000) / 1000)).padStart(2, '0')}
                  </div>
                  <div className="text-sm text-gray-500">用时</div>
                </div>
              </div>

              {/* Detailed Results */}
              <div className="space-y-3 mb-6">
                <h3 className="font-medium text-gray-900">详细结果</h3>
                <div className="grid grid-cols-4 gap-2">
                  <div className="flex items-center gap-2 p-3 bg-red-50 rounded-lg">
                    <ThumbsDown className="h-4 w-4 text-red-500" />
                    <div>
                      <div className="text-lg font-semibold text-red-600">{stats.forgot}</div>
                      <div className="text-xs text-gray-500">忘记</div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 p-3 bg-orange-50 rounded-lg">
                    <Clock className="h-4 w-4 text-orange-500" />
                    <div>
                      <div className="text-lg font-semibold text-orange-600">{stats.hard}</div>
                      <div className="text-xs text-gray-500">困难</div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 p-3 bg-green-50 rounded-lg">
                    <ThumbsUp className="h-4 w-4 text-green-500" />
                    <div>
                      <div className="text-lg font-semibold text-green-600">{stats.good}</div>
                      <div className="text-xs text-gray-500">记住</div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 p-3 bg-blue-50 rounded-lg">
                    <Zap className="h-4 w-4 text-blue-500" />
                    <div>
                      <div className="text-lg font-semibold text-blue-600">{stats.easy}</div>
                      <div className="text-xs text-gray-500">简单</div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Streak indicator */}
              <div className="flex items-center justify-center gap-2 p-4 bg-amber-50 rounded-xl mb-6">
                <Flame className="h-6 w-6 text-orange-500" />
                <span className="text-lg font-medium text-gray-900">连续学习 7 天</span>
                <span className="text-orange-500">🔥</span>
              </div>

              {/* Actions */}
              <div className="flex gap-3">
                <Button variant="outline" className="flex-1" onClick={() => navigate('/review')}>
                  <ArrowLeft className="h-4 w-4 mr-2" />
                  返回
                </Button>
                <Button className="flex-1" onClick={restartSession}>
                  <RotateCcw className="h-4 w-4 mr-2" />
                  再来一轮
                </Button>
              </div>
            </div>
          </div>
        </main>
        <Footer />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50 flex flex-col">
      <Header />

      <main className="flex-1 mx-auto w-full max-w-3xl p-4 flex flex-col">
        {/* Top bar with progress */}
        <div className="bg-white rounded-xl shadow-lg p-4 mb-4">
          <div className="flex items-center justify-between mb-3">
            <Button variant="ghost" size="sm" onClick={() => navigate('/review')}>
              <X className="h-4 w-4 mr-1" />
              退出
            </Button>
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <span className="font-medium">{currentIndex + 1}</span>
              <span>/</span>
              <span>{totalWords}</span>
            </div>
            <div className="flex items-center gap-2 text-sm text-gray-500">
              <Clock className="h-4 w-4" />
              <span>
                {Math.floor((Date.now() - startTime) / 60000)}:
                {String(Math.floor(((Date.now() - startTime) % 60000) / 1000)).padStart(2, '0')}
              </span>
            </div>
          </div>
          {/* Progress bar */}
          <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-orange-500 to-amber-500 transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>

        {/* Flashcard */}
        <div className="flex-1 flex items-center justify-center">
          <div
            className={cn(
              'w-full max-w-xl perspective-1000',
            )}
          >
            <div
              onClick={!showAnswer ? handleFlip : undefined}
              className={cn(
                'relative w-full min-h-[400px] cursor-pointer transition-transform duration-500 transform-style-3d',
                isFlipped && 'rotate-y-180',
                !showAnswer && 'hover:scale-[1.02]'
              )}
              style={{
                transformStyle: 'preserve-3d',
                transform: isFlipped ? 'rotateY(180deg)' : 'rotateY(0deg)',
              }}
            >
              {/* Front of card - Word */}
              <div
                className="absolute inset-0 bg-white rounded-2xl shadow-xl p-8 flex flex-col items-center justify-center backface-hidden"
                style={{ backfaceVisibility: 'hidden' }}
              >
                <div className="text-center">
                  <h1 className="text-4xl font-bold text-gray-900 mb-3">{currentWord.word}</h1>
                  <p className="text-lg text-gray-500 mb-6">{currentWord.phonetic}</p>
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      playAudio()
                    }}
                    className="inline-flex items-center gap-2 px-4 py-2 bg-orange-100 text-orange-600 rounded-full hover:bg-orange-200 transition-colors"
                  >
                    <Volume2 className="h-5 w-5" />
                    <span className="text-sm font-medium">播放发音</span>
                  </button>
                </div>

                {/* Mastery indicator */}
                <div className="absolute bottom-6 left-6 right-6">
                  <div className="flex items-center justify-between text-sm text-gray-500 mb-1">
                    <span>掌握度</span>
                    <span>{currentWord.mastery}%</span>
                  </div>
                  <div className="h-2 bg-gray-200 rounded-full">
                    <div
                      className={cn(
                        'h-2 rounded-full',
                        currentWord.mastery >= 80
                          ? 'bg-green-500'
                          : currentWord.mastery >= 50
                            ? 'bg-amber-500'
                            : 'bg-red-500'
                      )}
                      style={{ width: `${currentWord.mastery}%` }}
                    />
                  </div>
                </div>

                {/* Hint */}
                <p className="absolute bottom-24 text-sm text-gray-400">
                  点击卡片或按空格键查看答案
                </p>
              </div>

              {/* Back of card - Answer */}
              <div
                className="absolute inset-0 bg-white rounded-2xl shadow-xl p-8 flex flex-col backface-hidden"
                style={{
                  backfaceVisibility: 'hidden',
                  transform: 'rotateY(180deg)',
                }}
              >
                <div className="flex-1">
                  <div className="flex items-start justify-between mb-4">
                    <div>
                      <h2 className="text-2xl font-bold text-gray-900">{currentWord.word}</h2>
                      <p className="text-gray-500">{currentWord.phonetic}</p>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        playAudio()
                      }}
                      className="p-2 bg-orange-100 text-orange-600 rounded-full hover:bg-orange-200 transition-colors"
                    >
                      <Volume2 className="h-5 w-5" />
                    </button>
                  </div>

                  <div className="space-y-4">
                    <div>
                      <h3 className="text-sm font-medium text-gray-500 mb-1">释义</h3>
                      <p className="text-xl text-orange-600 font-medium">{currentWord.meaning}</p>
                    </div>

                    <div className="bg-gray-50 rounded-xl p-4">
                      <h3 className="text-sm font-medium text-gray-500 mb-2">例句</h3>
                      <p className="text-gray-900 mb-1">"{currentWord.example}"</p>
                      <p className="text-gray-500 text-sm">{currentWord.exampleZh}</p>
                    </div>

                    <div className="flex items-center gap-4 text-sm text-gray-500">
                      <span>已复习 {currentWord.reviewCount} 次</span>
                      <span>•</span>
                      <span>掌握度 {currentWord.mastery}%</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Rating buttons */}
        {showAnswer && (
          <div className="bg-white rounded-xl shadow-lg p-4 mt-4">
            <p className="text-center text-sm text-gray-500 mb-3">你对这个单词的掌握程度如何？</p>
            <div className="grid grid-cols-4 gap-2">
              <button
                onClick={() => handleResult('forgot')}
                className="flex flex-col items-center gap-1 p-3 rounded-xl bg-red-50 hover:bg-red-100 text-red-600 transition-colors"
              >
                <ThumbsDown className="h-5 w-5" />
                <span className="text-sm font-medium">忘记了</span>
                <span className="text-xs text-gray-500">1分钟后</span>
              </button>
              <button
                onClick={() => handleResult('hard')}
                className="flex flex-col items-center gap-1 p-3 rounded-xl bg-orange-50 hover:bg-orange-100 text-orange-600 transition-colors"
              >
                <Clock className="h-5 w-5" />
                <span className="text-sm font-medium">有点难</span>
                <span className="text-xs text-gray-500">10分钟后</span>
              </button>
              <button
                onClick={() => handleResult('good')}
                className="flex flex-col items-center gap-1 p-3 rounded-xl bg-green-50 hover:bg-green-100 text-green-600 transition-colors"
              >
                <ThumbsUp className="h-5 w-5" />
                <span className="text-sm font-medium">记住了</span>
                <span className="text-xs text-gray-500">1天后</span>
              </button>
              <button
                onClick={() => handleResult('easy')}
                className="flex flex-col items-center gap-1 p-3 rounded-xl bg-blue-50 hover:bg-blue-100 text-blue-600 transition-colors"
              >
                <Zap className="h-5 w-5" />
                <span className="text-sm font-medium">太简单</span>
                <span className="text-xs text-gray-500">4天后</span>
              </button>
            </div>
            <p className="text-center text-xs text-gray-400 mt-3">
              快捷键: 1-忘记 2-困难 3-记住 4-简单
            </p>
          </div>
        )}

        {/* Navigation hint when not showing answer */}
        {!showAnswer && (
          <div className="bg-white rounded-xl shadow-lg p-4 mt-4">
            <div className="flex items-center justify-center gap-6 text-sm text-gray-500">
              <span className="flex items-center gap-1">
                <kbd className="px-2 py-1 bg-gray-100 rounded text-xs">Space</kbd>
                翻转卡片
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-2 py-1 bg-gray-100 rounded text-xs">Esc</kbd>
                退出
              </span>
            </div>
          </div>
        )}
      </main>

      <Footer />
    </div>
  )
}
