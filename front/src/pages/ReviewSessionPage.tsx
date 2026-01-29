import { useState, useEffect, useCallback } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { X, Clock, Trophy, Flame, ArrowLeft, RotateCcw, Loader2 } from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import {
  getReviewItems,
  submitReview,
  type ReviewItem,
  type SubmitReviewRequest,
} from '../lib/api'
import { REVIEW_TYPES, type ReviewType, type ReviewResult } from '../types/review'
import {
  PronunciationPractice,
  MeaningPractice,
  SpellingPractice,
  RecognitionPractice,
  PhrasePractice,
} from '../components/review'

export function ReviewSessionPage() {
  const { type } = useParams<{ type: string }>()
  const { token } = useAuth()
  const navigate = useNavigate()

  const [items, setItems] = useState<ReviewItem[]>([])
  const [currentIndex, setCurrentIndex] = useState(0)
  const [results, setResults] = useState<ReviewResult[]>([])
  const [loading, setLoading] = useState(true)
  const [sessionComplete, setSessionComplete] = useState(false)
  const [startTime] = useState(() => Date.now())

  // Validate and get review type
  const reviewType = (type as ReviewType) || 'recognition'
  const typeInfo = REVIEW_TYPES.find((t) => t.type === reviewType)

  // Fetch items
  useEffect(() => {
    if (!token) return

    const fetchItems = async () => {
      setLoading(true)
      try {
        const response = await getReviewItems(token, reviewType, 20)
        setItems(response.items)
      } catch (err) {
        console.error('Failed to fetch review items:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchItems()
  }, [token, reviewType])

  const currentItem = items[currentIndex]
  const progress = items.length > 0 ? (currentIndex / items.length) * 100 : 0

  // Handle practice completion
  const handleComplete = useCallback(
    async (result: ReviewResult) => {
      if (!token || !currentItem) return

      // Save result locally
      setResults((prev) => [...prev, result])

      // Submit to backend
      try {
        const submitData: SubmitReviewRequest = {
          source: currentItem.source,
          source_id: currentItem.source_id,
          review_type: reviewType,
          result: result.result,
          score: result.score,
          time_spent_ms: result.time_spent_ms,
        }
        await submitReview(token, submitData)
      } catch (err) {
        console.error('Failed to submit review:', err)
      }

      // Move to next item or complete session
      if (currentIndex < items.length - 1) {
        setCurrentIndex(currentIndex + 1)
      } else {
        setSessionComplete(true)
      }
    },
    [token, currentItem, currentIndex, items.length, reviewType]
  )

  // Handle skip
  const handleSkip = useCallback(() => {
    if (currentIndex < items.length - 1) {
      setCurrentIndex(currentIndex + 1)
    } else {
      setSessionComplete(true)
    }
  }, [currentIndex, items.length])

  // Restart session
  const restartSession = () => {
    setCurrentIndex(0)
    setResults([])
    setSessionComplete(false)
  }

  // Calculate stats
  const calculateStats = () => {
    const correct = results.filter((r) => r.result === 'correct').length
    const partial = results.filter((r) => r.result === 'partial').length
    const incorrect = results.filter((r) => r.result === 'incorrect').length
    const totalTime = Date.now() - startTime
    const avgScore =
      results.length > 0
        ? results.reduce((acc, r) => acc + (r.score || 0), 0) / results.length
        : 0

    return { correct, partial, incorrect, totalTime, avgScore }
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
              <a href={`/login?redirectTo=/review/session/${reviewType}`}>登录开始复习</a>
            </Button>
          </div>
        </div>
      </div>
    )
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <Loader2 className="h-12 w-12 animate-spin text-orange-500 mx-auto mb-4" />
            <p className="text-gray-600">加载复习内容中...</p>
          </div>
        </div>
      </div>
    )
  }

  if (items.length === 0) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">🎉</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">暂无待复习内容</h1>
            <p className="text-gray-600 mb-6">
              {typeInfo?.title || '该类型'}的复习内容已全部完成，继续学习新内容吧！
            </p>
            <Button asChild>
              <a href="/review">返回复习中心</a>
            </Button>
          </div>
        </div>
      </div>
    )
  }

  // Session complete screen
  if (sessionComplete) {
    const stats = calculateStats()
    const accuracy = items.length > 0 ? ((stats.correct + stats.partial) / items.length) * 100 : 0

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
              <h1 className="text-3xl font-bold mb-2">{typeInfo?.title}完成!</h1>
              <p className="text-orange-100">太棒了，你完成了这轮复习</p>
            </div>

            {/* Stats */}
            <div className="p-6">
              <div className="grid grid-cols-3 gap-4 mb-6">
                <div className="text-center p-4 bg-gray-50 rounded-xl">
                  <div className="text-3xl font-bold text-gray-900">{items.length}</div>
                  <div className="text-sm text-gray-500">复习项目</div>
                </div>
                <div className="text-center p-4 bg-green-50 rounded-xl">
                  <div className="text-3xl font-bold text-green-600">{accuracy.toFixed(0)}%</div>
                  <div className="text-sm text-gray-500">正确率</div>
                </div>
                <div className="text-center p-4 bg-blue-50 rounded-xl">
                  <div className="text-3xl font-bold text-blue-600">
                    {Math.floor(stats.totalTime / 60000)}:
                    {String(Math.floor((stats.totalTime % 60000) / 1000)).padStart(2, '0')}
                  </div>
                  <div className="text-sm text-gray-500">用时</div>
                </div>
              </div>

              {/* Detailed results */}
              <div className="grid grid-cols-3 gap-3 mb-6">
                <div className="p-3 bg-green-50 rounded-lg text-center">
                  <div className="text-2xl font-bold text-green-600">{stats.correct}</div>
                  <div className="text-sm text-gray-500">正确</div>
                </div>
                <div className="p-3 bg-amber-50 rounded-lg text-center">
                  <div className="text-2xl font-bold text-amber-600">{stats.partial}</div>
                  <div className="text-sm text-gray-500">部分正确</div>
                </div>
                <div className="p-3 bg-red-50 rounded-lg text-center">
                  <div className="text-2xl font-bold text-red-600">{stats.incorrect}</div>
                  <div className="text-sm text-gray-500">错误</div>
                </div>
              </div>

              {/* Streak */}
              <div className="flex items-center justify-center gap-2 p-4 bg-amber-50 rounded-xl mb-6">
                <Flame className="h-6 w-6 text-orange-500" />
                <span className="text-lg font-medium text-gray-900">继续保持学习！</span>
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
      </div>
    )
  }

  // Render appropriate practice component
  const renderPractice = () => {
    if (!currentItem) return null

    const commonProps = {
      item: currentItem,
      token,
      onComplete: handleComplete,
      onSkip: handleSkip,
    }

    switch (reviewType) {
      case 'pronunciation':
        return <PronunciationPractice {...commonProps} />
      case 'meaning':
        return <MeaningPractice {...commonProps} allItems={items} />
      case 'spelling':
        return <SpellingPractice {...commonProps} />
      case 'recognition':
        return <RecognitionPractice {...commonProps} />
      case 'phrase':
        return <PhrasePractice {...commonProps} />
      default:
        return <RecognitionPractice {...commonProps} />
    }
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
            <div className="text-center">
              <span className="text-sm font-medium text-gray-900">{typeInfo?.title}</span>
              <div className="text-xs text-gray-500">
                {currentIndex + 1} / {items.length}
              </div>
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

        {/* Practice content */}
        <div className="flex-1 bg-white rounded-xl shadow-lg p-6">{renderPractice()}</div>
      </main>
    </div>
  )
}
