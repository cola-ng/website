import { useState } from 'react'
import { Search, BookOpen, Volume2, Star, ChevronRight } from 'lucide-react'
import { lookup, type WordQueryResponse } from '../lib/api'

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
                <div>
                  <h2 className="text-4xl font-bold text-gray-900 mb-2">{result.word.word}</h2>
                  <div className="flex flex-wrap gap-3 text-gray-600">
                    {result.word.phonetic_us && (
                      <span className="flex items-center gap-1">
                        US: /{result.word.phonetic_us}/
                        {result.word.audio_us && (
                          <button
                            onClick={() => playAudio(result.word.audio_us)}
                            className="text-indigo-600 hover:text-indigo-800"
                          >
                            <Volume2 className="w-4 h-4" />
                          </button>
                        )}
                      </span>
                    )}
                    {result.word.phonetic_uk && (
                      <span className="flex items-center gap-1">
                        UK: /{result.word.phonetic_uk}/
                        {result.word.audio_uk && (
                          <button
                            onClick={() => playAudio(result.word.audio_uk)}
                            className="text-indigo-600 hover:text-indigo-800"
                          >
                            <Volume2 className="w-4 h-4" />
                          </button>
                        )}
                      </span>
                    )}
                  </div>
                </div>
                <div className="flex flex-wrap gap-2">
                  {result.word.difficulty_level && (
                    <span className="px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-sm font-medium">
                      {result.word.difficulty_level}
                    </span>
                  )}
                  {result.word.core_level && (
                    <span className="px-3 py-1 bg-purple-100 text-purple-800 rounded-full text-sm font-medium">
                      {result.word.core_level}
                    </span>
                  )}
                  {result.word.frequency_rank && (
                    <span className="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm font-medium">
                      Rank #{result.word.frequency_rank}
                    </span>
                  )}
                </div>
              </div>
            </div>

            {result.definitions.length > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4 flex items-center gap-2">
                  <Star className="w-6 h-6 text-yellow-500" />
                  Definitions
                </h3>
                <div className="space-y-4">
                  {result.definitions.map((def) => (
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
                      <p className="text-lg text-gray-800">{def.definition_en}</p>
                      {def.definition_zh && (
                        <p className="text-gray-600">{def.definition_zh}</p>
                      )}
                      {def.usage_notes && (
                        <p className="text-sm text-gray-500 mt-1 italic">{def.usage_notes}</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {result.examples.length > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4 flex items-center gap-2">
                  <ChevronRight className="w-6 h-6 text-indigo-500" />
                  Examples
                </h3>
                <div className="space-y-3">
                  {result.examples.map((example) => (
                    <div key={example.id} className="bg-gray-50 rounded-lg p-4">
                      <p className="text-gray-800">{example.example_en}</p>
                      {example.example_zh && (
                        <p className="text-gray-600 mt-1">{example.example_zh}</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div className="grid md:grid-cols-2 gap-6">
              {result.synonyms.length > 0 && (
                <div className="bg-white rounded-2xl shadow-lg p-6">
                  <h3 className="text-xl font-bold text-gray-900 mb-4">Synonyms</h3>
                  <div className="flex flex-wrap gap-2">
                    {result.synonyms.map((syn) => (
                      <span
                        key={syn.id}
                        className="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm cursor-pointer hover:bg-green-200 transition-colors"
                        onClick={() => setQuery(syn.synonym)}
                      >
                        {syn.synonym}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {result.antonyms.length > 0 && (
                <div className="bg-white rounded-2xl shadow-lg p-6">
                  <h3 className="text-xl font-bold text-gray-900 mb-4">Antonyms</h3>
                  <div className="flex flex-wrap gap-2">
                    {result.antonyms.map((ant) => (
                      <span
                        key={ant.id}
                        className="px-3 py-1 bg-red-100 text-red-800 rounded-full text-sm cursor-pointer hover:bg-red-200 transition-colors"
                        onClick={() => setQuery(ant.antonym)}
                      >
                        {ant.antonym}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {result.collocations.length > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4">Collocations</h3>
                <div className="space-y-3">
                  {result.collocations.map((col) => (
                    <div key={col.id} className="border-l-4 border-blue-500 pl-4">
                      <p className="text-lg font-medium text-gray-800">
                        <span className="text-indigo-600 font-bold">{result.word.word}</span> {col.collocation}
                      </p>
                      {col.example_en && (
                        <p className="text-gray-600 mt-1">{col.example_en}</p>
                      )}
                      {col.example_zh && (
                        <p className="text-gray-500">{col.example_zh}</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {result.phrases.length > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4">Phrases</h3>
                <div className="space-y-3">
                  {result.phrases.map((phrase) => (
                    <div key={phrase.id} className="bg-gradient-to-r from-purple-50 to-pink-50 rounded-lg p-4">
                      <p className="text-lg font-medium text-purple-900">{phrase.phrase}</p>
                      {phrase.meaning_zh && (
                        <p className="text-gray-700 mt-1">{phrase.meaning_zh}</p>
                      )}
                      {phrase.example_en && (
                        <p className="text-gray-600 mt-2">{phrase.example_en}</p>
                      )}
                      {phrase.example_zh && (
                        <p className="text-gray-500">{phrase.example_zh}</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {result.common_errors.length > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4 flex items-center gap-2">
                  <span className="text-2xl">⚠️</span>
                  Common Errors
                </h3>
                <div className="space-y-4">
                  {result.common_errors.map((error) => (
                    <div key={error.id} className="border-l-4 border-red-500 pl-4 bg-red-50 rounded-r-lg p-4">
                      <p className="font-medium text-red-900 mb-2">{error.error_type}</p>
                      {error.error_example && (
                        <p className="text-red-700">
                          <span className="font-semibold">Wrong:</span> {error.error_example}
                        </p>
                      )}
                      {error.correct_example && (
                        <p className="text-green-700 mt-1">
                          <span className="font-semibold">Correct:</span> {error.correct_example}
                        </p>
                      )}
                      {error.explanation && (
                        <p className="text-gray-600 mt-2 italic">{error.explanation}</p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {result.roots.length > 0 && (
              <div className="bg-white rounded-2xl shadow-lg p-8">
                <h3 className="text-2xl font-bold text-gray-900 mb-4">Word Roots</h3>
                <div className="grid md:grid-cols-2 gap-4">
                  {result.roots.map((root) => (
                    <div key={root.id} className="bg-gradient-to-r from-amber-50 to-yellow-50 rounded-lg p-4 border border-amber-200">
                      <p className="text-lg font-bold text-amber-900">{root.root}</p>
                      {root.meaning && (
                        <p className="text-gray-700">{root.meaning}</p>
                      )}
                      {root.language && (
                        <p className="text-sm text-gray-500 mt-1">Origin: {root.language}</p>
                      )}
                    </div>
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
