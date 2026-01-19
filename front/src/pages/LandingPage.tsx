import { Link } from 'react-router-dom'
import { MessageSquare, RotateCcw, Theater, Mic, BookOpen, TrendingUp, Target, Lightbulb, ChevronRight } from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'

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

interface QuickActionProps {
  icon: React.ReactNode
  label: string
  to: string
}

function QuickAction({ icon, label, to }: QuickActionProps) {
  return (
    <Link
      to={to}
      className="flex flex-col items-center gap-2 p-4 bg-gray-50 rounded-lg hover:bg-orange-50 hover:border-orange-200 border border-transparent transition-all"
    >
      <div className="h-10 w-10 rounded-full bg-white shadow-sm flex items-center justify-center">
        {icon}
      </div>
      <span className="text-sm text-gray-700 font-medium">{label}</span>
    </Link>
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

interface SceneCardProps {
  icon: string
  title: string
  subtitle: string
}

function SceneCard({ icon, title, subtitle }: SceneCardProps) {
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
                      <Link to="/conversation">
                        开始对话 <ChevronRight className="h-4 w-4 ml-1" />
                      </Link>
                    </Button>
                    <Button variant="outline" asChild>
                      <Link to="/scenes">选择场景</Link>
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
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <TaskItem title="3分钟自由对话" status="completed" />
                <TaskItem title="场景练习：点餐" status="in_progress" />
                <TaskItem title="复习 8 个易错点" status="pending" />
                <TaskItem title="跟读训练 5 句" status="pending" />
              </div>
            </div>

            {/* Quick Actions */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h2 className="font-semibold text-gray-900 mb-4">
                <span className="mr-2">⚡</span>
                快捷入口
              </h2>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                <QuickAction
                  icon={<Theater className="h-5 w-5 text-purple-600" />}
                  label="场景模拟"
                  to="/scenes"
                />
                <QuickAction
                  icon={<MessageSquare className="h-5 w-5 text-blue-600" />}
                  label="经典对白"
                  to="/scenes"
                />
                <QuickAction
                  icon={<Mic className="h-5 w-5 text-red-600" />}
                  label="跟读练习"
                  to="/reading"
                />
                <QuickAction
                  icon={<BookOpen className="h-5 w-5 text-amber-600" />}
                  label="词典查询"
                  to="/dict"
                />
              </div>
            </div>
          </div>

          {/* Right Column - Stats and Insights */}
          <div className="space-y-4">
            {/* Stats */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h2 className="font-semibold text-gray-900 mb-4">
                <TrendingUp className="h-5 w-5 inline mr-2 text-orange-500" />
                学习数据
              </h2>
              <div className="grid grid-cols-2 gap-3">
                <StatCard value="47" label="本周对话(分钟)" color="orange" />
                <StatCard value="156" label="已掌握词汇" color="green" />
                <StatCard value="23" label="待复习" color="amber" />
                <StatCard value="B1" label="当前水平" color="blue" />
              </div>
              <div className="mt-4 bg-gray-50 rounded-lg p-3">
                <div className="text-xs text-gray-500">本周学习时长分布</div>
                <div className="flex items-center gap-1 mt-2">
                  {[3, 5, 4, 2, 6, 5, 4].map((h, i) => (
                    <div
                      key={i}
                      className="flex-1 bg-orange-400 rounded-sm"
                      style={{ height: `${h * 6}px` }}
                    />
                  ))}
                </div>
              </div>
            </div>

            {/* AI Insights */}
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
              </div>
            </div>

            {/* Recommended Scenes */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h2 className="font-semibold text-gray-900 mb-4">
                <span className="mr-2">🎯</span>
                推荐场景
              </h2>
              <div className="grid grid-cols-3 gap-2">
                <SceneCard icon="🏨" title="酒店入住" subtitle="继续上次" />
                <SceneCard icon="🍽️" title="餐厅点餐" subtitle="新场景" />
                <SceneCard icon="💼" title="工作面试" subtitle="挑战" />
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
