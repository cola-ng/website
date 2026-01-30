import { useState, useEffect, useCallback } from 'react'
import { useParams, useNavigate, useLocation, Link } from 'react-router-dom'
import { Search, Volume2, Star, Languages, BookOpen, Loader2, Copy, Check } from 'lucide-react'
import { lookup, translate, type WordQueryResponse } from '../lib/api'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../components/ui/tabs'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { useAuth } from '../lib/auth'
import { useToast } from '../contexts/ToastContext'

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

  // Translation state
  const [sourceText, setSourceText] = useState('')
  const [translatedText, setTranslatedText] = useState('')
  const [translating, setTranslating] = useState(false)
  const [translateError, setTranslateError] = useState('')
  const [detectedLang, setDetectedLang] = useState<'en' | 'zh' | null>(null)
  const [copied, setCopied] = useState(false)

  const searchWord = useCallback(async (word: string) => {
    if (!word.trim()) return

    setLoading(true)
    setError('')
    try {
      const data = await lookup(word.trim())
      setResult(data)
      setQuery(word.trim())
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch word')
      setResult(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (urlWord) {
      searchWord(decodeURIComponent(urlWord))
    }
  }, [urlWord, searchWord])

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
            词典翻译
          </h1>
          <p className="text-gray-600 text-sm">单词查询与内容翻译</p>
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
              />
            )}
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

// Separate component for word result display
function WordResult({
  result,
  showMoreSentences,
  setShowMoreSentences,
  playAudio,
}: {
  result: WordQueryResponse
  showMoreSentences: boolean
  setShowMoreSentences: (show: boolean) => void
  playAudio: (url: string | null) => void
}) {
  return (
    <div className="space-y-4 animate-fade-in">
      <div className="bg-white rounded-xl shadow-lg p-5">
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-center gap-2">
            <h2 className="text-3xl font-bold text-gray-900">{result.word.word}</h2>
            <button className="text-gray-300 hover:text-yellow-500 transition-colors">
              <Star className="w-5 h-5" />
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

      {(result.definitions ?? []).length > 0 && (() => {
        const zhDefs = (result.definitions ?? []).filter(d => d.language === 'zh')
        const enDefs = (result.definitions ?? []).filter(d => d.language === 'en')
        const hasZh = zhDefs.length > 0
        const hasEn = enDefs.length > 0
        const defaultTab = hasZh ? 'zh' : 'en'

        return (
          <Tabs defaultValue={defaultTab}>
            <TabsList className={`grid w-full max-w-sm ${hasZh && hasEn ? 'grid-cols-2' : 'grid-cols-1'}`}>
              {hasZh && <TabsTrigger value="zh">中文释义</TabsTrigger>}
              {hasEn && <TabsTrigger value="en">英文释义</TabsTrigger>}
            </TabsList>
            {hasZh && (
              <TabsContent value="zh" className="mt-3">
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
            {hasEn && (
              <TabsContent value="en" className="mt-3">
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
        )
      })()}

      {(() => {
        const hasSentences = (result.sentences?.length ?? 0) > 0
        const hasCollocations = false
        const hasAny = hasSentences || hasCollocations

        if (!hasAny) return null

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
          <Tabs defaultValue={hasSentences ? 'sentences' : 'collocations'}>
            <TabsList className={`grid w-full max-w-sm ${hasSentences && hasCollocations ? 'grid-cols-2' : 'grid-cols-1'}`}>
              {hasSentences && <TabsTrigger value="sentences">使用例句</TabsTrigger>}
              {hasCollocations && <TabsTrigger value="collocations">词汇搭配</TabsTrigger>}
            </TabsList>
            {hasSentences && (
              <TabsContent value="sentences" className="mt-3">
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
            {hasCollocations && (
              <TabsContent value="collocations" className="mt-3">
                <div className="bg-white rounded-xl shadow-lg p-5">
                  <h3 className="text-xl font-bold text-gray-900 mb-3">词汇搭配</h3>
                  <p className="text-gray-500 italic text-sm">词汇搭配功能开发中...</p>
                </div>
              </TabsContent>
            )}
          </Tabs>
        )
      })()}

      {(() => {
        const hasEtymologies = (result.etymologies?.length ?? 0) > 0
        const relations = result.relations ?? []
        const synonyms = relations.filter(r => r.relation_type === 'synonym' || r.relation_type === 'similar')
        const antonyms = relations.filter(r => r.relation_type === 'antonym')
        const relatedWords = relations.filter(r =>
          r.relation_type !== 'synonym' && r.relation_type !== 'similar' && r.relation_type !== 'antonym'
        )

        const hasSynonyms = synonyms.length > 0
        const hasAntonyms = antonyms.length > 0
        const hasRelated = relatedWords.length > 0
        const hasAny = hasEtymologies || hasSynonyms || hasAntonyms || hasRelated

        if (!hasAny) return null

        const defaultTab = hasSynonyms ? 'synonyms' : hasAntonyms ? 'antonyms' : hasRelated ? 'related' : 'etymology'

        const tabCount = [hasSynonyms, hasAntonyms, hasRelated, hasEtymologies].filter(Boolean).length
        const gridColsClass = tabCount === 1 ? 'grid-cols-1' : tabCount === 2 ? 'grid-cols-2' : tabCount === 3 ? 'grid-cols-3' : 'grid-cols-4'

        return (
          <Tabs defaultValue={defaultTab}>
            <TabsList className={`grid w-full max-w-md ${gridColsClass}`}>
              {hasSynonyms && <TabsTrigger value="synonyms">近义词</TabsTrigger>}
              {hasAntonyms && <TabsTrigger value="antonyms">反义词</TabsTrigger>}
              {hasRelated && <TabsTrigger value="related">相关词</TabsTrigger>}
              {hasEtymologies && <TabsTrigger value="etymology">词源</TabsTrigger>}
            </TabsList>
            {hasSynonyms && (
              <TabsContent value="synonyms" className="mt-3">
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
            {hasAntonyms && (
              <TabsContent value="antonyms" className="mt-3">
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
            {hasRelated && (
              <TabsContent value="related" className="mt-3">
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
            {hasEtymologies && (
              <TabsContent value="etymology" className="mt-3">
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
        )
      })()}

      {(result.categories?.length ?? 0) > 0 && (
        <div className="bg-white rounded-xl shadow-lg p-5">
          <h3 className="text-xl font-bold text-gray-900 mb-3">分类</h3>
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
