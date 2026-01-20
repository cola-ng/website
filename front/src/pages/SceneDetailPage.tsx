import { useState, useEffect } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import {
  ArrowLeft,
  Play,
  Pause,
  RotateCcw,
  Volume2,
  ChevronRight,
  CheckCircle,
  Clock,
  BookOpen,
  MessageSquare,
} from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'

interface Scene {
  id: number
  name_en: string
  name_zh: string
  description_en: string | null
  description_zh: string | null
  icon_emoji: string | null
  difficulty: string | null
  category: string | null
  display_order: number | null
  duration_minutes?: number
  is_featured?: boolean
}

interface Dialogue {
  id: number
  scene_id: number
  title_en: string
  title_zh: string
  description_en: string | null
  description_zh: string | null
  total_turns: number | null
  estimated_duration_seconds: number | null
  difficulty: string | null
}

interface DialogueTurn {
  id: number
  dialogue_id: number
  turn_number: number
  speaker_role: string
  speaker_name: string | null
  content_en: string
  content_zh: string
  audio_path: string | null
  notes: string | null
}

export function SceneDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { token } = useAuth()

  const [scene, setScene] = useState<Scene | null>(null)
  const [dialogues, setDialogues] = useState<Dialogue[]>([])
  const [selectedDialogue, setSelectedDialogue] = useState<Dialogue | null>(null)
  const [turns, setTurns] = useState<DialogueTurn[]>([])
  const [currentTurnIndex, setCurrentTurnIndex] = useState(0)
  const [isPlaying, setIsPlaying] = useState(false)
  const [showTranslation, setShowTranslation] = useState(true)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Fetch scene data
  useEffect(() => {
    async function fetchScene() {
      if (!id) return
      try {
        setLoading(true)
        const response = await fetch(`/api/asset/scenes/${id}`)
        if (!response.ok) throw new Error('Failed to fetch scene')
        const data = await response.json()
        setScene(data)
      } catch (err) {
        setError('Failed to load scene')
        console.error(err)
      } finally {
        setLoading(false)
      }
    }
    fetchScene()
  }, [id])

  // Fetch dialogues for this scene
  useEffect(() => {
    async function fetchDialogues() {
      if (!id) return
      try {
        const response = await fetch(`/api/asset/scenes/${id}/dialogues`)
        if (!response.ok) throw new Error('Failed to fetch dialogues')
        const data = await response.json()
        setDialogues(data)
        if (data.length > 0) {
          setSelectedDialogue(data[0])
        }
      } catch (err) {
        console.error('Failed to fetch dialogues:', err)
      }
    }
    fetchDialogues()
  }, [id])

  // Fetch turns for selected dialogue
  useEffect(() => {
    async function fetchTurns() {
      if (!selectedDialogue) return
      try {
        const response = await fetch(`/api/asset/dialogues/${selectedDialogue.id}/turns`)
        if (!response.ok) throw new Error('Failed to fetch turns')
        const data = await response.json()
        setTurns(data)
        setCurrentTurnIndex(0)
      } catch (err) {
        console.error('Failed to fetch turns:', err)
      }
    }
    fetchTurns()
  }, [selectedDialogue])

  const currentTurn = turns[currentTurnIndex]

  const handleNextTurn = () => {
    if (currentTurnIndex < turns.length - 1) {
      setCurrentTurnIndex(currentTurnIndex + 1)
    }
  }

  const handlePrevTurn = () => {
    if (currentTurnIndex > 0) {
      setCurrentTurnIndex(currentTurnIndex - 1)
    }
  }

  const handleRestart = () => {
    setCurrentTurnIndex(0)
  }

  const playAudio = () => {
    // In real implementation, this would play TTS audio
    console.log('Playing audio for:', currentTurn?.content_en)
  }

  const getDifficultyColor = (difficulty: string | null) => {
    switch (difficulty) {
      case 'beginner':
        return 'bg-green-100 text-green-700'
      case 'intermediate':
        return 'bg-amber-100 text-amber-700'
      case 'advanced':
        return 'bg-red-100 text-red-700'
      default:
        return 'bg-gray-100 text-gray-700'
    }
  }

  const getDifficultyLabel = (difficulty: string | null) => {
    switch (difficulty) {
      case 'beginner':
        return '初级'
      case 'intermediate':
        return '中级'
      case 'advanced':
        return '高级'
      default:
        return '未知'
    }
  }

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

  if (error || !scene) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <main className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">😕</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">场景未找到</h1>
            <p className="text-gray-600 mb-6">{error || '无法加载场景内容'}</p>
            <Button asChild>
              <Link to="/scenes">返回场景中心</Link>
            </Button>
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
        {/* Back button and scene info */}
        <div className="mb-6">
          <button
            onClick={() => navigate('/scenes')}
            className="flex items-center gap-2 text-gray-600 hover:text-gray-900 mb-4"
          >
            <ArrowLeft className="h-4 w-4" />
            <span>返回场景中心</span>
          </button>

          <div className="bg-white rounded-xl shadow-lg p-6">
            <div className="flex items-start gap-4">
              <div className="text-5xl">{scene.icon_emoji || '📚'}</div>
              <div className="flex-1">
                <h1 className="text-2xl font-bold text-gray-900 mb-1">{scene.name_zh}</h1>
                <p className="text-gray-500 mb-3">{scene.name_en}</p>
                <p className="text-gray-600 mb-4">{scene.description_zh || scene.description_en}</p>
                <div className="flex items-center gap-4 text-sm">
                  <span
                    className={cn(
                      'px-3 py-1 rounded-full font-medium',
                      getDifficultyColor(scene.difficulty)
                    )}
                  >
                    {getDifficultyLabel(scene.difficulty)}
                  </span>
                  <span className="flex items-center gap-1 text-gray-500">
                    <Clock className="h-4 w-4" />
                    约 {scene.duration_minutes || 5} 分钟
                  </span>
                  <span className="flex items-center gap-1 text-gray-500">
                    <MessageSquare className="h-4 w-4" />
                    {dialogues.length} 个对话
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Dialogue list */}
          <div className="lg:col-span-1">
            <div className="bg-white rounded-xl shadow-lg p-4">
              <h2 className="font-semibold text-gray-900 mb-4 flex items-center gap-2">
                <BookOpen className="h-5 w-5 text-orange-500" />
                对话列表
              </h2>
              <div className="space-y-2">
                {dialogues.map((dialogue) => (
                  <button
                    key={dialogue.id}
                    onClick={() => setSelectedDialogue(dialogue)}
                    className={cn(
                      'w-full text-left p-3 rounded-lg transition-colors',
                      selectedDialogue?.id === dialogue.id
                        ? 'bg-orange-100 border-2 border-orange-300'
                        : 'bg-gray-50 hover:bg-gray-100'
                    )}
                  >
                    <div className="font-medium text-gray-900">{dialogue.title_zh}</div>
                    <div className="text-sm text-gray-500">{dialogue.title_en}</div>
                    <div className="flex items-center gap-2 mt-1 text-xs text-gray-400">
                      <span>{dialogue.total_turns || 0} 轮对话</span>
                      {dialogue.estimated_duration_seconds && (
                        <span>• {Math.ceil(dialogue.estimated_duration_seconds / 60)} 分钟</span>
                      )}
                    </div>
                  </button>
                ))}
                {dialogues.length === 0 && (
                  <div className="text-center text-gray-500 py-8">
                    暂无对话内容
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Dialogue practice area */}
          <div className="lg:col-span-2">
            <div className="bg-white rounded-xl shadow-lg overflow-hidden">
              {/* Progress bar */}
              {turns.length > 0 && (
                <div className="bg-gray-100 px-6 py-3">
                  <div className="flex items-center justify-between text-sm text-gray-600 mb-2">
                    <span>进度</span>
                    <span>
                      {currentTurnIndex + 1} / {turns.length}
                    </span>
                  </div>
                  <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-orange-500 to-amber-500 transition-all duration-300"
                      style={{ width: `${((currentTurnIndex + 1) / turns.length) * 100}%` }}
                    />
                  </div>
                </div>
              )}

              {/* Current turn display */}
              <div className="p-6">
                {currentTurn ? (
                  <div className="space-y-6">
                    {/* Speaker info */}
                    <div className="flex items-center gap-3">
                      <div
                        className={cn(
                          'w-10 h-10 rounded-full flex items-center justify-center text-white font-bold',
                          currentTurn.speaker_role === 'user'
                            ? 'bg-blue-500'
                            : 'bg-orange-500'
                        )}
                      >
                        {currentTurn.speaker_role === 'user' ? '你' : 'AI'}
                      </div>
                      <div>
                        <div className="font-medium text-gray-900">
                          {currentTurn.speaker_role === 'user' ? '你的回复' : '对方说'}
                        </div>
                        <div className="text-sm text-gray-500">
                          第 {currentTurn.turn_number} 轮
                        </div>
                      </div>
                    </div>

                    {/* Content */}
                    <div className="bg-gray-50 rounded-xl p-6">
                      <p className="text-xl text-gray-900 mb-4 leading-relaxed">
                        {currentTurn.content_en}
                      </p>
                      {showTranslation && (
                        <p className="text-gray-600">{currentTurn.content_zh}</p>
                      )}
                    </div>

                    {/* Hints for user turns */}
                    {currentTurn.speaker_role === 'user' && currentTurn.notes && (
                      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                        <div className="text-sm font-medium text-blue-800 mb-1">提示</div>
                        <div className="text-blue-700">{currentTurn.notes}</div>
                      </div>
                    )}

                    {/* Action buttons */}
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <Button variant="outline" size="sm" onClick={playAudio}>
                          <Volume2 className="h-4 w-4 mr-1" />
                          播放
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setShowTranslation(!showTranslation)}
                        >
                          {showTranslation ? '隐藏翻译' : '显示翻译'}
                        </Button>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={handlePrevTurn}
                          disabled={currentTurnIndex === 0}
                        >
                          上一句
                        </Button>
                        <Button
                          size="sm"
                          onClick={handleNextTurn}
                          disabled={currentTurnIndex >= turns.length - 1}
                        >
                          下一句
                          <ChevronRight className="h-4 w-4 ml-1" />
                        </Button>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-center py-12">
                    <div className="text-6xl mb-4">📝</div>
                    <h3 className="text-lg font-medium text-gray-900 mb-2">
                      选择一个对话开始练习
                    </h3>
                    <p className="text-gray-500">
                      从左侧列表选择对话，开始你的场景练习
                    </p>
                  </div>
                )}
              </div>

              {/* Bottom controls */}
              {turns.length > 0 && (
                <div className="border-t px-6 py-4 bg-gray-50">
                  <div className="flex items-center justify-between">
                    <Button variant="outline" onClick={handleRestart}>
                      <RotateCcw className="h-4 w-4 mr-2" />
                      重新开始
                    </Button>
                    {currentTurnIndex >= turns.length - 1 && (
                      <div className="flex items-center gap-2 text-green-600">
                        <CheckCircle className="h-5 w-5" />
                        <span className="font-medium">对话完成！</span>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  )
}
