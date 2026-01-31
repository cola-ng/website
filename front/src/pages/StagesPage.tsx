import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { Play } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { cn } from '../lib/utils'
import { useAuth } from '../lib/auth'

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
  const { token } = useAuth()
  const [stages, setStages] = useState<Stage[]>([])
  const [loading, setLoading] = useState(true)

  // Fetch stage from API
  useEffect(() => {
    if (!token) {
      setLoading(false)
      return
    }

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
  }, [token])

  const displayStages = stages.slice(0, 8)
  const otherStages = stages.slice(8)

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

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">🎭</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">角色扮演</h1>
            <p className="text-gray-600 mb-6">沉浸式场景模拟，在真实情境中练习英语对话</p>
            <Button asChild>
              <a href="/login?redirectTo=/stages">登录开始练习</a>
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
        <div className="flex items-baseline gap-3 mb-6">
          <h1 className="text-2xl font-bold text-gray-900">
            <span className="mr-2">🎭</span>
            角色扮演
          </h1>
          <p className="text-gray-500">沉浸式场景模拟 · AI智能推荐</p>
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
