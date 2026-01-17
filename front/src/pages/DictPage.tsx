import { useState } from 'react'
import { Search, BookOpen, Volume2, Star } from 'lucide-react'
import { lookup, type WordQueryResponse } from '../lib/api'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '../components/ui/tabs'

export function DictPage() {
  const [query, setQuery] = useState('')
  const [result, setResult] = useState<WordQueryResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

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
    <div className="min-h-screen bg-gradient-to-br from-blue-50 via-indigo-50 to-purple-50">
      <div className="container mx-auto px-4 py-8 max-w-6xl">
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold text-indigo-900 mb-2 flex items-center justify-center gap-3">
            <BookOpen className="w-10 h-10" />
            Dictionary
          </h1>
          <p className="text-gray-600">Search for English words and explore their meanings</p>
        </div>

        <form onSubmit={handleSearch} className="mb-8">
          <div className="flex gap-3 max-w-2xl mx-auto">
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Enter a word..."
              className="flex-1 px-6 py-4 rounded-xl border-2 border-indigo-200 focus:border-indigo-500 focus:outline-none text-lg shadow-sm transition-all"
            />
            <button
              type="submit"
              disabled={loading}
              className="px-8 py-4 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-xl shadow-md transition-all flex items-center gap-2 disabled:opacity-50"
            >
              <Search className="w-5 h-5" />
              {loading ? 'Searching...' : 'Search'}
            </button>
          </div>
        </form>

        {error && (
          <div className="max-w-2xl mx-auto mb-8 p-4 bg-red-50 border border-red-200 rounded-xl text-red-700">
            {error}
          </div>
        )}

        {result && (
          <div className="space-y-6 animate-fade-in">
            <div className="bg-white rounded-2xl shadow-lg p-8">
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-2">
                  <h2 className="text-4xl font-bold text-gray-900 mb-2">{result.word.word}</h2>
                  <button className="text-gray-300 hover:text-yellow-500 transition-colors">
                    <Star className="w-6 h-6" />
                  </button>
                </div>
                <div className="flex flex-wrap gap-2">
                  {result.word.difficulty && (
                    <span className="px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-sm font-medium">
                      Difficulty: {result.word.difficulty}
                    </span>
                  )}
                  {result.word.frequency && (
                    <span className="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm font-medium">
                      Frequency: {result.word.frequency}
                    </span>
                  )}
                  {result.word.word_type && (
                    <span className="px-3 py-1 bg-purple-100 text-purple-800 rounded-full text-sm font-medium">
                      {result.word.word_type}
                    </span>
                  )}
                </div>
              </div>
              <div className="flex flex-wrap gap-3 text-gray-600">
                {result.pronunciations?.filter(p => p.dialect === 'UK').map(p => (
                  <span key={p.id} className="flex items-center gap-1">
                    英 /{p.ipa}/
                    <button
                      onClick={() => playAudio(p.audio_url)}
                      className="text-indigo-600 hover:text-indigo-800"
                      title="Play audio"
                    >
                      <Volume2 className="w-4 h-4" />
                    </button>
                  </span>
                ))}
                {result.pronunciations?.filter(p => p.dialect === 'US').map(p => (
                  <span key={p.id} className="flex items-center gap-1">
                    美 /{p.ipa}/
                    <button
                      onClick={() => playAudio(p.audio_url)}
                      className="text-indigo-600 hover:text-indigo-800"
                      title="Play audio"
                    >
                      <Volume2 className="w-4 h-4" />
                    </button>
                  </span>
                ))}
              </div>
              {(result.forms?.length ?? 0) > 0 && (
                <div className="flex flex-wrap gap-3 mt-3">
                  {(result.forms ?? []).map((form) => (
                    <span
                      key={form.id}
                      className="px-3 py-1 bg-indigo-100 text-indigo-800 rounded-full text-sm"
                    >
                      <span className="font-medium">{form.form_type}:</span> {form.form}
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
                  <TabsList className={`grid w-full max-w-md ${hasZh && hasEn ? 'grid-cols-2' : 'grid-cols-1'}`}>
                    {hasZh && <TabsTrigger value="zh">中文释义</TabsTrigger>}
                    {hasEn && <TabsTrigger value="en">英文释义</TabsTrigger>}
                  </TabsList>
                  {hasZh && (
                    <TabsContent value="zh" className="mt-4">
                      <div className="bg-white rounded-2xl shadow-lg p-8">
                        <div className="space-y-4">
                          {zhDefs.map((def) => (
                            <div key={def.id} className="pl-4 border-l-4 border-indigo-500">
                              <div className="flex items-center gap-2 mb-1">
                                {def.part_of_speech && (
                                  <span className="px-2 py-0.5 bg-gray-200 text-gray-700 rounded text-sm font-medium">
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
                              <p className="text-lg text-gray-800">{def.definition}</p>
                              {def.usage_notes && (
                                <p className="text-sm text-gray-500 mt-1 italic">{def.usage_notes}</p>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    </TabsContent>
                  )}
                  {hasEn && (
                    <TabsContent value="en" className="mt-4">
                      <div className="bg-white rounded-2xl shadow-lg p-8">
                        <div className="space-y-4">
                          {enDefs.map((def) => (
                            <div key={def.id} className="pl-4 border-l-4 border-indigo-500">
                              <div className="flex items-center gap-2 mb-1">
                                {def.part_of_speech && (
                                  <span className="px-2 py-0.5 bg-gray-200 text-gray-700 rounded text-sm font-medium">
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
                              <p className="text-lg text-gray-800">{def.definition}</p>
                              {def.usage_notes && (
                                <p className="text-sm text-gray-500 mt-1 italic">{def.usage_notes}</p>
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

              return (
                <Tabs defaultValue={hasSentences ? 'sentences' : 'collocations'}>
                  <TabsList className={`grid w-full max-w-md ${hasSentences && hasCollocations ? 'grid-cols-2' : 'grid-cols-1'}`}>
                    {hasSentences && <TabsTrigger value="sentences">使用例句</TabsTrigger>}
                    {hasCollocations && <TabsTrigger value="collocations">词汇搭配</TabsTrigger>}
                  </TabsList>
                  {hasSentences && (
                    <TabsContent value="sentences" className="mt-4">
                      <div className="bg-white rounded-2xl shadow-lg p-8">
                        <div className="space-y-3">
                          {(result.sentences ?? []).map((sentence) => (
                            <div key={sentence.id} className="bg-gray-50 rounded-lg p-4">
                              <p className="text-gray-800">{sentence.sentence}</p>
                              {sentence.source && (
                                <p className="text-gray-500 text-sm mt-1">— {sentence.source}</p>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    </TabsContent>
                  )}
                  {hasCollocations && (
                    <TabsContent value="collocations" className="mt-4">
                      <div className="bg-white rounded-2xl shadow-lg p-8">
                        <h3 className="text-2xl font-bold text-gray-900 mb-4">词汇搭配</h3>
                        <p className="text-gray-500 italic">词汇搭配功能开发中...</p>
                      </div>
                    </TabsContent>
                  )}
                </Tabs>
              )
            })()}

            {(result.etymologies?.length ?? 0) > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4">Etymology</h3>
                <div className="space-y-3">
                  {(result.etymologies ?? []).map((etym) => (
                    <div key={etym.id} className="bg-amber-50 rounded-lg p-4 border border-amber-200">
                      <p className="text-gray-800">{etym.etymology}</p>
                      {etym.origin_language && (
                        <p className="text-sm text-gray-500 mt-1">Origin: {etym.origin_language}</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {(result.categories?.length ?? 0) > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4">Categories</h3>
                <div className="flex flex-wrap gap-2">
                  {(result.categories ?? []).map((cat) => (
                    <span
                      key={cat.id}
                      className="px-3 py-1 bg-purple-100 text-purple-800 rounded-full text-sm"
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
    </div>
  )
}
