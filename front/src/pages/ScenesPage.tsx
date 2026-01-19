import { useState } from 'react'
import { Search, Play, Clock, ChevronRight, Film, Tv, Mic2 } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { cn } from '../lib/utils'

interface Scene {
  id: string
  icon: string
  title: string
  titleEn: string
  difficulty: 'beginner' | 'intermediate' | 'advanced'
  duration: string
  category: string
}

interface ClassicDialogue {
  id: string
  icon: string
  title: string
  source: 'movie' | 'tv_show' | 'ted_talk'
  description: string
}

const scenes: Scene[] = [
  { id: '1', icon: '🍽️', title: '餐厅点餐', titleEn: 'Restaurant Ordering', difficulty: 'beginner', duration: '5分钟', category: 'daily' },
  { id: '2', icon: '🏨', title: '酒店入住', titleEn: 'Hotel Check-in', difficulty: 'beginner', duration: '8分钟', category: 'travel' },
  { id: '3', icon: '✈️', title: '机场出行', titleEn: 'Airport Travel', difficulty: 'intermediate', duration: '10分钟', category: 'travel' },
  { id: '4', icon: '🛒', title: '超市购物', titleEn: 'Grocery Shopping', difficulty: 'beginner', duration: '5分钟', category: 'daily' },
  { id: '5', icon: '💼', title: '工作面试', titleEn: 'Job Interview', difficulty: 'advanced', duration: '15分钟', category: 'business' },
  { id: '6', icon: '🏥', title: '看病就医', titleEn: 'Doctor Visit', difficulty: 'intermediate', duration: '10分钟', category: 'daily' },
  { id: '7', icon: '🏦', title: '银行业务', titleEn: 'Banking', difficulty: 'intermediate', duration: '8分钟', category: 'daily' },
  { id: '8', icon: '📞', title: '电话预约', titleEn: 'Phone Booking', difficulty: 'intermediate', duration: '6分钟', category: 'daily' },
]

const classicDialogues: ClassicDialogue[] = [
  { id: '1', icon: '🎬', title: '《肖申克的救赎》', source: 'movie', description: '希望是个好东西' },
  { id: '2', icon: '📺', title: '《老友记》', source: 'tv_show', description: '日常对话精选' },
  { id: '3', icon: '🎤', title: 'TED: 你的身体语言', source: 'ted_talk', description: '自信表达技巧' },
  { id: '4', icon: '🎬', title: '《阿甘正传》', source: 'movie', description: '经典励志台词' },
]

const categories = [
  { id: 'all', label: '全部' },
  { id: 'daily', label: '日常生活' },
  { id: 'travel', label: '旅行出行' },
  { id: 'business', label: '商务职场' },
]

function DifficultyBadge({ level }: { level: 'beginner' | 'intermediate' | 'advanced' }) {
  const config = {
    beginner: { label: '初级', stars: 1, color: 'text-green-600 bg-green-50' },
    intermediate: { label: '中级', stars: 2, color: 'text-amber-600 bg-amber-50' },
    advanced: { label: '高级', stars: 3, color: 'text-red-600 bg-red-50' },
  }
  const { stars, color } = config[level]

  return (
    <span className={cn('text-xs px-2 py-0.5 rounded-full', color)}>
      {'⭐'.repeat(stars)}
    </span>
  )
}

function SceneCard({ scene }: { scene: Scene }) {
  return (
    <div className="bg-white border rounded-xl p-4 hover:shadow-lg hover:border-orange-200 transition-all cursor-pointer group">
      <div className="text-4xl mb-3">{scene.icon}</div>
      <h3 className="font-semibold text-gray-900">{scene.title}</h3>
      <p className="text-sm text-gray-500 mb-2">{scene.titleEn}</p>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <DifficultyBadge level={scene.difficulty} />
          <span className="text-xs text-gray-400 flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {scene.duration}
          </span>
        </div>
        <Button size="sm" variant="ghost" className="opacity-0 group-hover:opacity-100 transition-opacity">
          <Play className="h-4 w-4" />
        </Button>
      </div>
    </div>
  )
}

function ClassicCard({ dialogue }: { dialogue: ClassicDialogue }) {
  const sourceIcons = {
    movie: <Film className="h-4 w-4" />,
    tv_show: <Tv className="h-4 w-4" />,
    ted_talk: <Mic2 className="h-4 w-4" />,
  }

  return (
    <div className="bg-white border rounded-xl p-4 hover:shadow-lg hover:border-orange-200 transition-all cursor-pointer flex items-center gap-4">
      <div className="w-16 h-16 bg-gray-900 rounded-lg flex items-center justify-center text-2xl shrink-0">
        {dialogue.icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <h3 className="font-semibold text-gray-900 truncate">{dialogue.title}</h3>
          <span className="text-gray-400">{sourceIcons[dialogue.source]}</span>
        </div>
        <p className="text-sm text-orange-600 mt-1">{dialogue.description}</p>
      </div>
      <ChevronRight className="h-5 w-5 text-gray-300" />
    </div>
  )
}

export function ScenesPage() {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState('all')

  const filteredScenes = scenes.filter((scene) => {
    const matchesSearch = scene.title.includes(searchQuery) || scene.titleEn.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesCategory = selectedCategory === 'all' || scene.category === selectedCategory
    return matchesSearch && matchesCategory
  })

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-6xl p-4">
        {/* Header */}
        <div className="flex items-baseline gap-3 mb-6">
          <h1 className="text-2xl font-bold text-gray-900">
            <span className="mr-2">🎭</span>
            场景中心
          </h1>
          <p className="text-gray-500">沉浸式场景模拟 · AI智能推荐 · 经典对白学习</p>
        </div>

        {/* Search, Filters & Continue Learning - Compact Layout */}
        <div className="bg-white rounded-xl shadow-sm border p-3 mb-6 space-y-2">
          {/* Row 1: Search and Filters */}
          <div className="flex flex-col sm:flex-row gap-2">
            <div className="relative flex-1 min-w-0">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="搜索场景..."
                className="w-full pl-9 pr-3 py-1.5 text-sm border rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent bg-gray-50"
              />
            </div>
            <div className="flex gap-1.5 flex-shrink-0">
              {categories.map((cat) => (
                <button
                  key={cat.id}
                  onClick={() => setSelectedCategory(cat.id)}
                  className={cn(
                    'px-3 py-1.5 rounded-lg text-xs font-medium transition-colors whitespace-nowrap',
                    selectedCategory === cat.id
                      ? 'bg-orange-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-orange-50'
                  )}
                >
                  {cat.label}
                </button>
              ))}
            </div>
          </div>

          {/* Row 2: Continue Learning */}
          <div className="flex items-center gap-3 px-3 py-2 bg-gradient-to-r from-orange-50 to-amber-50 rounded-lg">
            <div className="text-2xl">🏨</div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="font-medium text-sm text-gray-900">酒店入住</span>
                <span className="text-xs text-gray-500">进度 60%</span>
                <span className="text-xs text-gray-400">·</span>
                <span className="text-xs text-orange-600">下一个：前台预订房间</span>
              </div>
            </div>
            <Button size="sm" className="h-7 text-xs px-3 flex-shrink-0">
              继续学习 <ChevronRight className="h-3 w-3 ml-0.5" />
            </Button>
          </div>
        </div>

        {/* Today's Featured */}
        <div className="mb-6">
          <h2 className="font-semibold text-gray-900 mb-4">
            <span className="mr-2">🌟</span>
            今日精选
          </h2>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
            {filteredScenes.slice(0, 8).map((scene) => (
              <SceneCard key={scene.id} scene={scene} />
            ))}
          </div>
        </div>

        {/* Classic Dialogues */}
        <div>
          <div className="flex items-center justify-between mb-4">
            <h2 className="font-semibold text-gray-900">
              <span className="mr-2">🎬</span>
              经典对白
            </h2>
            <span className="text-sm text-gray-500">从电影/美剧中学习地道表达</span>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {classicDialogues.map((dialogue) => (
              <ClassicCard key={dialogue.id} dialogue={dialogue} />
            ))}
          </div>
        </div>
      </main>
      <Footer />
    </div>
  )
}
