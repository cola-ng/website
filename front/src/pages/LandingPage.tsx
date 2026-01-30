import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { TrendingUp, Target, Lightbulb, ChevronRight } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { getLearnSummary, type LearnSummary } from '../lib/api'

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
  title: string
  status: 'completed' | 'in_progress' | 'pending'
}

function TaskItem({ title, status }: TaskItemProps) {
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
        {statusIcons[status]} {title}
      </div>
      <div className="text-xs text-gray-500 mt-1">{statusLabels[status]}</div>
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

export function LandingPage() {
  const { token, user } = useAuth()
  const [learnSummary, setLearnSummary] = useState<LearnSummary | null>(null)
  const [chartPeriod, setChartPeriod] = useState<'day' | 'week' | 'month'>('day')
  const [stages, setStages] = useState<Stage[]>([])

  useEffect(() => {
    if (token) {
      getLearnSummary(token)
        .then(setLearnSummary)
        .catch(() => setLearnSummary(null))
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
              <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
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
                <div className="bg-orange-50 rounded-lg p-3 max-w-xs">
                  <div className="text-xs font-medium text-orange-600 mb-1">
                    <Lightbulb className="h-3 w-3 inline mr-1" />
                    AI 建议
                  </div>
                  <p className="text-sm text-gray-700">
                    "昨天我们聊了旅行，今天继续练习酒店预订怎么样？"
                  </p>
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
                <span className="text-sm text-gray-500">3/5 已完成</span>
              </div>
              <div className="h-2 bg-gray-200 rounded-full mb-4">
                <div className="h-2 bg-orange-500 rounded-full" style={{ width: '60%' }} />
              </div>
              <div className="space-y-2">
                <TaskItem title="3分钟自由对话" status="completed" />
                <TaskItem title="场景练习：点餐" status="in_progress" />
                <TaskItem title="复习 8 个易错点" status="pending" />
                <TaskItem title="跟读训练 5 句" status="pending" />
              </div>
            </div>

            {/* AI Insights - 4 items */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h2 className="font-semibold text-gray-900 mb-4">
                <span className="mr-2">🧠</span>
                AI 洞察
              </h2>
              <div className="space-y-3">
                <div className="bg-green-50 rounded-lg p-3">
                  <p className="text-sm text-gray-700">
                    <Lightbulb className="h-4 w-4 inline mr-1 text-green-600" />
                    你的冠词使用进步明显！a/an 错误率下降 40%
                  </p>
                  <span className="text-xs text-green-600 mt-1 block">持续保持</span>
                </div>
                <div className="bg-amber-50 rounded-lg p-3">
                  <p className="text-sm text-gray-700">
                    <span className="mr-1">⚠️</span>
                    建议多练习过去时态，这是你目前的薄弱点
                  </p>
                  <Link to="/review" className="text-xs text-amber-600 mt-1 block hover:underline">
                    点击开始专项练习 →
                  </Link>
                </div>
                <div className="bg-blue-50 rounded-lg p-3">
                  <p className="text-sm text-gray-700">
                    <span className="mr-1">📈</span>
                    本周学习时长比上周提升 25%，继续加油！
                  </p>
                  <span className="text-xs text-blue-600 mt-1 block">稳步提升中</span>
                </div>
                <div className="bg-purple-50 rounded-lg p-3">
                  <p className="text-sm text-gray-700">
                    <span className="mr-1">💡</span>
                    尝试"餐厅点餐"场景，巩固已学的日常用语
                  </p>
                  <Link to="/chat" className="text-xs text-purple-600 mt-1 block hover:underline">
                    立即体验 →
                  </Link>
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  )
}
