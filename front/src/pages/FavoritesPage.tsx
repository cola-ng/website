import { useState, useEffect, useCallback, useRef } from 'react'
import { Link } from 'react-router-dom'
import { Heart, Loader2, Trash2, BookOpen, MessageSquare, Volume2 } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../components/ui/tabs'
import { useAuth } from '../lib/auth'
import { useToast } from '../contexts/ToastContext'
import {
  listFavorites,
  removeFavorite,
  type Favorite,
  type FavoriteWord,
  type FavoriteSentence,
} from '../lib/api'

// Entity type configuration
const ENTITY_TYPES = [
  { key: 'word', label: '单词', icon: <BookOpen className="h-4 w-4" /> },
  { key: 'sentence', label: '句子', icon: <MessageSquare className="h-4 w-4" /> },
] as const

type EntityType = typeof ENTITY_TYPES[number]['key']

export function FavoritesPage() {
  const { token } = useAuth()
  const toast = useToast()

  const [activeTab, setActiveTab] = useState<EntityType>('word')
  const [favorites, setFavorites] = useState<Favorite[]>([])
  const [loading, setLoading] = useState(true)
  const [removingIds, setRemovingIds] = useState<Set<number>>(new Set())

  // Track current audio instance for global audio management
  const currentAudioRef = useRef<HTMLAudioElement | null>(null)
  const currentBlobUrlRef = useRef<string | null>(null)

  // Fetch favorites
  const fetchFavorites = useCallback(async () => {
    if (!token) return

    setLoading(true)
    try {
      const data = await listFavorites(token, activeTab, 100)
      setFavorites(data)
    } catch (err) {
      console.error('Failed to fetch favorites:', err)
      toast.error('加载失败', '无法加载收藏列表')
    } finally {
      setLoading(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, activeTab])

  useEffect(() => {
    fetchFavorites()
  }, [fetchFavorites])

  // Remove favorite
  const handleRemove = async (favorite: Favorite) => {
    if (!token) return

    setRemovingIds(prev => new Set([...prev, favorite.id]))
    try {
      await removeFavorite(token, favorite.entity, favorite.record_id)
      setFavorites(prev => prev.filter(f => f.id !== favorite.id))
      toast.success('已移除', '已从收藏夹中移除')
    } catch {
      toast.error('移除失败', '无法移除收藏')
    } finally {
      setRemovingIds(prev => {
        const next = new Set(prev)
        next.delete(favorite.id)
        return next
      })
    }
  }

  // Stop current audio and clean up
  const stopCurrentAudio = () => {
    if (currentAudioRef.current) {
      currentAudioRef.current.pause()
      currentAudioRef.current = null
    }
    if (currentBlobUrlRef.current) {
      URL.revokeObjectURL(currentBlobUrlRef.current)
      currentBlobUrlRef.current = null
    }
  }

  // Play pronunciation audio by ID
  const playPronunciation = async (pronunciationId: number) => {
    const url = `/api/dict/pronunciations/${pronunciationId}/audio`

    // Stop any currently playing audio
    stopCurrentAudio()

    try {
      const audio = new Audio(url)
      currentAudioRef.current = audio
      audio.onended = () => {
        stopCurrentAudio()
      }
      await audio.play()
    } catch (err) {
      console.error('Failed to play audio:', err)
      toast.error('播放失败', '无法播放音频')
    }
  }

  // Play sentence audio (requires auth)
  const playSentenceAudio = async (audioUrl: string) => {
    // Stop any currently playing audio
    stopCurrentAudio()

    try {
      const response = await fetch(audioUrl, {
        headers: { Authorization: `Bearer ${token}` },
      })
      if (!response.ok) throw new Error('Failed to fetch audio')
      const blob = await response.blob()
      const blobUrl = URL.createObjectURL(blob)
      currentBlobUrlRef.current = blobUrl
      const audio = new Audio(blobUrl)

      currentAudioRef.current = audio
      audio.onended = () => {
        stopCurrentAudio()
      }
      await audio.play()
    } catch (err) {
      console.error('Failed to play audio:', err)
      toast.error('播放失败', '无法播放音频')
    }
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">❤️</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">收藏夹</h1>
            <p className="text-gray-600 mb-6">登录后查看和管理您的收藏</p>
            <Button asChild>
              <Link to="/login?redirectTo=/favorites">登录</Link>
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
              <span className="mr-2">❤️</span>
              收藏夹
            </h1>
            <p className="text-gray-500">管理您收藏的内容</p>
          </div>

          <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as EntityType)}>
            <TabsList className="mb-4">
              {ENTITY_TYPES.map((type) => (
                <TabsTrigger key={type.key} value={type.key} className="gap-2">
                  {type.icon}
                  {type.label}
                </TabsTrigger>
              ))}
            </TabsList>

            {/* Word favorites */}
            <TabsContent value="word">
              {loading ? (
                <div className="py-12 text-center">
                  <Loader2 className="h-8 w-8 animate-spin text-orange-500 mx-auto mb-4" />
                  <p className="text-gray-500">加载中...</p>
                </div>
              ) : favorites.length === 0 ? (
                <div className="py-12 text-center">
                  <Heart className="h-12 w-12 text-gray-300 mx-auto mb-4" />
                  <p className="text-gray-500">还没有收藏任何单词</p>
                  <p className="text-sm text-gray-400 mt-2">
                    在词典中查询单词时，点击星标即可收藏
                  </p>
                  <Button asChild className="mt-4" variant="outline">
                    <Link to="/dict">去词典查询</Link>
                  </Button>
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {favorites
                    .filter((f): f is FavoriteWord => f.entity === 'word')
                    .map((favorite) => {
                      const isRemoving = removingIds.has(favorite.id)

                      return (
                        <div
                          key={favorite.id}
                          className="group relative bg-gray-50 rounded-lg p-4 hover:bg-orange-50 transition-colors border border-gray-100 hover:border-orange-200"
                        >
                          <div className="flex items-start justify-between">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2 flex-wrap">
                                <Link
                                  to={`/dict/${encodeURIComponent(favorite.word)}`}
                                  className="font-medium text-gray-900 hover:text-orange-600 transition-colors"
                                >
                                  {favorite.word}
                                </Link>
                                {favorite.pronunciations.map((pron) => (
                                  <span key={pron.id} className="inline-flex items-center gap-1">
                                    <span className="text-sm text-gray-500 font-mono">
                                      /{pron.ipa}/
                                    </span>
                                    <button
                                      onClick={() => playPronunciation(pron.id)}
                                      className="p-1 text-gray-400 hover:text-orange-500 hover:bg-orange-100 rounded transition-colors"
                                      title={pron.dialect ? `播放发音 (${pron.dialect})` : '播放发音'}
                                    >
                                      <Volume2 className="h-4 w-4" />
                                    </button>
                                  </span>
                                ))}
                              </div>
                              {favorite.word_zh && (
                                <p className="text-sm text-gray-600 mt-1 line-clamp-2">
                                  {favorite.word_zh}
                                </p>
                              )}
                              <div className="text-xs text-gray-400 mt-2">
                                收藏于 {new Date(favorite.created_at).toLocaleDateString('zh-CN')}
                              </div>
                            </div>
                            <button
                              onClick={() => handleRemove(favorite)}
                              disabled={isRemoving}
                              className="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded transition-colors opacity-0 group-hover:opacity-100 flex-shrink-0"
                              title="移除收藏"
                            >
                              {isRemoving ? (
                                <Loader2 className="h-4 w-4 animate-spin" />
                              ) : (
                                <Trash2 className="h-4 w-4" />
                              )}
                            </button>
                          </div>
                        </div>
                      )
                    })}
                </div>
              )}
            </TabsContent>

            {/* Sentence favorites */}
            <TabsContent value="sentence">
              {loading ? (
                <div className="py-12 text-center">
                  <Loader2 className="h-8 w-8 animate-spin text-orange-500 mx-auto mb-4" />
                  <p className="text-gray-500">加载中...</p>
                </div>
              ) : favorites.length === 0 ? (
                <div className="py-12 text-center">
                  <Heart className="h-12 w-12 text-gray-300 mx-auto mb-4" />
                  <p className="text-gray-500">还没有收藏任何句子</p>
                  <p className="text-sm text-gray-400 mt-2">
                    在学习过程中遇到喜欢的句子时可以收藏
                  </p>
                </div>
              ) : (
                <div className="space-y-3">
                  {favorites
                    .filter((f): f is FavoriteSentence => f.entity === 'sentence')
                    .map((favorite) => {
                      const isRemoving = removingIds.has(favorite.id)

                      return (
                        <div
                          key={favorite.id}
                          className="group relative bg-gray-50 rounded-lg p-4 hover:bg-orange-50 transition-colors border border-gray-100 hover:border-orange-200"
                        >
                          <div className="flex items-start justify-between">
                            <div className="flex-1 min-w-0">
                              <p className="text-gray-900">{favorite.sentence_en}</p>
                              {favorite.sentence_zh && (
                                <p className="text-sm text-gray-600 mt-1">{favorite.sentence_zh}</p>
                              )}
                              <div className="text-xs text-gray-400 mt-2">
                                收藏于 {new Date(favorite.created_at).toLocaleDateString('zh-CN')}
                              </div>
                            </div>
                            <div className="flex items-center gap-1 flex-shrink-0">
                              {favorite.audio_url && (
                                <button
                                  onClick={() => playSentenceAudio(favorite.audio_url!)}
                                  className="p-1.5 text-gray-400 hover:text-orange-500 hover:bg-orange-50 rounded transition-colors opacity-0 group-hover:opacity-100"
                                  title="播放音频"
                                >
                                  <Volume2 className="h-4 w-4" />
                                </button>
                              )}
                              <button
                                onClick={() => handleRemove(favorite)}
                                disabled={isRemoving}
                                className="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded transition-colors opacity-0 group-hover:opacity-100"
                                title="移除收藏"
                              >
                                {isRemoving ? (
                                  <Loader2 className="h-4 w-4 animate-spin" />
                                ) : (
                                  <Trash2 className="h-4 w-4" />
                                )}
                              </button>
                            </div>
                          </div>
                        </div>
                      )
                    })}
                </div>
              )}
            </TabsContent>
          </Tabs>
        </div>
      </main>
      <Footer />
    </div>
  )
}
