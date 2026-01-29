import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import {
  RotateCcw,
  CheckCircle,
  XCircle,
  TrendingUp,
  Clock,
  Target,
  Mic,
  BookOpen,
  PenLine,
  Layers,
  Puzzle,
} from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'
import {
  getLearnSummary,
  listVocabulary,
  listIssueWords,
  getReviewStats,
  type LearnSummary,
  type UserVocabulary,
  type IssueWord,
  type ReviewStats,
} from '../lib/api'
import { REVIEW_TYPES, type ReviewType } from '../types/review'

interface ReviewWord {
  id: string
  word: string
  meaning: string
  example: string
  lastReview: string
  mastery: number
  dueIn?: string
}

function formatTimeAgo(dateStr: string | null): string {
  if (!dateStr) return '从未'
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (diffDays === 0) return '今天'
  if (diffDays === 1) return '1 天前'
  if (diffDays < 7) return `${diffDays} 天前`
  if (diffDays < 14) return '1 周前'
  if (diffDays < 30) return `${Math.floor(diffDays / 7)} 周前`
  return `${Math.floor(diffDays / 30)} 月前`
}

function formatDueIn(dateStr: string | null): string {
  if (!dateStr) return '现在'
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = date.getTime() - now.getTime()

  if (diffMs <= 0) return '现在'

  const diffHours = Math.floor(diffMs / (1000 * 60 * 60))
  if (diffHours < 1) return '即将'
  if (diffHours < 24) return `${diffHours} 小时后`
  const diffDays = Math.floor(diffHours / 24)
  return `${diffDays} 天后`
}

function vocabToReviewWord(vocab: UserVocabulary): ReviewWord {
  return {
    id: vocab.id.toString(),
    word: vocab.word,
    meaning: vocab.word_zh || '',
    example: '',
    lastReview: formatTimeAgo(vocab.last_practiced_at),
    mastery: (vocab.mastery_level || 1) * 20,
    dueIn: formatDueIn(vocab.next_review_at),
  }
}

function issueToReviewWord(issue: IssueWord): ReviewWord {
  return {
    id: issue.id.toString(),
    word: issue.word,
    meaning: issue.description_zh || issue.description_en || '',
    example: issue.context || '',
    lastReview: formatTimeAgo(issue.last_picked_at),
    mastery: issue.difficulty ? (5 - issue.difficulty) * 20 : 40,
  }
}

function WordCard({ word, showDue = false }: { word: ReviewWord; showDue?: boolean }) {
  const [flipped, setFlipped] = useState(false)

  return (
    <div
      onClick={() => setFlipped(!flipped)}
      className="bg-white border rounded-xl p-4 cursor-pointer hover:shadow-md transition-all"
    >
      {!flipped ? (
        <div>
          <div className="flex items-start justify-between">
            <h3 className="text-lg font-semibold text-gray-900">{word.word}</h3>
            {showDue && word.dueIn && (
              <span className={cn(
                'text-xs px-2 py-0.5 rounded-full',
                word.dueIn === 'Now' ? 'bg-red-100 text-red-600' : 'bg-amber-100 text-amber-600'
              )}>
                {word.dueIn}
              </span>
            )}
          </div>
          <p className="text-sm text-gray-500 mt-1">点击查看释义</p>
          <div className="mt-3 flex items-center gap-2">
            <div className="flex-1 h-2 bg-gray-200 rounded-full">
              <div
                className={cn(
                  'h-2 rounded-full',
                  word.mastery >= 80 ? 'bg-green-500' : word.mastery >= 50 ? 'bg-amber-500' : 'bg-red-500'
                )}
                style={{ width: `${word.mastery}%` }}
              />
            </div>
            <span className="text-xs text-gray-500">{word.mastery}%</span>
          </div>
        </div>
      ) : (
        <div>
          <h3 className="text-lg font-semibold text-gray-900">{word.word}</h3>
          <p className="text-orange-600 font-medium mt-2">{word.meaning}</p>
          <p className="text-sm text-gray-600 mt-2 italic">"{word.example}"</p>
          <p className="text-xs text-gray-400 mt-2">上次复习: {word.lastReview}</p>
        </div>
      )}
    </div>
  )
}

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
              <span className={cn('text-sm font-medium px-2 py-0.5 rounded-full', colors.bg, colors.text)}>
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

interface StatsPanelProps {
  summary: LearnSummary | null
}

function StatsPanel({ summary }: StatsPanelProps) {
  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
      <div className="bg-orange-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-orange-600">{summary?.pending_review_count ?? 0}</div>
        <div className="text-xs text-gray-500">待复习</div>
      </div>
      <div className="bg-green-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-green-600">{summary?.mastered_vocabulary_count ?? 0}</div>
        <div className="text-xs text-gray-500">已掌握</div>
      </div>
      <div className="bg-red-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-red-600">{summary?.issue_words_count ?? 0}</div>
        <div className="text-xs text-gray-500">易错点</div>
      </div>
      <div className="bg-blue-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-blue-600">{summary?.average_mastery ?? 0}%</div>
        <div className="text-xs text-gray-500">平均掌握度</div>
      </div>
    </div>
  )
}

export function ReviewPage() {
  const { token } = useAuth()
  const [summary, setSummary] = useState<LearnSummary | null>(null)
  const [reviewStats, setReviewStats] = useState<ReviewStats | null>(null)
  const [dueWords, setDueWords] = useState<ReviewWord[]>([])
  const [masteredWords, setMasteredWords] = useState<ReviewWord[]>([])
  const [mistakes, setMistakes] = useState<ReviewWord[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!token) return

    const fetchData = async () => {
      setLoading(true)
      try {
        const [summaryData, statsData, vocabData, issueData] = await Promise.all([
          getLearnSummary(token),
          getReviewStats(token),
          listVocabulary(token, false, 100),
          listIssueWords(token, false, 50),
        ])

        setSummary(summaryData)
        setReviewStats(statsData)

        // Split vocabulary into due and mastered
        const due = vocabData
          .filter((v) => (v.mastery_level || 0) < 4)
          .map(vocabToReviewWord)
        const mastered = vocabData
          .filter((v) => (v.mastery_level || 0) >= 4)
          .map(vocabToReviewWord)

        setDueWords(due)
        setMasteredWords(mastered)
        setMistakes(issueData.map(issueToReviewWord))
      } catch (err) {
        console.error('Failed to fetch review data:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [token])

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">📚</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">温故知新</h1>
            <p className="text-gray-600 mb-6">
              科学的间隔重复系统，帮助你巩固所学知识
            </p>
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
        <div className="bg-white rounded-xl shadow-lg p-6 mb-4">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-baseline gap-3">
              <h1 className="text-2xl font-bold text-gray-900">
                <span className="mr-2">📚</span>
                温故知新
              </h1>
              <p className="text-gray-500">科学复习，牢记所学</p>
            </div>
          </div>
          <StatsPanel summary={summary} />
        </div>

        {/* Review Type Selection */}
        <div className="bg-white rounded-xl shadow-lg p-6 mb-4">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">选择练习类型</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
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
        </div>

        {loading ? (
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600 mx-auto mb-4"></div>
            <p className="text-gray-500">加载中...</p>
          </div>
        ) : (
        <div className="bg-white rounded-xl shadow-lg overflow-hidden">
          <Tabs defaultValue="due">
            <div className="border-b px-6 py-3">
              <TabsList>
                <TabsTrigger value="due" className="gap-2">
                  <Clock className="h-4 w-4" />
                  待复习
                </TabsTrigger>
                <TabsTrigger value="mistakes" className="gap-2">
                  <XCircle className="h-4 w-4" />
                  易错点
                </TabsTrigger>
                <TabsTrigger value="mastered" className="gap-2">
                  <CheckCircle className="h-4 w-4" />
                  已掌握
                </TabsTrigger>
                <TabsTrigger value="stats" className="gap-2">
                  <TrendingUp className="h-4 w-4" />
                  统计
                </TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="due" className="p-6">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {dueWords.map((word) => (
                  <WordCard key={word.id} word={word} showDue />
                ))}
              </div>
              {dueWords.length === 0 && (
                <div className="text-center py-8 text-gray-500">
                  太棒了！暂时没有需要复习的内容
                </div>
              )}
            </TabsContent>

            <TabsContent value="mistakes" className="p-6">
              <div className="mb-4 bg-amber-50 rounded-lg p-4">
                <p className="text-sm text-amber-800">
                  <Target className="h-4 w-4 inline mr-2" />
                  这些是你经常出错的词汇，多加练习可以帮助你克服这些难点
                </p>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {mistakes.map((word) => (
                  <WordCard key={word.id} word={word} />
                ))}
              </div>
            </TabsContent>

            <TabsContent value="mastered" className="p-6">
              <div className="mb-4 bg-green-50 rounded-lg p-4">
                <p className="text-sm text-green-800">
                  <CheckCircle className="h-4 w-4 inline mr-2" />
                  这些词汇你已经掌握得很好了，继续保持！
                </p>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {masteredWords.map((word) => (
                  <WordCard key={word.id} word={word} />
                ))}
              </div>
            </TabsContent>

            <TabsContent value="stats" className="p-6">
              <div className="space-y-6">
                <div>
                  <h3 className="font-medium text-gray-900 mb-3">学习概览</h3>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">总学习词汇</div>
                      <div className="text-2xl font-bold text-gray-900">{summary?.total_vocabulary_count ?? 0}</div>
                    </div>
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">学习天数</div>
                      <div className="text-2xl font-bold text-gray-900">{summary?.learning_days ?? 0}</div>
                    </div>
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">复习次数</div>
                      <div className="text-2xl font-bold text-gray-900">{summary?.total_review_times ?? 0}</div>
                    </div>
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">平均掌握度</div>
                      <div className="text-2xl font-bold text-gray-900">{summary?.average_mastery ?? 0}%</div>
                    </div>
                  </div>
                </div>

                <div>
                  <h3 className="font-medium text-gray-900 mb-3">本周复习</h3>
                  <div className="flex items-end gap-2 h-32">
                    {['一', '二', '三', '四', '五', '六', '日'].map((day, i) => {
                      const minutes = summary?.weekly_minutes?.[i]?.minutes ?? 0
                      const maxMinutes = Math.max(...(summary?.weekly_minutes?.map(w => w.minutes) ?? [1]), 1)
                      const height = maxMinutes > 0 ? (minutes / maxMinutes) * 100 : 0
                      return (
                        <div key={day} className="flex-1 flex flex-col items-center gap-1">
                          <div
                            className="w-full bg-orange-400 rounded-t"
                            style={{ height: `${Math.max(height, 5)}%` }}
                          />
                          <span className="text-xs text-gray-500">{day}</span>
                        </div>
                      )
                    })}
                  </div>
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </div>
        )}
      </main>
      <Footer />
    </div>
  )
}
