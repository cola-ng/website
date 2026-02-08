import { useState, useEffect, useCallback, useRef } from 'react'
import { useParams, useNavigate, useLocation, Link } from 'react-router-dom'
import { Search, Volume2, Star, Languages, BookOpen, Loader2, Copy, Check, History, Trash2, X, ChevronDown } from 'lucide-react'
import {
  lookup,
  translate,
  getSearchedWords,
  saveSearchedWord,
  clearSearchedWords,
  deleteSearchedWord,
  getLocalSearchHistory,
  saveLocalSearchHistory,
  deleteLocalSearchHistory,
  clearLocalSearchHistory,
  syncLocalHistoryToServer,
  toggleFavorite,
  checkFavorites,
  type WordQueryResponse,
  type TranslateOptions,
  type TranslateTone,
  type Definition,
} from '../lib/api'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../components/ui/tabs'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { useAuth } from '../lib/auth'
import { useToast } from '../contexts/ToastContext'
import { useStudyHeartbeat, type PageType } from '../hooks/useStudyHeartbeat'

// Unified search history item type
type SearchHistoryItem = {
  id?: number
  word: string
  searched_at: string
}

type SectionNavRequest = {
  sectionId: string
  tabValue?: string
  nonce: number
}

export function DictPage() {
  const { word: urlWord } = useParams<{ word: string }>()
  const navigate = useNavigate()
  const location = useLocation()
  const { token } = useAuth()
  const toast = useToast()

  // Determine active tab from URL
  const activeTab = location.pathname === '/translate' ? 'translate' : 'lookup'

  // Track study time based on current mode
  const heartbeatType: PageType = activeTab === 'translate' ? 'translate' : 'dictionary'
  useStudyHeartbeat(heartbeatType)

  const handleTabChange = (value: string) => {
    if (value === 'translate') {
      navigate('/translate')
    } else {
      navigate('/dict')
    }
  }

  // Word lookup state
  const [query, setQuery] = useState('')
  const [result, setResult] = useState<WordQueryResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [showMoreSentences, setShowMoreSentences] = useState(false)
  const [activeNavKey, setActiveNavKey] = useState('')
  const [sectionNavRequest, setSectionNavRequest] = useState<SectionNavRequest | null>(null)

  // Translation state
  const [sourceText, setSourceText] = useState('')
  const [translatedText, setTranslatedText] = useState('')
  const [translating, setTranslating] = useState(false)
  const [translateError, setTranslateError] = useState('')
  const [detectedLang, setDetectedLang] = useState<'en' | 'zh' | null>(null)
  const [copied, setCopied] = useState(false)

  // Translation options state
  const [translateOptions, setTranslateOptions] = useState<TranslateOptions>({
    auto_correct: false,
    tone: 'original',
    preserve_formatting: true,
  })

  // Search history state
  const [searchHistory, setSearchHistory] = useState<SearchHistoryItem[]>([])
  const [_loadingHistory, setLoadingHistory] = useState(false)
  const [isFavorite, setIsFavorite] = useState(false)
  const [togglingFavorite, setTogglingFavorite] = useState(false)
  const [favoriteWordIds, setFavoriteWordIds] = useState<Set<number>>(new Set())
  // Map word string (lowercase) to word ID for tracking favorites
  const [wordIdMap, setWordIdMap] = useState<Map<string, number>>(new Map())
  const [togglingHistoryWord, setTogglingHistoryWord] = useState<string | null>(null)
  const syncedRef = useRef(false)

  // Fetch search history on mount and when auth changes
  useEffect(() => {
    const fetchHistory = async () => {
      setLoadingHistory(true)
      try {
        if (token) {
          // User is logged in - sync local history first (only once)
          if (!syncedRef.current) {
            syncedRef.current = true
            await syncLocalHistoryToServer(token)
          }
          // Then fetch from server
          const words = await getSearchedWords(20, token)
          setSearchHistory(words.map(w => ({
            id: w.id,
            word: w.word,
            searched_at: w.searched_at,
          })))
        } else {
          // User not logged in - use local storage
          const local = getLocalSearchHistory()
          setSearchHistory(local.map(w => ({
            word: w.word,
            searched_at: w.searched_at,
          })))
        }
      } catch {
        // Silently fail - history is optional
        // Fall back to local storage
        const local = getLocalSearchHistory()
        setSearchHistory(local.map(w => ({
          word: w.word,
          searched_at: w.searched_at,
        })))
      } finally {
        setLoadingHistory(false)
      }
    }
    fetchHistory()
  }, [token])

  const searchWord = useCallback(async (word: string, skipHistory = false) => {
    if (!word.trim()) return

    setLoading(true)
    setError('')
    setShowMoreSentences(false)
    setIsFavorite(false)
    try {
      const data = await lookup(word.trim())
      setResult(data)
      setQuery(word.trim())
      // Set active group to first available
      const navGroups = getNavGroups(data)
      if (navGroups.length > 0) {
        const navLinks = getNavLinks(data)
        if (navLinks.length > 0) {
          setActiveNavKey(navLinks[0].key)
        }
      }

      // Store the word ID mapping for this word
      const wordLower = data.word.word.toLowerCase()
      setWordIdMap(prev => new Map(prev).set(wordLower, data.word.id))

      // Check if this word is in favorites
      if (token && data.word.id) {
        try {
          const favoritedIds = await checkFavorites(token, 'word', [data.word.id])
          const isFav = favoritedIds.includes(data.word.id)
          setIsFavorite(isFav)
          if (isFav) {
            setFavoriteWordIds(prev => new Set([...prev, data.word.id]))
          }
        } catch {
          // Silently fail
        }
      }

      // Save to search history (skip if clicking from history)
      if (!skipHistory) {
        const trimmedWord = word.trim()

        if (token) {
          // User is logged in - save to server
          try {
            const saved = await saveSearchedWord(trimmedWord, token)
            setSearchHistory(prev => {
              const filtered = prev.filter(w => w.word.toLowerCase() !== trimmedWord.toLowerCase())
              return [{ id: saved.id, word: saved.word, searched_at: saved.searched_at }, ...filtered].slice(0, 20)
            })
          } catch {
            // Silently fail
          }
        } else {
          // User not logged in - save to local storage
          const updated = saveLocalSearchHistory(trimmedWord)
          setSearchHistory(updated.map(w => ({
            word: w.word,
            searched_at: w.searched_at,
          })))
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch word')
      setResult(null)
    } finally {
      setLoading(false)
    }
  }, [token])

  const searchWordRef = useRef(searchWord)
  searchWordRef.current = searchWord
  const lastLookedUp = useRef<string | null>(null)

  useEffect(() => {
    if (!urlWord) return
    const decoded = decodeURIComponent(urlWord)
    // Prevent duplicate lookups for the same word (e.g. StrictMode double-mount)
    if (lastLookedUp.current === decoded) return
    lastLookedUp.current = decoded

    const fromHistory = (location.state as { fromHistory?: boolean })?.fromHistory === true
    searchWordRef.current(decoded, fromHistory)
    if (fromHistory) {
      window.history.replaceState({}, '')
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [urlWord])

  const handleClearHistory = async () => {
    if (token) {
      try {
        await clearSearchedWords(token)
        setSearchHistory([])
      } catch {
        toast.error('清除失败', '无法清除搜索历史')
      }
    } else {
      clearLocalSearchHistory()
      setSearchHistory([])
    }
  }

  const handleDeleteHistoryItem = async (item: SearchHistoryItem) => {
    if (token && item.id) {
      try {
        await deleteSearchedWord(item.id, token)
        setSearchHistory(prev => prev.filter(h => h.id !== item.id))
      } catch {
        toast.error('删除失败', '无法删除历史记录')
      }
    } else {
      const updated = deleteLocalSearchHistory(item.word)
      setSearchHistory(updated.map(w => ({
        word: w.word,
        searched_at: w.searched_at,
      })))
    }
  }

  const handleToggleFavorite = async () => {
    if (!token) {
      toast.error('请先登录', '收藏功能需要登录后使用')
      return
    }
    if (!result) return

    setTogglingFavorite(true)
    try {
      const res = await toggleFavorite(token, 'word', result.word.id)
      setIsFavorite(res.is_favorited)
      // Update the favoriteWordIds set
      if (res.is_favorited) {
        setFavoriteWordIds(prev => new Set([...prev, result.word.id]))
      } else {
        setFavoriteWordIds(prev => {
          const next = new Set(prev)
          next.delete(result.word.id)
          return next
        })
      }
      toast.success(res.is_favorited ? '已收藏' : '已取消收藏', res.is_favorited ? '单词已添加到收藏夹' : '单词已从收藏夹移除')
    } catch {
      toast.error('操作失败', '无法更新收藏状态')
    } finally {
      setTogglingFavorite(false)
    }
  }

  // Toggle favorite for a word in the history list
  const handleToggleHistoryFavorite = async (word: string) => {
    if (!token) {
      toast.error('请先登录', '收藏功能需要登录后使用')
      return
    }

    setTogglingHistoryWord(word)
    try {
      // Look up the word to get its ID
      const wordData = await lookup(word)
      const wordId = wordData.word.id
      const wordLower = word.toLowerCase()

      // Store the word ID mapping
      setWordIdMap(prev => new Map(prev).set(wordLower, wordId))

      const res = await toggleFavorite(token, 'word', wordId)
      if (res.is_favorited) {
        setFavoriteWordIds(prev => new Set([...prev, wordId]))
      } else {
        setFavoriteWordIds(prev => {
          const next = new Set(prev)
          next.delete(wordId)
          return next
        })
      }
      // If this is also the current result word, update isFavorite
      if (result && result.word.id === wordId) {
        setIsFavorite(res.is_favorited)
      }
    } catch {
      toast.error('操作失败', '无法更新收藏状态')
    } finally {
      setTogglingHistoryWord(null)
    }
  }

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!query.trim()) return
    navigate(`/dict/${encodeURIComponent(query.trim())}`)
  }

  const playAudio = (pronunciationId: number) => {
    const url = `/api/dict/pronunciations/${pronunciationId}/audio`
    new Audio(url).play()
  }

  // Translation handlers
  const handleTranslate = async () => {
    if (!sourceText.trim()) {
      setTranslateError('请输入要翻译的文本')
      return
    }

    setTranslating(true)
    setTranslateError('')
    setTranslatedText('')
    setDetectedLang(null)

    // Auto-detect language: if contains Chinese characters, translate to English; otherwise translate to Chinese
    const hasChinese = /[\u4e00-\u9fff\u3400-\u4dbf]/.test(sourceText)
    const targetLang = hasChinese ? 'en' : 'zh'

    try {
      const result = await translate(
        {
          text: sourceText.trim(),
          source: 'auto',
          target: targetLang,
          options: translateOptions,
        },
        token || undefined
      )
      setTranslatedText(result.translated)
      setDetectedLang(hasChinese ? 'zh' : 'en')
    } catch (err) {
      const message = err instanceof Error ? err.message : '翻译失败'
      setTranslateError(message)
      toast.error('翻译失败', message)
    } finally {
      setTranslating(false)
    }
  }

  const copyToClipboard = async () => {
    if (!translatedText) return
    try {
      await navigator.clipboard.writeText(translatedText)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast.error('复制失败', '无法复制到剪贴板')
    }
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />
      <div className="container mx-auto px-4 py-4 max-w-6xl">
        <div className="text-center mb-4">
          <h1 className="text-2xl font-bold text-orange-900 mb-1">
            <span className="mr-2">📖</span>
            词典翻译
          </h1>
        </div>

        {/* Main tabs for Dictionary and Translation */}
        <Tabs value={activeTab} onValueChange={handleTabChange} className="mb-6">
          <TabsList className="grid w-full max-w-md mx-auto grid-cols-2 mb-4 h-10 gap-0 overflow-hidden rounded-xl border border-orange-200 bg-white shadow-sm p-0">
            <TabsTrigger
              value="lookup"
              className="gap-2 h-full rounded-none border-0 data-[state=active]:bg-orange-500 data-[state=active]:text-white data-[state=inactive]:text-orange-700"
            >
              <BookOpen className="w-4 h-4" />
              词典查询
            </TabsTrigger>
            <TabsTrigger
              value="translate"
              className="gap-2 h-full rounded-none border-0 data-[state=active]:bg-orange-500 data-[state=active]:text-white data-[state=inactive]:text-orange-700"
            >
              <Languages className="w-4 h-4" />
              内容翻译
            </TabsTrigger>
          </TabsList>

          {/* Word Lookup Tab */}
          <TabsContent value="lookup" className="mt-4">
            {/* Search form - matches content width */}
            <div className="grid grid-cols-1 lg:grid-cols-[200px_1fr_200px] gap-6 mb-4">
              <div className="hidden lg:block" />
              <form onSubmit={handleSearch}>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    placeholder="输入英/汉字词..."
                    className="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                  />
                  <button
                    type="submit"
                    disabled={loading}
                    className="px-4 py-2 text-sm bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:bg-orange-400 flex items-center gap-2 transition-colors"
                  >
                    <Search className="w-4 h-4" />
                    {loading ? '查询中...' : '查询'}
                  </button>
                </div>
              </form>
              <div className="hidden lg:block" />
            </div>

            {error && (
              <div className="grid grid-cols-1 lg:grid-cols-[200px_1fr_200px] gap-6 mb-6">
                <div className="hidden lg:block" />
                <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
                  {error}
                </div>
                <div className="hidden lg:block" />
              </div>
            )}

            {/* Three-column layout: sidebars align with word result */}
            {result && (
              <div className="grid grid-cols-1 lg:grid-cols-[200px_1fr_200px] gap-6">
                {/* Left column - Navigation */}
                <div className="hidden lg:block">
                  <div className="sticky top-20">
                    <SectionNav
                      result={result}
                      activeNavKey={activeNavKey}
                      onNavKeyChange={setActiveNavKey}
                      onNavigateTab={(sectionId, tabValue) =>
                        setSectionNavRequest({ sectionId, tabValue, nonce: Date.now() })
                      }
                    />
                  </div>
                </div>

                {/* Center column - Word content */}
                <div className="min-w-0">
                  <WordResult
                    result={result}
                    showMoreSentences={showMoreSentences}
                    setShowMoreSentences={setShowMoreSentences}
                    playAudio={playAudio}
                    isFavorite={isFavorite}
                    togglingFavorite={togglingFavorite}
                    onToggleFavorite={handleToggleFavorite}
                    sectionNavRequest={sectionNavRequest}
                  />
                </div>

                {/* Right column - Search History */}
                <div className="hidden lg:block">
                  <div className="sticky top-20">
                    {searchHistory.length > 0 && (
                      <div className="bg-white rounded-xl shadow-lg p-4">
                        <div className="flex items-center justify-between mb-3">
                          <div className="flex items-center gap-2">
                            <History className="w-4 h-4 text-gray-500" />
                            <span className="text-sm font-medium text-gray-700">搜索历史</span>
                          </div>
                          <button
                            onClick={handleClearHistory}
                            className="text-gray-400 hover:text-red-500 transition-colors"
                            title="清除历史"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </div>
                        <div className="space-y-1 max-h-96 overflow-y-auto">
                          {searchHistory.map((item, index) => {
                            // Check if this history item is the current result
                            const isCurrentWord = result && item.word.toLowerCase() === result.word.word.toLowerCase()
                            // Check if this word is favorited using wordIdMap
                            const wordId = wordIdMap.get(item.word.toLowerCase())
                            const isInFavorites = wordId !== undefined && favoriteWordIds.has(wordId)
                            const isToggling = togglingHistoryWord === item.word
                            return (
                              <div
                                key={item.id ?? `local-${index}`}
                                className={`group flex items-center justify-between px-3 py-2 text-sm rounded-lg transition-colors ${
                                  isCurrentWord
                                    ? 'bg-orange-100 text-orange-800 font-medium'
                                    : 'text-gray-700 hover:bg-gray-100'
                                }`}
                              >
                                <button
                                  onClick={(e) => {
                                    e.preventDefault()
                                    e.stopPropagation()
                                    handleToggleHistoryFavorite(item.word)
                                  }}
                                  disabled={isToggling}
                                  className={`flex-shrink-0 transition-colors ${
                                    isInFavorites
                                      ? 'text-yellow-500 hover:text-yellow-600'
                                      : 'text-gray-300 hover:text-yellow-500'
                                  }`}
                                  title={isInFavorites ? '取消收藏' : '添加到收藏夹'}
                                >
                                  {isToggling ? (
                                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                                  ) : (
                                    <Star className={`w-3.5 h-3.5 ${isInFavorites ? 'fill-current' : ''}`} />
                                  )}
                                </button>
                                <button
                                  onClick={() => {
                                    navigate(`/dict/${encodeURIComponent(item.word)}`, { state: { fromHistory: true } })
                                  }}
                                  className="flex-1 truncate text-left ml-1.5"
                                >
                                  {item.word}
                                </button>
                                <button
                                  onClick={(e) => {
                                    e.preventDefault()
                                    e.stopPropagation()
                                    handleDeleteHistoryItem(item)
                                  }}
                                  className="opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-500 transition-all ml-2"
                                  title="删除"
                                >
                                  <X className="w-3.5 h-3.5" />
                                </button>
                              </div>
                            )
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}
          </TabsContent>

          {/* Translation Tab */}
          <TabsContent value="translate" className="mt-4">
            <div className="grid grid-cols-1 lg:grid-cols-[1fr_280px] gap-6">
              {/* Left column - Translation input/output */}
              <div className="space-y-4">
                {/* Source text card */}
                <div className="bg-white rounded-xl shadow-lg p-5">
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-gray-700">输入文本</span>
                      <span className="text-xs text-gray-400 bg-gray-100 px-2 py-0.5 rounded">自动检测语言</span>
                    </div>
                    <span className="text-xs text-gray-400">
                      {sourceText.length}/5000
                    </span>
                  </div>
                  <textarea
                    value={sourceText}
                    onChange={(e) => setSourceText(e.target.value.slice(0, 5000))}
                    placeholder="输入中文或英文，自动检测并翻译..."
                    className="w-full h-32 p-4 text-base text-gray-800 bg-gray-50 rounded-lg border border-gray-200 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent resize-none"
                  />
                  <div className="flex items-center justify-between mt-3">
                    <p className="text-xs text-gray-400">
                      支持中英文互译，AI 自动识别语言
                    </p>
                    <button
                      onClick={handleTranslate}
                      disabled={translating || !sourceText.trim()}
                      className="px-6 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:bg-orange-400 flex items-center gap-2 transition-colors font-medium text-sm"
                    >
                      {translating ? (
                        <>
                          <Loader2 className="w-4 h-4 animate-spin" />
                          翻译中...
                        </>
                      ) : (
                        <>
                          <Languages className="w-4 h-4" />
                          翻译
                        </>
                      )}
                    </button>
                  </div>
                </div>

                {/* Translated text card */}
                <div className="bg-white rounded-xl shadow-lg p-5">
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-gray-700">翻译结果</span>
                      {detectedLang && (
                        <span className="text-xs text-orange-600 bg-orange-50 px-2 py-0.5 rounded">
                          {detectedLang === 'zh' ? '中文 → 英文' : '英文 → 中文'}
                        </span>
                      )}
                    </div>
                    {translatedText && (
                      <button
                        onClick={copyToClipboard}
                        className="flex items-center gap-1 text-xs text-orange-600 hover:text-orange-700 px-2 py-1 rounded hover:bg-orange-50 transition-colors"
                      >
                        {copied ? (
                          <>
                            <Check className="w-3.5 h-3.5" />
                            已复制
                          </>
                        ) : (
                          <>
                            <Copy className="w-3.5 h-3.5" />
                            复制结果
                          </>
                        )}
                      </button>
                    )}
                  </div>
                  <div className="w-full min-h-32 p-4 bg-gray-50 rounded-lg border border-gray-200">
                    {translating ? (
                      <div className="flex items-center justify-center h-32 text-gray-400">
                        <Loader2 className="w-6 h-6 animate-spin mr-2" />
                        正在翻译...
                      </div>
                    ) : translatedText ? (
                      <p className="text-base text-gray-800 whitespace-pre-wrap leading-relaxed">{translatedText}</p>
                    ) : (
                      <p className="text-gray-400 italic h-32 flex items-center justify-center">翻译结果将显示在这里</p>
                    )}
                  </div>
                  {translateError && (
                    <p className="mt-2 text-sm text-red-600">{translateError}</p>
                  )}
                </div>

                {/* Tips */}
                <div className="p-4 bg-amber-50 border border-amber-200 rounded-lg">
                  <p className="text-sm text-amber-800">
                    <strong>提示：</strong>翻译功能使用 AI 模型，自动检测输入语言并翻译为对应语言。如果您在设置中配置了自己的 AI 提供商，将使用您的配置进行翻译。
                  </p>
                </div>
              </div>

              {/* Right column - Translation options panel */}
              <div className="hidden lg:block">
                <div className="bg-white rounded-xl shadow-lg p-4 sticky top-4">
                  {/* Tone selection */}
                  <div className="mb-4">
                    <label className="block text-xs font-medium text-gray-600 mb-2">语言风格</label>
                    <div className="space-y-1.5">
                      {([
                        { value: 'original', label: '保持原样', desc: '忠实原文风格' },
                        { value: 'casual', label: '日常口语', desc: '轻松自然' },
                        { value: 'business', label: '商务职场', desc: '正式专业' },
                        { value: 'academic', label: '学术正式', desc: '严谨规范' },
                        { value: 'literary', label: '文学优美', desc: '富有文采' },
                        { value: 'simple', label: '简单易懂', desc: '通俗直白' },
                      ] as { value: TranslateTone; label: string; desc: string }[]).map((option) => (
                        <label
                          key={option.value}
                          className={`flex items-center justify-between p-2 rounded-lg cursor-pointer transition-colors ${
                            translateOptions.tone === option.value
                              ? 'bg-orange-50 border border-orange-200'
                              : 'hover:bg-gray-50 border border-transparent'
                          }`}
                        >
                          <div className="flex items-center gap-2">
                            <input
                              type="radio"
                              name="tone"
                              value={option.value}
                              checked={translateOptions.tone === option.value}
                              onChange={(e) => setTranslateOptions(prev => ({ ...prev, tone: e.target.value as TranslateTone }))}
                              className="w-3.5 h-3.5 text-orange-600 focus:ring-orange-500"
                            />
                            <span className="text-sm text-gray-700">{option.label}</span>
                          </div>
                          <span className="text-xs text-gray-400">{option.desc}</span>
                        </label>
                      ))}
                    </div>
                  </div>

                  {/* Format options */}
                  <div className="mb-4 pt-3 border-t border-gray-100">
                    <label className="block text-xs font-medium text-gray-600 mb-2">格式处理</label>
                    <div className="space-y-1.5">
                      <label
                        className={`flex items-center justify-between p-2 rounded-lg cursor-pointer transition-colors ${
                          translateOptions.preserve_formatting
                            ? 'bg-orange-50 border border-orange-200'
                            : 'hover:bg-gray-50 border border-transparent'
                        }`}
                      >
                        <div className="flex items-center gap-2">
                          <input
                            type="radio"
                            name="formatting"
                            checked={translateOptions.preserve_formatting === true}
                            onChange={() => setTranslateOptions(prev => ({ ...prev, preserve_formatting: true }))}
                            className="w-3.5 h-3.5 text-orange-600 focus:ring-orange-500"
                          />
                          <span className="text-sm text-gray-700">保留格式</span>
                        </div>
                        <span className="text-xs text-gray-400">保持原有结构</span>
                      </label>
                      <label
                        className={`flex items-center justify-between p-2 rounded-lg cursor-pointer transition-colors ${
                          !translateOptions.preserve_formatting
                            ? 'bg-orange-50 border border-orange-200'
                            : 'hover:bg-gray-50 border border-transparent'
                        }`}
                      >
                        <div className="flex items-center gap-2">
                          <input
                            type="radio"
                            name="formatting"
                            checked={translateOptions.preserve_formatting === false}
                            onChange={() => setTranslateOptions(prev => ({ ...prev, preserve_formatting: false }))}
                            className="w-3.5 h-3.5 text-orange-600 focus:ring-orange-500"
                          />
                          <span className="text-sm text-gray-700">重新排版</span>
                        </div>
                        <span className="text-xs text-gray-400">优化可读性</span>
                      </label>
                    </div>
                  </div>

                  {/* Auto-correct option */}
                  <div className="pt-3 border-t border-gray-100">
                    <label className="flex items-center justify-between cursor-pointer group">
                      <div>
                        <span className="text-sm text-gray-700">自动纠错</span>
                        <p className="text-xs text-gray-400">修正原文语法拼写错误</p>
                      </div>
                      <input
                        type="checkbox"
                        checked={translateOptions.auto_correct}
                        onChange={(e) => setTranslateOptions(prev => ({ ...prev, auto_correct: e.target.checked }))}
                        className="w-4 h-4 text-orange-600 rounded focus:ring-orange-500"
                      />
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </div>
      <Footer />
    </div>
  )
}

const FORM_TYPE_LABELS: Record<string, string> = {
  plural: '复数',
  singular: '单数',
  past: '过去式',
  present: '第三人称单数',
  future: '将来时',
  present_participle: '现在分词',
  past_participle: '过去分词',
  comparative: '比较级',
  superlative: '最高级',
  adverbial: '副词形式',
  nominalization: '名词形式',
  other: '其他',
}

function formTypeLabel(type: string | null): string {
  return (type && FORM_TYPE_LABELS[type]) || type || '其他'
}

// Group definitions by part_of_speech, preserving order of first appearance
const colorMap = {
  orange: { border: 'border-orange-500', badgeBg: 'bg-orange-200', badgeText: 'text-orange-800' },
  emerald: { border: 'border-emerald-500', badgeBg: 'bg-emerald-200', badgeText: 'text-emerald-800' },
  blue: { border: 'border-blue-500', badgeBg: 'bg-blue-200', badgeText: 'text-blue-800' },
} as const

function groupByPos(defs: Definition[]) {
  const groups: { pos: string | null; items: Definition[] }[] = []
  for (const def of defs) {
    const last = groups[groups.length - 1]
    if (last && last.pos === (def.part_of_speech ?? null)) {
      last.items.push(def)
    } else {
      groups.push({ pos: def.part_of_speech ?? null, items: [def] })
    }
  }
  return groups
}

function DefList({ groups, color }: { groups: ReturnType<typeof groupByPos>, color: keyof typeof colorMap }) {
  const c = colorMap[color]
  return (
    <div className="space-y-3">
      {groups.map((group, gi) => (
        <div key={gi} className={`pl-3 border-l-2 ${c.border}`}>
          {group.pos && (
            <span className={`inline-block px-2 py-0.5 ${c.badgeBg} ${c.badgeText} rounded text-xs font-medium mb-1`}>
              {group.pos}
            </span>
          )}
          <ol className={`${group.items.length > 1 ? 'list-decimal list-inside' : ''} space-y-0.5`}>
            {group.items.map((def) => (
              <li key={def.id} className="text-base text-gray-800">
                {def.definition}
                {def.register && (
                  <span className="text-xs text-gray-400 ml-1.5">{def.register}</span>
                )}
                {def.region && (
                  <span className="text-xs text-gray-400 ml-1.5">{def.region}</span>
                )}
                {def.usage_notes && (
                  <p className="text-xs text-gray-500 mt-0.5 ml-4 italic">{def.usage_notes}</p>
                )}
              </li>
            ))}
          </ol>
        </div>
      ))}
    </div>
  )
}

function DefinitionGroups({ defs, color }: { defs: Definition[], color: keyof typeof colorMap }) {
  const [showDetail, setShowDetail] = useState(false)

  const summaryDefs = defs.filter(d => d.is_summary)
  const detailDefs = defs.filter(d => !d.is_summary)
  const hasSummary = summaryDefs.length > 0
  const hasDetail = detailDefs.length > 0

  // No summary/detail split — show everything flat
  if (!hasSummary || !hasDetail) {
    return <DefList groups={groupByPos(defs)} color={color} />
  }

  const summaryGroups = groupByPos(summaryDefs)
  const detailGroups = groupByPos(detailDefs)

  return (
    <div className="space-y-3">
      {/* Summary definitions — always visible */}
      <DefList groups={summaryGroups} color={color} />

      {/* Expand toggle */}
      <button
        onClick={() => setShowDetail(v => !v)}
        className="group flex items-center gap-1.5 text-xs text-gray-400 hover:text-orange-600 transition-colors py-1"
      >
        <span className="h-px flex-1 max-w-12 bg-gray-200 group-hover:bg-orange-300 transition-colors" />
        <ChevronDown className={`w-3.5 h-3.5 transition-transform duration-200 ${showDetail ? 'rotate-180' : ''}`} />
        <span>{showDetail ? '收起详细释义' : `展开详细释义 (${detailDefs.length})`}</span>
        <span className="h-px flex-1 max-w-12 bg-gray-200 group-hover:bg-orange-300 transition-colors" />
      </button>

      {/* Detail definitions — collapsible */}
      <div className={`grid transition-all duration-300 ease-in-out ${showDetail ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}>
        <div className="overflow-hidden">
          <DefList groups={detailGroups} color={color} />
        </div>
      </div>
    </div>
  )
}

// Navigation group type for left sidebar
type NavGroup = {
  id: string
  label: string
  anchorId: string
  available: boolean
}

type NavLink = {
  key: string
  sectionId: string
  label: string
  anchorId: string
  tabValue?: string
  isChild?: boolean
}

// Helper function to compute nav groups
function getNavGroups(result: WordQueryResponse): NavGroup[] {
  const zhDefs = (result.definitions ?? []).filter(d => d.language === 'zh')
  const enDefs = (result.definitions ?? []).filter(d => d.language === 'en')
  const dualDefs = (result.definitions ?? []).filter(d => d.language === 'dual')
  const hasDefinitions = zhDefs.length > 0 || enDefs.length > 0 || dualDefs.length > 0

  const hasSentences = (result.sentences?.length ?? 0) > 0

  const relations = result.relations ?? []
  const synonyms = relations.filter(r => r.relation_type === 'synonym' || r.relation_type === 'similar')
  const antonyms = relations.filter(r => r.relation_type === 'antonym')
  const relatedWords = relations.filter(r =>
    r.relation_type !== 'synonym' && r.relation_type !== 'similar' && r.relation_type !== 'antonym'
  )
  const hasRelated = synonyms.length > 0 || antonyms.length > 0 || relatedWords.length > 0

  const hasEtymologies = (result.etymologies?.length ?? 0) > 0
  const hasMemory = !!result.word.memory
  const hasOther = hasEtymologies || hasMemory

  return [
    { id: 'definitions', label: '释义', anchorId: 'section-definitions', available: hasDefinitions },
    { id: 'examples', label: '例句', anchorId: 'section-examples', available: hasSentences },
    { id: 'related', label: '相关', anchorId: 'section-related', available: hasRelated },
    { id: 'other', label: '其他', anchorId: 'section-other', available: hasOther },
  ].filter(item => item.available)
}

function getNavLinks(result: WordQueryResponse): NavLink[] {
  const groups = getNavGroups(result)
  const links: NavLink[] = []

  const zhDefs = (result.definitions ?? []).filter(d => d.language === 'zh')
  const enDefs = (result.definitions ?? []).filter(d => d.language === 'en')
  const dualDefs = (result.definitions ?? []).filter(d => d.language === 'dual')

  const relations = result.relations ?? []
  const synonyms = relations.filter(r => r.relation_type === 'synonym' || r.relation_type === 'similar')
  const antonyms = relations.filter(r => r.relation_type === 'antonym')
  const relatedWords = relations.filter(r =>
    r.relation_type !== 'synonym' && r.relation_type !== 'similar' && r.relation_type !== 'antonym'
  )

  const hasEtymologies = (result.etymologies?.length ?? 0) > 0
  const hasMemory = !!result.word.memory

  for (const group of groups) {
    if (group.id === 'definitions') {
      if (zhDefs.length > 0) {
        links.push({
          key: 'definitions-zh',
          sectionId: 'definitions',
          label: '中文释义',
          anchorId: 'section-definitions',
          tabValue: 'zh',
          isChild: true,
        })
      }
      if (dualDefs.length > 0) {
        links.push({
          key: 'definitions-dual',
          sectionId: 'definitions',
          label: '英英释义',
          anchorId: 'section-definitions',
          tabValue: 'dual',
          isChild: true,
        })
      }
      if (enDefs.length > 0) {
        links.push({
          key: 'definitions-en',
          sectionId: 'definitions',
          label: '英文释义',
          anchorId: 'section-definitions',
          tabValue: 'en',
          isChild: true,
        })
      }
      continue
    }

    if (group.id === 'related') {
      if (synonyms.length > 0) {
        links.push({
          key: 'related-synonyms',
          sectionId: 'related',
          label: '近义词',
          anchorId: 'section-related',
          tabValue: 'synonyms',
          isChild: true,
        })
      }
      if (antonyms.length > 0) {
        links.push({
          key: 'related-antonyms',
          sectionId: 'related',
          label: '反义词',
          anchorId: 'section-related',
          tabValue: 'antonyms',
          isChild: true,
        })
      }
      if (relatedWords.length > 0) {
        links.push({
          key: 'related-related',
          sectionId: 'related',
          label: '相关词',
          anchorId: 'section-related',
          tabValue: 'related',
          isChild: true,
        })
      }
      continue
    }

    if (group.id === 'other') {
      if (hasEtymologies) {
        links.push({
          key: 'other-etymology',
          sectionId: 'other',
          label: '词源',
          anchorId: 'section-other',
          tabValue: 'etymology',
          isChild: true,
        })
      }
      if (hasMemory) {
        links.push({
          key: 'other-memory',
          sectionId: 'other',
          label: '记忆技巧',
          anchorId: 'section-other',
          tabValue: 'memory',
          isChild: true,
        })
      }
      continue
    }

    links.push({
      key: group.id,
      sectionId: group.id,
      label: group.label,
      anchorId: group.anchorId,
    })
  }

  return links
}

// Section navigation component for left sidebar
function SectionNav({ result, activeNavKey, onNavKeyChange, onNavigateTab }: {
  result: WordQueryResponse
  activeNavKey: string
  onNavKeyChange: (key: string) => void
  onNavigateTab: (sectionId: string, tabValue?: string) => void
}) {
  const navLinks = getNavLinks(result)

  if (navLinks.length === 0) return null

  const scrollToSection = (item: NavLink) => {
    onNavKeyChange(item.key)
    onNavigateTab(item.sectionId, item.tabValue)

    window.requestAnimationFrame(() => {
      const element = document.getElementById(item.anchorId)
      if (!element) return
      const topOffset = 88
      const y = element.getBoundingClientRect().top + window.scrollY - topOffset
      window.scrollTo({ top: Math.max(0, y), behavior: 'smooth' })
    })
  }

  return (
    <div className="bg-white rounded-xl shadow-lg p-4">
      <div className="space-y-1">
        {navLinks.map((item) => (
          <button
            key={item.key}
            onClick={() => scrollToSection(item)}
            className={`block w-full text-left px-3 py-2 text-sm rounded-lg transition-colors ${
              activeNavKey === item.key
                ? item.isChild
                  ? 'bg-orange-50 text-orange-700'
                  : 'bg-orange-100 text-orange-800 font-medium'
                : item.isChild
                  ? 'text-gray-500 hover:bg-gray-50 hover:text-gray-700'
                  : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'
            }`}
          >
            {item.label}
          </button>
        ))}
      </div>
    </div>
  )
}

// Separate component for word result display
function WordResult({
  result,
  showMoreSentences,
  setShowMoreSentences,
  playAudio,
  isFavorite,
  togglingFavorite,
  onToggleFavorite,
  sectionNavRequest,
}: {
  result: WordQueryResponse
  showMoreSentences: boolean
  setShowMoreSentences: (show: boolean) => void
  playAudio: (pronunciationId: number) => void
  isFavorite: boolean
  togglingFavorite: boolean
  onToggleFavorite: () => void
  sectionNavRequest: SectionNavRequest | null
}) {
  // Compute available sections
  const zhDefs = (result.definitions ?? []).filter(d => d.language === 'zh')
  const enDefs = (result.definitions ?? []).filter(d => d.language === 'en')
  const dualDefs = (result.definitions ?? []).filter(d => d.language === 'dual')
  const hasZhDefs = zhDefs.length > 0
  const hasEnDefs = enDefs.length > 0
  const hasDualDefs = dualDefs.length > 0
  const hasDefinitions = hasZhDefs || hasEnDefs || hasDualDefs

  const hasSentences = (result.sentences?.length ?? 0) > 0

  const relations = result.relations ?? []
  const synonyms = relations.filter(r => r.relation_type === 'synonym' || r.relation_type === 'similar')
  const antonyms = relations.filter(r => r.relation_type === 'antonym')
  const relatedWords = relations.filter(r =>
    r.relation_type !== 'synonym' && r.relation_type !== 'similar' && r.relation_type !== 'antonym'
  )
  const hasSynonyms = synonyms.length > 0
  const hasAntonyms = antonyms.length > 0
  const hasRelatedWords = relatedWords.length > 0
  const hasRelated = hasSynonyms || hasAntonyms || hasRelatedWords

  const hasEtymologies = (result.etymologies?.length ?? 0) > 0
  const hasMemory = !!result.word.memory
  const hasOther = hasEtymologies || hasMemory
  const definitionDefaultTab = hasZhDefs ? 'zh' : hasDualDefs ? 'dual' : 'en'
  const relatedDefaultTab = hasSynonyms ? 'synonyms' : hasAntonyms ? 'antonyms' : 'related'
  const otherDefaultTab = hasEtymologies ? 'etymology' : 'memory'
  const definitionTabs = [
    hasZhDefs ? { value: 'zh', label: '中文释义' } : null,
    hasDualDefs ? { value: 'dual', label: '英英释义' } : null,
    hasEnDefs ? { value: 'en', label: '英文释义' } : null,
  ].filter(Boolean) as { value: string; label: string }[]
  const relatedTabs = [
    hasSynonyms ? { value: 'synonyms', label: '近义词' } : null,
    hasAntonyms ? { value: 'antonyms', label: '反义词' } : null,
    hasRelatedWords ? { value: 'related', label: '相关词' } : null,
  ].filter(Boolean) as { value: string; label: string }[]
  const otherTabs = [
    hasEtymologies ? { value: 'etymology', label: '词源' } : null,
    hasMemory ? { value: 'memory', label: '记忆技巧' } : null,
  ].filter(Boolean) as { value: string; label: string }[]
  const [definitionTab, setDefinitionTab] = useState(definitionDefaultTab)
  const [relatedTab, setRelatedTab] = useState(relatedDefaultTab)
  const [otherTab, setOtherTab] = useState(otherDefaultTab)
  useEffect(() => {
    setDefinitionTab(definitionDefaultTab)
  }, [result.word.id, definitionDefaultTab])
  useEffect(() => {
    setRelatedTab(relatedDefaultTab)
  }, [result.word.id, relatedDefaultTab])
  useEffect(() => {
    setOtherTab(otherDefaultTab)
  }, [result.word.id, otherDefaultTab])
  useEffect(() => {
    if (!sectionNavRequest?.tabValue) return

    if (
      sectionNavRequest.sectionId === 'definitions' &&
      definitionTabs.some(t => t.value === sectionNavRequest.tabValue)
    ) {
      setDefinitionTab(sectionNavRequest.tabValue)
    }
    if (
      sectionNavRequest.sectionId === 'related' &&
      relatedTabs.some(t => t.value === sectionNavRequest.tabValue)
    ) {
      setRelatedTab(sectionNavRequest.tabValue)
    }
    if (
      sectionNavRequest.sectionId === 'other' &&
      otherTabs.some(t => t.value === sectionNavRequest.tabValue)
    ) {
      setOtherTab(sectionNavRequest.tabValue)
    }
  }, [sectionNavRequest?.nonce, definitionTabs, relatedTabs, otherTabs])
  const sectionCardClass = 'bg-white rounded-xl shadow-lg border border-orange-100/80 overflow-hidden'
  const sectionTabsHeaderClass = 'border-b border-orange-100 bg-gradient-to-r from-orange-50 to-amber-50 px-2'
  const sectionTabsListClass = 'relative mb-0 inline-flex h-11 w-auto rounded-none p-0'
  const sectionTabsTriggerClass =
    'relative h-full rounded-none border border-transparent px-4 py-2 text-sm data-[state=active]:bg-white data-[state=active]:text-orange-700 data-[state=inactive]:text-gray-500 data-[state=inactive]:hover:bg-white/70 data-[state=inactive]:hover:text-gray-700'
  const sectionTabsContentClass = 'mt-0 p-5'
  const definitionTabRefs = useRef<Record<string, HTMLButtonElement | null>>({})
  const relatedTabRefs = useRef<Record<string, HTMLButtonElement | null>>({})
  const otherTabRefs = useRef<Record<string, HTMLButtonElement | null>>({})
  const [definitionIndicator, setDefinitionIndicator] = useState({ left: 0, width: 0 })
  const [relatedIndicator, setRelatedIndicator] = useState({ left: 0, width: 0 })
  const [otherIndicator, setOtherIndicator] = useState({ left: 0, width: 0 })

  useEffect(() => {
    const el = definitionTabRefs.current[definitionTab]
    if (el) setDefinitionIndicator({ left: el.offsetLeft, width: el.offsetWidth })
  }, [definitionTab, definitionTabs.length, result.word.id])

  useEffect(() => {
    const el = relatedTabRefs.current[relatedTab]
    if (el) setRelatedIndicator({ left: el.offsetLeft, width: el.offsetWidth })
  }, [relatedTab, relatedTabs.length, result.word.id])

  useEffect(() => {
    const el = otherTabRefs.current[otherTab]
    if (el) setOtherIndicator({ left: el.offsetLeft, width: el.offsetWidth })
  }, [otherTab, otherTabs.length, result.word.id])

  const displayedSentences = showMoreSentences
    ? (result.sentences ?? [])
    : (result.sentences ?? []).slice(0, 10)

  const highlightWord = (sentence: string, word: string) => {
    const escapedWord = word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const regex = new RegExp(`\\b${escapedWord}\\b`, 'gi')
    return sentence.replace(regex, (match) =>
      `<span class="font-bold text-orange-600">${match}</span>`
    )
  }

  return (
    <div className="space-y-4 animate-fade-in">
      {/* Word Header */}
      <div className="bg-white rounded-xl shadow-lg p-5">
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-center gap-2">
            <h2 className="text-3xl font-bold text-gray-900">{result.word.word}</h2>
            <button
              onClick={onToggleFavorite}
              disabled={togglingFavorite}
              className={`transition-colors ${
                isFavorite
                  ? 'text-yellow-500 hover:text-yellow-600'
                  : 'text-gray-300 hover:text-yellow-500'
              }`}
              title={isFavorite ? '取消收藏' : '添加到生词本'}
            >
              {togglingFavorite ? (
                <Loader2 className="w-5 h-5 animate-spin" />
              ) : (
                <Star className={`w-5 h-5 ${isFavorite ? 'fill-current' : ''}`} />
              )}
            </button>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {result.word.difficulty && (
              <span className="px-2 py-0.5 bg-orange-100 text-orange-800 rounded-full text-xs font-medium">
                难度: {result.word.difficulty}
              </span>
            )}
            {result.word.frequency && (
              <span className="px-2 py-0.5 bg-amber-100 text-amber-800 rounded-full text-xs font-medium">
                频率: {result.word.frequency}
              </span>
            )}
            {result.word.word_type && (
              <span className="px-2 py-0.5 bg-yellow-100 text-yellow-800 rounded-full text-xs font-medium">
                {result.word.word_type}
              </span>
            )}
          </div>
        </div>
        {(result.pronunciations?.length ?? 0) > 0 && (
          <div className="flex flex-wrap items-center gap-x-5 gap-y-1 mt-1">
            {result.pronunciations!.map(p => {
              const label = p.dialect === 'UK' ? '英' : p.dialect === 'US' ? '美' : p.dialect
              return (
                <span key={p.id} className="flex items-center gap-1.5 text-gray-600">
                  {label && <span className="text-xs text-gray-400">{label}</span>}
                  <span className="text-sm font-serif italic text-gray-700">/{p.ipa}/</span>
                  <button
                    onClick={() => playAudio(p.id)}
                    className="p-1 rounded-full text-orange-500 hover:text-white hover:bg-orange-500 transition-colors"
                    title={`播放${label || ''}发音`}
                  >
                    <Volume2 className="w-4 h-4" />
                  </button>
                </span>
              )
            })}
          </div>
        )}
        {(result.forms?.length ?? 0) > 0 && (
          <div className="flex flex-wrap gap-2 mt-2">
            {(result.forms ?? []).map((form) => (
              <span
                key={form.id}
                className="px-2 py-0.5 bg-orange-100 text-orange-800 rounded-full text-xs"
              >
                <span className="font-medium">{formTypeLabel(form.form_type)}:</span> {form.form}
              </span>
            ))}
          </div>
        )}
        {(result.dictionaries ?? []).length > 0 && (
          <div className="flex flex-wrap gap-1.5 mt-2">
            {(result.dictionaries ?? []).map((wd) => (
              <span
                key={wd.id}
                className="px-2 py-0.5 bg-amber-100 text-amber-800 rounded-full text-xs font-medium"
              >
                {wd.name}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Definitions Section */}
      {hasDefinitions && (
        <div id="section-definitions" className={sectionCardClass}>
          <Tabs
            key={`definitions-${result.word.id}-${definitionDefaultTab}`}
            value={definitionTab}
            onValueChange={setDefinitionTab}
          >
            <div className={sectionTabsHeaderClass}>
              <TabsList className={sectionTabsListClass}>
                {definitionTabs.map((tab) => (
                  <TabsTrigger
                    key={tab.value}
                    value={tab.value}
                    className={sectionTabsTriggerClass}
                    ref={(el) => { definitionTabRefs.current[tab.value] = el }}
                  >
                    {tab.label}
                  </TabsTrigger>
                ))}
                <div
                  className="pointer-events-none absolute bottom-0 h-0.5 bg-orange-500 transition-all duration-300 ease-out"
                  style={{
                    left: definitionIndicator.left,
                    width: definitionIndicator.width,
                  }}
                />
              </TabsList>
            </div>

            {hasZhDefs && (
              <TabsContent value="zh" className={sectionTabsContentClass}>
                <DefinitionGroups defs={zhDefs} color="orange" />
              </TabsContent>
            )}

            {hasDualDefs && (
              <TabsContent value="dual" className={sectionTabsContentClass}>
                <DefinitionGroups defs={dualDefs} color="emerald" />
              </TabsContent>
            )}

            {hasEnDefs && (
              <TabsContent value="en" className={sectionTabsContentClass}>
                <DefinitionGroups defs={enDefs} color="blue" />
              </TabsContent>
            )}
          </Tabs>
        </div>
      )}

      {/* Example Sentences Section */}
      {hasSentences && (
        <div id="section-examples" className="bg-white rounded-xl shadow-lg p-5">
          <h3 className="text-sm font-medium text-gray-500 mb-3">使用例句</h3>
          <div className="space-y-2">
            {displayedSentences.map((sentence) => (
              <div key={sentence.id} className="bg-gray-50 rounded-lg p-3">
                <p
                  className="text-gray-800 text-sm"
                  dangerouslySetInnerHTML={{
                    __html: highlightWord(sentence.sentence, result.word.word)
                  }}
                />
                {sentence.source && (
                  <p className="text-gray-500 text-xs mt-1">— {sentence.source}</p>
                )}
              </div>
            ))}
          </div>
          {(result.sentences ?? []).length > 10 && (
            <button
              onClick={() => setShowMoreSentences(!showMoreSentences)}
              className="mt-3 w-full py-2 text-orange-600 hover:text-orange-800 text-sm font-medium transition-colors"
            >
              {showMoreSentences ? '收起' : `查看更多 (${((result.sentences ?? []).length - 10)} 条)`}
            </button>
          )}
        </div>
      )}

      {/* Related Section (Synonyms, Antonyms, Related Words) */}
      {hasRelated && (
        <div id="section-related" className={sectionCardClass}>
          <Tabs
            key={`related-${result.word.id}-${relatedDefaultTab}`}
            value={relatedTab}
            onValueChange={setRelatedTab}
          >
            <div className={sectionTabsHeaderClass}>
              <TabsList className={sectionTabsListClass}>
                {relatedTabs.map((tab) => (
                  <TabsTrigger
                    key={tab.value}
                    value={tab.value}
                    className={sectionTabsTriggerClass}
                    ref={(el) => { relatedTabRefs.current[tab.value] = el }}
                  >
                    {tab.label}
                  </TabsTrigger>
                ))}
                <div
                  className="pointer-events-none absolute bottom-0 h-0.5 bg-orange-500 transition-all duration-300 ease-out"
                  style={{
                    left: relatedIndicator.left,
                    width: relatedIndicator.width,
                  }}
                />
              </TabsList>
            </div>

            {hasSynonyms && (
              <TabsContent value="synonyms" className={sectionTabsContentClass}>
                <div className="flex flex-wrap gap-2">
                  {synonyms.map((rel) => (
                    <Link
                      key={rel.id}
                      to={`/dict/${encodeURIComponent(rel.related_word)}`}
                      className="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm cursor-pointer hover:bg-green-200 transition-colors"
                    >
                      {rel.related_word}
                    </Link>
                  ))}
                </div>
              </TabsContent>
            )}

            {hasAntonyms && (
              <TabsContent value="antonyms" className={sectionTabsContentClass}>
                <div className="flex flex-wrap gap-2">
                  {antonyms.map((rel) => (
                    <Link
                      key={rel.id}
                      to={`/dict/${encodeURIComponent(rel.related_word)}`}
                      className="px-3 py-1 bg-red-100 text-red-800 rounded-full text-sm cursor-pointer hover:bg-red-200 transition-colors"
                    >
                      {rel.related_word}
                    </Link>
                  ))}
                </div>
              </TabsContent>
            )}

            {hasRelatedWords && (
              <TabsContent value="related" className={sectionTabsContentClass}>
                <div className="flex flex-wrap gap-2">
                  {relatedWords.map((rel) => (
                    <Link
                      key={rel.id}
                      to={`/dict/${encodeURIComponent(rel.related_word)}`}
                      className="px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-sm cursor-pointer hover:bg-blue-200 transition-colors"
                    >
                      {rel.related_word}
                    </Link>
                  ))}
                </div>
              </TabsContent>
            )}
          </Tabs>
        </div>
      )}

      {/* Other Section (Etymology, Memory) */}
      {hasOther && (
        <div id="section-other" className={sectionCardClass}>
          <Tabs
            key={`other-${result.word.id}-${otherDefaultTab}`}
            value={otherTab}
            onValueChange={setOtherTab}
          >
            <div className={sectionTabsHeaderClass}>
              <TabsList className={sectionTabsListClass}>
                {otherTabs.map((tab) => (
                  <TabsTrigger
                    key={tab.value}
                    value={tab.value}
                    className={sectionTabsTriggerClass}
                    ref={(el) => { otherTabRefs.current[tab.value] = el }}
                  >
                    {tab.label}
                  </TabsTrigger>
                ))}
                <div
                  className="pointer-events-none absolute bottom-0 h-0.5 bg-orange-500 transition-all duration-300 ease-out"
                  style={{
                    left: otherIndicator.left,
                    width: otherIndicator.width,
                  }}
                />
              </TabsList>
            </div>

            {hasEtymologies && (
              <TabsContent value="etymology" className={sectionTabsContentClass}>
                <div className="space-y-2">
                  {(result.etymologies ?? []).map((etym) => (
                    <div key={etym.id} className="bg-amber-50 rounded-lg p-3 border border-amber-200">
                      <p className="text-sm text-gray-800">{etym.etymology}</p>
                      {etym.origin_language && (
                        <p className="text-xs text-gray-500 mt-1">来源: {etym.origin_language}</p>
                      )}
                    </div>
                  ))}
                </div>
              </TabsContent>
            )}

            {hasMemory && (
              <TabsContent value="memory" className={sectionTabsContentClass}>
                <div className="bg-purple-50 rounded-lg p-4 border border-purple-200">
                  <p className="text-gray-800 whitespace-pre-wrap">{result.word.memory}</p>
                </div>
              </TabsContent>
            )}
          </Tabs>
        </div>
      )}

      {/* Categories - always visible */}
      {(result.categories?.length ?? 0) > 0 && (
        <div className="bg-white rounded-xl shadow-lg p-5">
          <h3 className="text-lg font-semibold text-gray-900 mb-3">分类</h3>
          <div className="flex flex-wrap gap-1.5">
            {(result.categories ?? []).map((cat) => (
              <span
                key={cat.id}
                className="px-2 py-0.5 bg-orange-100 text-orange-800 rounded-full text-xs"
              >
                {cat.name}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
