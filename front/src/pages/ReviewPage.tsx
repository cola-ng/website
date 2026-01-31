import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { Mic, BookOpen, PenLine, Layers, Puzzle, Sparkles, Loader2 } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { useStudyHeartbeat } from '../hooks/useStudyHeartbeat'
import { cn } from '../lib/utils'
import { getReviewStats, type ReviewStats } from '../lib/api'
import { REVIEW_TYPES, type ReviewType } from '../types/review'

const REVIEW_TYPE_ICONS: Record<ReviewType, React.ReactNode> = {
  pronunciation: <Mic className="h-6 w-6" />,
  meaning: <BookOpen className="h-6 w-6" />,
  spelling: <PenLine className="h-6 w-6" />,
  recognition: <Layers className="h-6 w-6" />,
  phrase: <Puzzle className="h-6 w-6" />,
}

const REVIEW_TYPE_COLORS: Record<ReviewType, { bg: string; text: string; border: string }> = {
  pronunciation: { bg: 'bg-red-50', text: 'text-red-600', border: 'border-red-200' },
  meaning: { bg: 'bg-blue-50', text: 'text-blue-600', border: 'border-blue-200' },
  spelling: { bg: 'bg-purple-50', text: 'text-purple-600', border: 'border-purple-200' },
  recognition: { bg: 'bg-green-50', text: 'text-green-600', border: 'border-green-200' },
  phrase: { bg: 'bg-amber-50', text: 'text-amber-600', border: 'border-amber-200' },
}

interface ReviewTypeCardProps {
  type: ReviewType
  title: string
  description: string
  count: number
}

function ReviewTypeCard({ type, title, description, count }: ReviewTypeCardProps) {
  const colors = REVIEW_TYPE_COLORS[type]
  const icon = REVIEW_TYPE_ICONS[type]

  return (
    <Link
      to={`/review/session/${type}`}
      className={cn(
        'block rounded-xl border-2 p-4 transition-all hover:shadow-lg hover:scale-[1.02]',
        colors.bg,
        colors.border
      )}
    >
      <div className="flex items-start gap-3">
        <div className={cn('p-2 rounded-lg', colors.bg, colors.text)}>{icon}</div>
        <div className="flex-1">
          <div className="flex items-center justify-between">
            <h3 className="font-semibold text-gray-900">{title}</h3>
            {count > 0 && (
              <span
                className={cn('text-sm font-medium px-2 py-0.5 rounded-full', colors.bg, colors.text)}
              >
                {count}
              </span>
            )}
          </div>
          <p className="text-sm text-gray-500 mt-1">{description}</p>
        </div>
      </div>
    </Link>
  )
}

interface SmartReviewCardProps {
  totalCount: number
}

function SmartReviewCard({ totalCount }: SmartReviewCardProps) {
  return (
    <Link
      to="/review/session"
      className="block rounded-xl border-2 p-4 transition-all hover:shadow-lg hover:scale-[1.02] bg-gradient-to-br from-orange-50 to-amber-50 border-orange-300 col-span-full sm:col-span-1"
    >
      <div className="flex items-start gap-3">
        <div className="p-2 rounded-lg bg-gradient-to-br from-orange-400 to-amber-500 text-white">
          <Sparkles className="h-6 w-6" />
        </div>
        <div className="flex-1">
          <div className="flex items-center justify-between">
            <h3 className="font-semibold text-gray-900">智能选择</h3>
            {totalCount > 0 && (
              <span className="text-sm font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-600">
                {totalCount}
              </span>
            )}
          </div>
          <p className="text-sm text-gray-500 mt-1">根据遗忘曲线智能推荐复习内容</p>
        </div>
      </div>
    </Link>
  )
}

export function ReviewPage() {
  const { token } = useAuth()
  useStudyHeartbeat('review')
  const [reviewStats, setReviewStats] = useState<ReviewStats | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!token) return

    const fetchData = async () => {
      setLoading(true)
      try {
        const statsData = await getReviewStats(token)
        setReviewStats(statsData)
      } catch (err) {
        console.error('Failed to fetch review stats:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [token])

  // Calculate total items needing review
  const totalCount = reviewStats
    ? REVIEW_TYPES.reduce((sum, t) => sum + (reviewStats[t.type] ?? 0), 0)
    : 0

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">📚</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">温故知新</h1>
            <p className="text-gray-600 mb-6">科学的间隔重复系统，帮助你巩固所学知识</p>
            <Button asChild>
              <a href="/login?redirectTo=/review">登录开始复习</a>
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
        <div className="bg-white rounded-xl shadow-lg p-6">
          <div className="flex items-baseline gap-3 mb-6">
            <h1 className="text-2xl font-bold text-gray-900">
              <span className="mr-2">📚</span>
              温故知新
            </h1>
            <p className="text-gray-500">科学复习，牢记所学</p>
          </div>

          {loading ? (
            <div className="py-12 text-center">
              <Loader2 className="h-8 w-8 animate-spin text-orange-500 mx-auto mb-4" />
              <p className="text-gray-500">加载中...</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              <SmartReviewCard totalCount={totalCount} />
              {REVIEW_TYPES.map((typeInfo) => (
                <ReviewTypeCard
                  key={typeInfo.type}
                  type={typeInfo.type}
                  title={typeInfo.title}
                  description={typeInfo.description}
                  count={reviewStats?.[typeInfo.type] ?? 0}
                />
              ))}
            </div>
          )}
        </div>
      </main>
      <Footer />
    </div>
  )
}
