import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { TrendingUp, Target, Lightbulb, ChevronRight, Search } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Input } from '../components/ui/input'
import { useAuth } from '../lib/auth'
import {
  getLearnSummary,
  getDailyData,
  listFavorites,
  getLocalSearchHistory,
  type LearnSummary,
  type DailyTask,
  type DailyInsight,
  type FavoriteWord,
} from '../lib/api'

// Stage matches backend asset_stages table
interface Stage {
  id: number
  name_en: string
  name_zh: string
  description_en: string | null
  description_zh: string | null
  icon_emoji: string | null
  difficulty: number | null
  is_active: boolean | null
}

interface StatCardProps {
  value: string
  label: string
  color: 'orange' | 'green' | 'amber' | 'blue'
}

function StatCard({ value, label, color }: StatCardProps) {
  const colorClasses = {
    orange: 'text-orange-600',
    green: 'text-green-600',
    amber: 'text-amber-600',
    blue: 'text-blue-600',
  }

  return (
    <div className="bg-gray-50 rounded-lg p-4 text-center">
      <div className={`text-2xl font-bold ${colorClasses[color]}`}>{value}</div>
      <div className="text-xs text-gray-500 mt-1">{label}</div>
    </div>
  )
}

interface TaskItemProps {
  task: DailyTask
}

function TaskItem({ task }: TaskItemProps) {
  const statusIcons = {
    completed: '✅',
    in_progress: '⏳',
    pending: '⭕',
  }
  const statusLabels = {
    completed: '已完成',
    in_progress: '进行中...',
    pending: '待完成',
  }

  return (
    <div className="bg-gray-50 rounded-lg p-3">
      <div className="font-medium text-sm text-gray-800">
        {statusIcons[task.status]} {task.task_title}
      </div>
      <div className="text-xs text-gray-500 mt-1">
        {task.task_description || statusLabels[task.status]}
        {task.target_value > 1 && ` (${task.current_value}/${task.target_value})`}
      </div>
    </div>
  )
}

interface StageCardProps {
  icon: string
  title: string
  subtitle: string
}

function StageCard({ icon, title, subtitle }: StageCardProps) {
  return (
    <div className="bg-gray-50 rounded-lg p-4 text-center hover:bg-orange-50 cursor-pointer transition-colors">
      <div className="text-3xl mb-2">{icon}</div>
      <div className="font-medium text-sm text-gray-800">{title}</div>
      <div className="text-xs text-gray-500 mt-1">{subtitle}</div>
    </div>
  )
}

interface InsightItemProps {
  insight: DailyInsight
}

function InsightItem({ insight }: InsightItemProps) {
  const bgColors = {
    progress: 'bg-green-50',
    weakness: 'bg-amber-50',
    encouragement: 'bg-blue-50',
    suggestion: 'bg-purple-50',
  }
  const textColors = {
    progress: 'text-green-600',
    weakness: 'text-amber-600',
    encouragement: 'text-blue-600',
    suggestion: 'text-purple-600',
  }
  const icons = {
    bulb: <Lightbulb className="h-4 w-4 inline mr-1" />,
    warning: <span className="mr-1">⚠️</span>,
    chart: <span className="mr-1">📈</span>,
  }

  const icon = icons[insight.icon_type as keyof typeof icons] || <span className="mr-1">💡</span>

  return (
    <div className={`${bgColors[insight.insight_type]} rounded-lg p-3`}>
      <p className="text-sm text-gray-700">
        {icon}
        {insight.content_zh}
      </p>
      {insight.action_link ? (
        <Link to={insight.action_link} className={`text-xs ${textColors[insight.insight_type]} mt-1 block hover:underline`}>
          {insight.action_text || '查看详情 →'}
        </Link>
      ) : insight.action_text ? (
        <span className={`text-xs ${textColors[insight.insight_type]} mt-1 block`}>{insight.action_text}</span>
      ) : null}
    </div>
  )
}

export function LandingPage() {
  const { token, user } = useAuth()
  const navigate = useNavigate()
  const [learnSummary, setLearnSummary] = useState<LearnSummary | null>(null)
  const [chartPeriod, setChartPeriod] = useState<'day' | 'week' | 'month'>('day')
  const [stages, setStages] = useState<Stage[]>([])
  const [searchWord, setSearchWord] = useState('')
  const [dailyTasks, setDailyTasks] = useState<DailyTask[]>([])
  const [dailyInsights, setDailyInsights] = useState<DailyInsight[]>([])
  const [dailyLoading, setDailyLoading] = useState(false)
  const [suggestedWords, setSuggestedWords] = useState<{ word: string; source: 'favorite' | 'history' }[]>([])

  // Helper to shuffle and pick random items
  const pickRandom = <T,>(arr: T[], count: number): T[] => {
    const shuffled = [...arr].sort(() => Math.random() - 0.5)
    return shuffled.slice(0, count)
  }

  const handleDictSearch = () => {
    const word = searchWord.trim()
    if (word) {
      navigate(`/dict/${encodeURIComponent(word)}`)
    }
  }

  const handleSearchKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleDictSearch()
    }
  }

  useEffect(() => {
    if (token) {
      getLearnSummary(token)
        .then(setLearnSummary)
        .catch(() => setLearnSummary(null))

      // Fetch daily tasks and insights
      setDailyLoading(true)
      getDailyData(token)
        .then((data) => {
          setDailyTasks(data.tasks)
          setDailyInsights(data.insights)
        })
        .catch(() => {
          setDailyTasks([])
          setDailyInsights([])
        })
        .finally(() => setDailyLoading(false))

      // Fetch favorite words for suggestion
      listFavorites(token, 'word', 20)
        .then((favorites) => {
          const wordFavorites = favorites.filter((f): f is FavoriteWord => f.entity === 'word')
          if (wordFavorites.length > 0) {
            const picked = pickRandom(wordFavorites, 5)
            setSuggestedWords(picked.map(f => ({ word: f.word, source: 'favorite' })))
          } else {
            // Fall back to local search history
            const history = getLocalSearchHistory()
            if (history.length > 0) {
              const picked = pickRandom(history, 5)
              setSuggestedWords(picked.map(h => ({ word: h.word, source: 'history' })))
            }
          }
        })
        .catch(() => {
          // Fall back to local search history on error
          const history = getLocalSearchHistory()
          if (history.length > 0) {
            const picked = pickRandom(history, 5)
            setSuggestedWords(picked.map(h => ({ word: h.word, source: 'history' })))
          }
        })
    } else {
      // Not logged in, use local search history
      const history = getLocalSearchHistory()
      if (history.length > 0) {
        const picked = pickRandom(history, 5)
        setSuggestedWords(picked.map(h => ({ word: h.word, source: 'history' })))
      }
    }
  }, [token])

  // Fetch stages for role-play section
  useEffect(() => {
    async function fetchStages() {
      try {
        const res = await fetch('/api/asset/stages')
        if (res.ok) {
          const data = await res.json()
          setStages(data.slice(0, 6)) // Show first 6 stages
        }
      } catch (err) {
        console.error('Failed to fetch stages:', err)
      }
    }
    fetchStages()
  }, [])

  const greeting = () => {
    const hour = new Date().getHours()
    if (hour < 12) return '早上好'
    if (hour < 18) return '下午好'
    return '晚上好'
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-6xl p-4">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {/* Left Column - Main Content */}
          <div className="lg:col-span-2 space-y-4">
            {/* Welcome Card */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <div className="flex flex-col sm:flex-row sm:items-start gap-4">
                <div className="flex-1">
                  <h1 className="text-xl font-semibold text-gray-900">
                    {greeting()}，{token ? (user?.name || '用户') : '欢迎来到开朗英语'}！
                  </h1>
                  <p className="text-gray-600 mt-1">
                    {token ? '今天想聊点什么？AI 已经准备好陪你练习了' : '开始你的英语学习之旅'}
                  </p>
                  <div className="flex gap-2 mt-4">
                    <Button asChild>
                      <Link to="/chat">
                        开始对话 <ChevronRight className="h-4 w-4 ml-1" />
                      </Link>
                    </Button>
                    <Button variant="outline" asChild>
                      <Link to="/stages">选择场景</Link>
                    </Button>
                  </div>
                </div>
                {/* Dictionary Search */}
                <div className="flex-1">
                  <p className="text-sm text-gray-500 mb-2">词典查询</p>
                  <div className="flex gap-2">
                    <Input
                      type="text"
                      placeholder="输入单词..."
                      value={searchWord}
                      onChange={(e) => setSearchWord(e.target.value)}
                      onKeyDown={handleSearchKeyDown}
                      className="flex-1"
                    />
                    <Button onClick={handleDictSearch} variant="outline" size="icon">
                      <Search className="h-4 w-4" />
                    </Button>
                  </div>
                  {suggestedWords.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-2">
                      {suggestedWords.map((item, idx) => (
                        <Link
                          key={idx}
                          to={`/dict/${encodeURIComponent(item.word)}`}
                          className="text-xs px-2 py-1 bg-gray-100 hover:bg-orange-100 rounded-full text-gray-600 hover:text-orange-600 transition-colors"
                          title={item.source === 'favorite' ? '收藏的单词' : '搜索历史'}
                        >
                          {item.word}
                        </Link>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>

            {/* Stats - Learning Data */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="font-semibold text-gray-900">
                  <TrendingUp className="h-5 w-5 inline mr-2 text-orange-500" />
                  学习数据
                </h2>
                <Button size="sm" variant="outline" asChild>
                  <Link to="/review">开始复习</Link>
                </Button>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-5 gap-4">
                {/* Left: Stats Cards - 2x2 grid */}
                <div className="sm:col-span-2 grid grid-cols-2 gap-3">
                  <StatCard
                    value={String(learnSummary?.weekly_chat_minutes ?? 0)}
                    label="本周对话(分钟)"
                    color="orange"
                  />
                  <StatCard
                    value={String(learnSummary?.mastered_vocabulary_count ?? 0)}
                    label="已掌握词汇"
                    color="green"
                  />
                  <StatCard
                    value={String(learnSummary?.pending_review_count ?? 0)}
                    label="待复习"
                    color="amber"
                  />
                  <StatCard
                    value="85%"
                    label="正确率"
                    color="blue"
                  />
                </div>
                {/* Right: Time Distribution Chart */}
                <div className="sm:col-span-3 bg-gray-50 rounded-lg p-4 flex flex-col h-full">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs text-gray-500">学习时长分布</span>
                    <div className="flex gap-1">
                      {(['day', 'week', 'month'] as const).map((period) => (
                        <button
                          key={period}
                          onClick={() => setChartPeriod(period)}
                          className={`px-2 py-0.5 text-xs rounded transition-colors ${
                            chartPeriod === period
                              ? 'bg-orange-500 text-white'
                              : 'bg-white text-gray-500 hover:bg-orange-50'
                          }`}
                        >
                          {period === 'day' ? '日' : period === 'week' ? '周' : '月'}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="relative flex flex-1">
                    {/* Y轴标注 */}
                    <div className="w-6 flex flex-col justify-between text-xs text-gray-400">
                      <span>60</span>
                      <span>30</span>
                      <span>0</span>
                    </div>
                    {/* 图表区域 */}
                    <div className="flex-1 flex flex-col">
                      {/* 柱状图 */}
                      <div className="flex items-end gap-1 flex-1 border-b border-gray-300">
                        {(chartPeriod === 'day'
                          ? (learnSummary?.weekly_minutes ?? Array(7).fill({ minutes: 0 }))
                          : chartPeriod === 'week'
                          ? Array(8).fill({ minutes: 0 })
                          : Array(6).fill({ minutes: 0 })
                        ).map((item, i) => (
                          <div key={i} className="flex-1 flex flex-col justify-end items-center h-full">
                            <div
                              className="w-full bg-orange-400 rounded-sm transition-all"
                              style={{ height: `${Math.min((item.minutes / 60) * 100, 100)}%` }}
                            />
                          </div>
                        ))}
                      </div>
                      {/* X轴标注 */}
                      <div className="flex gap-1 mt-1">
                        {(chartPeriod === 'day'
                          ? ['一', '二', '三', '四', '五', '六', '日']
                          : chartPeriod === 'week'
                          ? ['1周', '2周', '3周', '4周', '5周', '6周', '7周', '8周']
                          : ['1月', '2月', '3月', '4月', '5月', '6月']
                        ).map((label, i) => (
                          <div key={i} className="flex-1 text-center text-xs text-gray-400">
                            {label}
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Role-play Stages */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="font-semibold text-gray-900">
                  <span className="mr-2">🎭</span>
                  角色扮演
                </h2>
                <Link to="/stages" className="text-sm text-orange-600 hover:text-orange-700">
                  查看全部 →
                </Link>
              </div>
              <div className="grid grid-cols-3 gap-4">
                {stages.length > 0 ? (
                  stages.map((stage) => (
                    <Link key={stage.id} to={`/stages/${stage.id}`} className="block">
                      <StageCard
                        icon={stage.icon_emoji || '📚'}
                        title={stage.name_zh}
                        subtitle={stage.name_en}
                      />
                    </Link>
                  ))
                ) : (
                  // Fallback loading state
                  Array(6).fill(0).map((_, i) => (
                    <div key={i} className="bg-gray-50 rounded-lg p-4 text-center animate-pulse">
                      <div className="text-3xl mb-2 opacity-30">📚</div>
                      <div className="h-4 bg-gray-200 rounded w-16 mx-auto mb-2" />
                      <div className="h-3 bg-gray-200 rounded w-12 mx-auto" />
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>

          {/* Right Column - Tasks and Insights */}
          <div className="space-y-4">
            {/* Today's Tasks */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="font-semibold text-gray-900 flex items-center gap-2">
                  <Target className="h-5 w-5 text-orange-500" />
                  今日任务
                </h2>
                {dailyTasks.length > 0 && (
                  <span className="text-sm text-gray-500">
                    {dailyTasks.filter(t => t.status === 'completed').length}/{dailyTasks.length} 已完成
                  </span>
                )}
              </div>
              {dailyTasks.length > 0 && (
                <div className="h-2 bg-gray-200 rounded-full mb-4">
                  <div
                    className="h-2 bg-orange-500 rounded-full transition-all"
                    style={{ width: `${(dailyTasks.filter(t => t.status === 'completed').length / dailyTasks.length) * 100}%` }}
                  />
                </div>
              )}
              <div className="space-y-2">
                {dailyLoading ? (
                  <div className="text-sm text-gray-500 text-center py-4">加载中...</div>
                ) : dailyTasks.length > 0 ? (
                  dailyTasks.map((task) => <TaskItem key={task.id} task={task} />)
                ) : token ? (
                  <div className="text-sm text-gray-500 text-center py-4">暂无任务</div>
                ) : (
                  <div className="text-sm text-gray-500 text-center py-4">登录后查看今日任务</div>
                )}
              </div>
            </div>

            {/* AI Insights */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h2 className="font-semibold text-gray-900 mb-4">
                <span className="mr-2">🧠</span>
                AI 洞察
              </h2>
              <div className="space-y-3">
                {dailyLoading ? (
                  <div className="text-sm text-gray-500 text-center py-4">加载中...</div>
                ) : dailyInsights.length > 0 ? (
                  dailyInsights.map((insight) => <InsightItem key={insight.id} insight={insight} />)
                ) : token ? (
                  <div className="text-sm text-gray-500 text-center py-4">暂无洞察</div>
                ) : (
                  <div className="text-sm text-gray-500 text-center py-4">登录后查看 AI 洞察</div>
                )}
              </div>
            </div>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  )
}
