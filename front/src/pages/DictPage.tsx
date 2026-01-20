import { useState } from 'react'
import { Search, Volume2, Star } from 'lucide-react'
import { lookup, type WordQueryResponse } from '../lib/api'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../components/ui/tabs'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'

export function DictPage() {
  const [query, setQuery] = useState('')
  const [result, setResult] = useState<WordQueryResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [showMoreSentences, setShowMoreSentences] = useState(false)

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!query.trim()) return

    setLoading(true)
    setError('')
    try {
      const data = await lookup(query.trim())
      setResult(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch word')
      setResult(null)
    } finally {
      setLoading(false)
    }
  }

  const playAudio = (url: string | null) => {
    if (url) {
      new Audio(url).play()
    }
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />
      <div className="container mx-auto px-4 py-4 max-w-6xl">
        <div className="text-center mb-4">
          <h1 className="text-2xl font-bold text-orange-900 mb-1">
            词典查询
          </h1>
          <p className="text-gray-600 text-sm">查询单词，探索英语世界</p>
        </div>

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

            {(result.etymologies?.length ?? 0) > 0 && (
              <div className="bg-white rounded-xl shadow-lg p-5">
                <h3 className="text-xl font-bold text-gray-900 mb-3">词源</h3>
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
            )}

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
        )}
      </div>
      <Footer />
    </div>
  )
}
