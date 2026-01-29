import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { Search, Play, ChevronRight } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { cn } from '../lib/utils'

// Stage matches backend asset_stages table
interface Stage {
  id: number
  name_en: string
  name_zh: string
  description_en: string | null
  description_zh: string | null
  icon_emoji: string | null
  display_order: number | null
  difficulty: number | null
  is_active: boolean | null
  created_at: string
}

const categories = [
  { id: 'all', label: '全部' },
  { id: 'daily', label: '日常生活' },
  { id: 'travel', label: '旅行出行' },
  { id: 'business', label: '商务职场' },
  { id: 'entertainment', label: '娱乐休闲' },
]

function DifficultyBadge({ level }: { level: number | null }) {
  // difficulty levels: 1-2 = beginner, 3 = intermediate, 4+ = advanced
  let stars = 1
  let color = 'text-green-600 bg-green-50'

  if (level !== null) {
    if (level <= 2) {
      stars = 1
      color = 'text-green-600 bg-green-50'
    } else if (level <= 3) {
      stars = 2
      color = 'text-amber-600 bg-amber-50'
    } else {
      stars = 3
      color = 'text-red-600 bg-red-50'
    }
  }

  return (
    <span className={cn('text-xs px-2 py-0.5 rounded-full', color)}>
      {'⭐'.repeat(stars)}
    </span>
  )
}

function StageCard({ stage }: { stage: Stage }) {
  return (
    <Link
      to={`/stages/${stage.id}`}
      className="bg-white border rounded-xl p-4 hover:shadow-lg hover:border-orange-200 transition-all cursor-pointer group block"
    >
      <div className="text-4xl mb-3">{stage.icon_emoji || '📚'}</div>
      <h3 className="font-semibold text-gray-900">{stage.name_zh}</h3>
      <p className="text-sm text-gray-500 mb-2">{stage.name_en}</p>
      <div className="flex items-center justify-between">
        <DifficultyBadge level={stage.difficulty} />
        <Button size="sm" variant="ghost" className="opacity-0 group-hover:opacity-100 transition-opacity">
          <Play className="h-4 w-4" />
        </Button>
      </div>
    </Link>
  )
}

export function StagesPage() {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState('all')
  const [stages, setStages] = useState<Stage[]>([])
  const [loading, setLoading] = useState(true)

  // Fetch stage from API
  useEffect(() => {
    async function fetchData() {
      try {
        setLoading(true)
        const stagesRes = await fetch('/api/asset/stages')

        if (stagesRes.ok) {
          const stagesData = await stagesRes.json()
          setStages(stagesData)
        }
      } catch (err) {
        console.error('Failed to fetch data:', err)
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [])

  const filteredStages = stages.filter((stage) => {
    const matchesSearch =
      stage.name_zh.includes(searchQuery) ||
      stage.name_en.toLowerCase().includes(searchQuery.toLowerCase())
    // Note: category filtering removed as backend Stage model doesn't have category field
    return matchesSearch && (selectedCategory === 'all')
  })

  const displayStages = filteredStages.slice(0, 8)
  const otherStages = filteredStages.slice(8)

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
        <div className="flex items-baseline gap-3 mb-6">
          <h1 className="text-2xl font-bold text-gray-900">
            <span className="mr-2">🎭</span>
            角色扮演
          </h1>
          <p className="text-gray-500">沉浸式场景模拟 · AI智能推荐</p>
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
            <div className="flex gap-1.5 flex-shrink-0 flex-wrap">
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

          {/* Row 2: Continue Learning (show first stage as suggestion) */}
          {stages.length > 0 && (
            <Link
              to={`/stages/${stages[0].id}`}
              className="flex items-center gap-3 px-3 py-2 bg-gradient-to-r from-orange-50 to-amber-50 rounded-lg hover:from-orange-100 hover:to-amber-100 transition-colors"
            >
              <div className="text-2xl">{stages[0].icon_emoji || '📚'}</div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm text-gray-900">{stages[0].name_zh}</span>
                  <span className="text-xs text-gray-500">开始学习</span>
                </div>
              </div>
              <Button size="sm" className="h-7 text-xs px-3 flex-shrink-0">
                继续学习 <ChevronRight className="h-3 w-3 ml-0.5" />
              </Button>
            </Link>
          )}
        </div>

        {/* Today's Featured */}
        <div className="mb-6">
          <h2 className="font-semibold text-gray-900 mb-4">
            <span className="mr-2">🌟</span>
            今日精选
          </h2>
          {displayStages.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
              {displayStages.map((stage) => (
                <StageCard key={stage.id} stage={stage} />
              ))}
            </div>
          ) : (
            <div className="bg-white rounded-xl p-8 text-center text-gray-500">
              暂无场景数据
            </div>
          )}
        </div>

        {/* All Stages (if there are more) */}
        {otherStages.length > 0 && (
          <div className="mb-6">
            <h2 className="font-semibold text-gray-900 mb-4">
              <span className="mr-2">📚</span>
              更多场景
            </h2>
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
              {otherStages.map((stage) => (
                <StageCard key={stage.id} stage={stage} />
              ))}
            </div>
          </div>
        )}
      </main>
      <Footer />
    </div>
  )
}
