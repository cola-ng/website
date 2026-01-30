import { useState, useEffect, useCallback, useRef } from 'react'
import { useParams, useNavigate, useLocation, Link } from 'react-router-dom'
import { Search, Volume2, Star, Languages, BookOpen, Loader2, Copy, Check, History, Trash2, X } from 'lucide-react'
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
  toggleWordInVocabulary,
  type WordQueryResponse,
} from '../lib/api'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../components/ui/tabs'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { useAuth } from '../lib/auth'
import { useToast } from '../contexts/ToastContext'

// Unified search history item type
type SearchHistoryItem = {
  id?: number
  word: string
  searched_at: string
}

export function DictPage() {
  const { word: urlWord } = useParams<{ word: string }>()
  const navigate = useNavigate()
  const location = useLocation()
  const { token } = useAuth()
  const toast = useToast()

  // Determine active tab from URL
  const activeTab = location.pathname === '/translate' ? 'translate' : 'lookup'

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
  const [activeGroup, setActiveGroup] = useState('definitions')
  const [activeDefTab, setActiveDefTab] = useState('definitions-zh')
  const [activeExampleTab, setActiveExampleTab] = useState('examples')
  const [activeRelatedTab, setActiveRelatedTab] = useState('synonyms')

  // Translation state
  const [sourceText, setSourceText] = useState('')
  const [translatedText, setTranslatedText] = useState('')
  const [translating, setTranslating] = useState(false)
  const [translateError, setTranslateError] = useState('')
  const [detectedLang, setDetectedLang] = useState<'en' | 'zh' | null>(null)
  const [copied, setCopied] = useState(false)

  // Search history state
  const [searchHistory, setSearchHistory] = useState<SearchHistoryItem[]>([])
  const [_loadingHistory, setLoadingHistory] = useState(false)
  const [isFavorite, setIsFavorite] = useState(false)
  const [togglingFavorite, setTogglingFavorite] = useState(false)
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
        setActiveGroup(navGroups[0].id)
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

  useEffect(() => {
    if (urlWord) {
      const fromHistory = (location.state as { fromHistory?: boolean })?.fromHistory === true
      searchWord(decodeURIComponent(urlWord), fromHistory)
      // Clear the state after using it
      if (fromHistory) {
        window.history.replaceState({}, '')
      }
    }
  }, [urlWord, searchWord, location.state])

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
      const res = await toggleWordInVocabulary(token, result.word.word)
      setIsFavorite(res.added)
      toast.success(res.added ? '已收藏' : '已取消收藏', res.added ? '单词已添加到生词本' : '单词已从生词本移除')
    } catch {
      toast.error('操作失败', '无法更新收藏状态')
    } finally {
      setTogglingFavorite(false)
    }
  }

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!query.trim()) return
    navigate(`/dict/${encodeURIComponent(query.trim())}`)
  }

  const playAudio = (url: string | null) => {
    if (url) {
      new Audio(url).play()
    }
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
        { text: sourceText.trim(), source: 'auto', target: targetLang },
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
          <TabsList className="grid w-full max-w-md mx-auto grid-cols-2 mb-4">
            <TabsTrigger value="lookup" className="gap-2">
              <BookOpen className="w-4 h-4" />
              词典查询
            </TabsTrigger>
            <TabsTrigger value="translate" className="gap-2">
              <Languages className="w-4 h-4" />
              内容翻译
            </TabsTrigger>
          </TabsList>

          {/* Word Lookup Tab */}
          <TabsContent value="lookup">
            {/* Three-column layout: always render structure to prevent shift */}
            <div className="grid grid-cols-1 lg:grid-cols-[200px_1fr_200px] gap-6">
              {/* Left column - Navigation placeholder */}
              <div className="hidden lg:block">
                {result && (
                  <SectionNav
                    result={result}
                    activeGroup={activeGroup}
                    onGroupChange={setActiveGroup}
                  />
                )}
              </div>

              {/* Center column - Search and content */}
              <div className="min-w-0">
                <form onSubmit={handleSearch} className="mb-4">
                  <div className="flex gap-2 max-w-xl mx-auto">
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

                {error && (
                  <div className="max-w-xl mx-auto mb-6 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
                    {error}
                  </div>
                )}

                {result && (
                  <WordResult
                    result={result}
                    showMoreSentences={showMoreSentences}
                    setShowMoreSentences={setShowMoreSentences}
                    playAudio={playAudio}
                    activeDefTab={activeDefTab}
                    onDefTabChange={setActiveDefTab}
                    activeExampleTab={activeExampleTab}
                    onExampleTabChange={setActiveExampleTab}
                    activeRelatedTab={activeRelatedTab}
                    onRelatedTabChange={setActiveRelatedTab}
                    isFavorite={isFavorite}
                    togglingFavorite={togglingFavorite}
                    onToggleFavorite={handleToggleFavorite}
                  />
                )}
              </div>

              {/* Right column - Search History */}
              <div className="hidden lg:block">
                {result && searchHistory.length > 0 && (
                  <div className="bg-white rounded-xl shadow-lg p-4 sticky top-4">
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
                      {searchHistory.map((item, index) => (
                        <div
                          key={item.id ?? `local-${index}`}
                          className={`group flex items-center justify-between px-3 py-2 text-sm rounded-lg transition-colors ${
                            item.word.toLowerCase() === result?.word.word.toLowerCase()
                              ? 'bg-orange-100 text-orange-800 font-medium'
                              : 'text-gray-700 hover:bg-gray-100'
                          }`}
                        >
                          <button
                            onClick={() => {
                              navigate(`/dict/${encodeURIComponent(item.word)}`, { state: { fromHistory: true } })
                            }}
                            className="flex-1 truncate text-left"
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
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          </TabsContent>

          {/* Translation Tab */}
          <TabsContent value="translate">
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
          </TabsContent>
        </Tabs>
      </div>
      <Footer />
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

// Helper function to compute nav groups
function getNavGroups(result: WordQueryResponse): NavGroup[] {
  const zhDefs = (result.definitions ?? []).filter(d => d.language === 'zh')
  const enDefs = (result.definitions ?? []).filter(d => d.language === 'en')
  const hasDefinitions = zhDefs.length > 0 || enDefs.length > 0

  const hasSentences = (result.sentences?.length ?? 0) > 0
  const hasMemory = !!result.word.memory
  const hasExamples = hasSentences || hasMemory

  const relations = result.relations ?? []
  const synonyms = relations.filter(r => r.relation_type === 'synonym' || r.relation_type === 'similar')
  const antonyms = relations.filter(r => r.relation_type === 'antonym')
  const relatedWords = relations.filter(r =>
    r.relation_type !== 'synonym' && r.relation_type !== 'similar' && r.relation_type !== 'antonym'
  )
  const hasEtymologies = (result.etymologies?.length ?? 0) > 0
  const hasRelated = synonyms.length > 0 || antonyms.length > 0 || relatedWords.length > 0 || hasEtymologies

  return [
    { id: 'definitions', label: '释义', anchorId: 'section-definitions', available: hasDefinitions },
    { id: 'examples', label: '例句', anchorId: 'section-examples', available: hasExamples },
    { id: 'related', label: '相关', anchorId: 'section-related', available: hasRelated },
  ].filter(item => item.available)
}

// Tab item type for individual tabs within a group
type TabItem = {
  id: string
  label: string
  available: boolean
}

// Section navigation component for left sidebar
function SectionNav({ result, activeGroup, onGroupChange }: {
  result: WordQueryResponse
  activeGroup: string
  onGroupChange: (groupId: string) => void
}) {
  const navGroups = getNavGroups(result)

  if (navGroups.length === 0) return null

  const scrollToSection = (anchorId: string, groupId: string) => {
    onGroupChange(groupId)
    const element = document.getElementById(anchorId)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }
  }

  return (
    <div className="bg-white rounded-xl shadow-lg p-4 sticky top-4">
      <div className="space-y-1">
        {navGroups.map((group) => (
          <button
            key={group.id}
            onClick={() => scrollToSection(group.anchorId, group.id)}
            className={`block w-full text-left px-3 py-2 text-sm rounded-lg transition-colors ${
              activeGroup === group.id
                ? 'bg-orange-100 text-orange-800 font-medium'
                : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'
            }`}
          >
            {group.label}
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
  activeDefTab,
  onDefTabChange,
  activeExampleTab,
  onExampleTabChange,
  activeRelatedTab,
  onRelatedTabChange,
  isFavorite,
  togglingFavorite,
  onToggleFavorite,
}: {
  result: WordQueryResponse
  showMoreSentences: boolean
  setShowMoreSentences: (show: boolean) => void
  playAudio: (url: string | null) => void
  activeDefTab: string
  onDefTabChange: (tab: string) => void
  activeExampleTab: string
  onExampleTabChange: (tab: string) => void
  activeRelatedTab: string
  onRelatedTabChange: (tab: string) => void
  isFavorite: boolean
  togglingFavorite: boolean
  onToggleFavorite: () => void
}) {
  // Compute available sections
  const zhDefs = (result.definitions ?? []).filter(d => d.language === 'zh')
  const enDefs = (result.definitions ?? []).filter(d => d.language === 'en')
  const hasZhDefs = zhDefs.length > 0
  const hasEnDefs = enDefs.length > 0
  const hasDefinitions = hasZhDefs || hasEnDefs

  const hasSentences = (result.sentences?.length ?? 0) > 0
  const hasMemory = !!result.word.memory
  const hasExamples = hasSentences || hasMemory

  const relations = result.relations ?? []
  const synonyms = relations.filter(r => r.relation_type === 'synonym' || r.relation_type === 'similar')
  const antonyms = relations.filter(r => r.relation_type === 'antonym')
  const relatedWords = relations.filter(r =>
    r.relation_type !== 'synonym' && r.relation_type !== 'similar' && r.relation_type !== 'antonym'
  )
  const hasSynonyms = synonyms.length > 0
  const hasAntonyms = antonyms.length > 0
  const hasRelatedWords = relatedWords.length > 0
  const hasEtymologies = (result.etymologies?.length ?? 0) > 0
  const hasRelated = hasSynonyms || hasAntonyms || hasRelatedWords || hasEtymologies

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

  // Build tab items for each group
  const defTabs: TabItem[] = [
    { id: 'definitions-zh', label: '中文释义', available: hasZhDefs },
    { id: 'definitions-en', label: '英文释义', available: hasEnDefs },
  ].filter(t => t.available)

  const exampleTabs: TabItem[] = [
    { id: 'examples', label: '使用例句', available: hasSentences },
    { id: 'memory', label: '记忆技巧', available: hasMemory },
  ].filter(t => t.available)

  const relatedTabs: TabItem[] = [
    { id: 'synonyms', label: '近义词', available: hasSynonyms },
    { id: 'antonyms', label: '反义词', available: hasAntonyms },
    { id: 'related', label: '相关词', available: hasRelatedWords },
    { id: 'etymology', label: '词源', available: hasEtymologies },
  ].filter(t => t.available)

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
        <div className="flex flex-wrap gap-2 text-sm text-gray-600">
          {result.pronunciations?.filter(p => p.dialect === 'UK').map(p => (
            <span key={p.id} className="flex items-center gap-1">
              英 /{p.ipa}/
              <button
                onClick={() => playAudio(p.audio_url)}
                className="text-orange-600 hover:text-orange-800"
                title="Play audio"
              >
                <Volume2 className="w-3.5 h-3.5" />
              </button>
            </span>
          ))}
          {result.pronunciations?.filter(p => p.dialect === 'US').map(p => (
            <span key={p.id} className="flex items-center gap-1">
              美 /{p.ipa}/
              <button
                onClick={() => playAudio(p.audio_url)}
                className="text-orange-600 hover:text-orange-800"
                title="Play audio"
              >
                <Volume2 className="w-3.5 h-3.5" />
              </button>
            </span>
          ))}
        </div>
        {(result.forms?.length ?? 0) > 0 && (
          <div className="flex flex-wrap gap-2 mt-2">
            {(result.forms ?? []).map((form) => (
              <span
                key={form.id}
                className="px-2 py-0.5 bg-orange-100 text-orange-800 rounded-full text-xs"
              >
                <span className="font-medium">{form.form_type}:</span> {form.form}
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

      {/* Tab Group 1: Definitions */}
      {hasDefinitions && (
        <div id="section-definitions">
          <Tabs value={activeDefTab} onValueChange={onDefTabChange}>
            <TabsList className="flex flex-wrap gap-1 bg-white rounded-xl shadow-lg p-2 mb-4">
              {defTabs.map((item) => (
                <TabsTrigger
                  key={item.id}
                  value={item.id}
                  className="text-sm px-3 py-1.5"
                >
                  {item.label}
                </TabsTrigger>
              ))}
            </TabsList>

            {/* Chinese Definitions */}
            {hasZhDefs && (
              <TabsContent value="definitions-zh">
                <div className="bg-white rounded-xl shadow-lg p-5">
                  <div className="space-y-3">
                    {zhDefs.map((def) => (
                      <div key={def.id} className="pl-3 border-l-2 border-orange-500">
                        <div className="flex items-center gap-2 mb-0.5">
                          {def.part_of_speech && (
                            <span className="px-2 py-0.5 bg-orange-200 text-orange-800 rounded text-xs font-medium">
                              {def.part_of_speech}
                            </span>
                          )}
                          {def.register && (
                            <span className="text-xs text-gray-500">{def.register}</span>
                          )}
                          {def.region && (
                            <span className="text-xs text-gray-500">{def.region}</span>
                          )}
                        </div>
                        <p className="text-base text-gray-800">{def.definition}</p>
                        {def.usage_notes && (
                          <p className="text-xs text-gray-500 mt-0.5 italic">{def.usage_notes}</p>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              </TabsContent>
            )}

            {/* English Definitions */}
            {hasEnDefs && (
              <TabsContent value="definitions-en">
                <div className="bg-white rounded-xl shadow-lg p-5">
                  <div className="space-y-3">
                    {enDefs.map((def) => (
                      <div key={def.id} className="pl-3 border-l-2 border-orange-500">
                        <div className="flex items-center gap-2 mb-0.5">
                          {def.part_of_speech && (
                            <span className="px-2 py-0.5 bg-orange-200 text-orange-800 rounded text-xs font-medium">
                              {def.part_of_speech}
                            </span>
                          )}
                          {def.register && (
                            <span className="text-xs text-gray-500">{def.register}</span>
                          )}
                          {def.region && (
                            <span className="text-xs text-gray-500">{def.region}</span>
                          )}
                        </div>
                        <p className="text-base text-gray-800">{def.definition}</p>
                        {def.usage_notes && (
                          <p className="text-xs text-gray-500 mt-0.5 italic">{def.usage_notes}</p>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              </TabsContent>
            )}
          </Tabs>
        </div>
      )}

      {/* Tab Group 2: Examples & Memory */}
      {hasExamples && (
        <div id="section-examples">
          <Tabs value={activeExampleTab} onValueChange={onExampleTabChange}>
            <TabsList className="flex flex-wrap gap-1 bg-white rounded-xl shadow-lg p-2 mb-4">
              {exampleTabs.map((item) => (
                <TabsTrigger
                  key={item.id}
                  value={item.id}
                  className="text-sm px-3 py-1.5"
                >
                  {item.label}
                </TabsTrigger>
              ))}
            </TabsList>

            {/* Example Sentences */}
            {hasSentences && (
              <TabsContent value="examples">
                <div className="bg-white rounded-xl shadow-lg p-5">
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
              </TabsContent>
            )}

            {/* Memory Tips */}
            {hasMemory && (
              <TabsContent value="memory">
                <div className="bg-white rounded-xl shadow-lg p-5">
                  <div className="bg-purple-50 rounded-lg p-4 border border-purple-200">
                    <p className="text-gray-800 whitespace-pre-wrap">{result.word.memory}</p>
                  </div>
                </div>
              </TabsContent>
            )}
          </Tabs>
        </div>
      )}

      {/* Tab Group 3: Related (Synonyms, Antonyms, Related Words, Etymology) */}
      {hasRelated && (
        <div id="section-related">
          <Tabs value={activeRelatedTab} onValueChange={onRelatedTabChange}>
            <TabsList className="flex flex-wrap gap-1 bg-white rounded-xl shadow-lg p-2 mb-4">
              {relatedTabs.map((item) => (
                <TabsTrigger
                  key={item.id}
                  value={item.id}
                  className="text-sm px-3 py-1.5"
                >
                  {item.label}
                </TabsTrigger>
              ))}
            </TabsList>

            {/* Synonyms */}
            {hasSynonyms && (
              <TabsContent value="synonyms">
                <div className="bg-white rounded-xl shadow-lg p-5">
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
                </div>
              </TabsContent>
            )}

            {/* Antonyms */}
            {hasAntonyms && (
              <TabsContent value="antonyms">
                <div className="bg-white rounded-xl shadow-lg p-5">
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
                </div>
              </TabsContent>
            )}

            {/* Related Words */}
            {hasRelatedWords && (
              <TabsContent value="related">
                <div className="bg-white rounded-xl shadow-lg p-5">
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
                </div>
              </TabsContent>
            )}

            {/* Etymology */}
            {hasEtymologies && (
              <TabsContent value="etymology">
                <div className="bg-white rounded-xl shadow-lg p-5">
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
