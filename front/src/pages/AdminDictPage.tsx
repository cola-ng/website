import { useState, useEffect } from 'react'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import {
  Search,
  Loader2,
  BookOpen,
  ArrowLeft,
  Plus,
  Trash2,
  Languages,
  Pencil,
  EyeOff,
  Eye,
  Volume2,
  Settings2,
} from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Input } from '../components/ui/input'
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog'
import { useAuth } from '../lib/auth'
import {
  listWords,
  lookup,
  createDefinition,
  updateDefinition,
  deleteDefinition,
  deleteWord,
  batchDeleteWords,
  batchUpdateWords,
  updateWord,
  getPronunciations,
  createPronunciation,
  updatePronunciation,
  deletePronunciation,
  type Word,
  type Definition,
  type Pronunciation,
  type WordQueryResponse,
  type CreateDefinitionRequest,
  type UpdateWordRequest,
  type CreatePronunciationRequest,
  type UpdatePronunciationRequest,
} from '../lib/api'

export function AdminDictPage() {
  const { token, user } = useAuth()
  const navigate = useNavigate()
  const { wordId } = useParams<{ wordId: string }>()

  // List view state
  const [words, setWords] = useState<Word[]>([])
  const [search, setSearch] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Detail view state
  const [wordDetail, setWordDetail] = useState<WordQueryResponse | null>(null)
  const [definitions, setDefinitions] = useState<Definition[]>([])
  const [detailLoading, setDetailLoading] = useState(false)

  // Dialog state
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [deleteWordDialogOpen, setDeleteWordDialogOpen] = useState(false)
  const [selectedDefinition, setSelectedDefinition] = useState<Definition | null>(null)
  const [actionLoading, setActionLoading] = useState(false)

  // Batch selection state (list view)
  const [selectedWordIds, setSelectedWordIds] = useState<Set<number>>(new Set())
  const [batchDeleteDialogOpen, setBatchDeleteDialogOpen] = useState(false)
  const [batchDeactivateDialogOpen, setBatchDeactivateDialogOpen] = useState(false)
  const [deleteListWordDialogOpen, setDeleteListWordDialogOpen] = useState(false)
  const [wordToDelete, setWordToDelete] = useState<Word | null>(null)

  // New definition form
  const [newDefinition, setNewDefinition] = useState<CreateDefinitionRequest>({
    language: 'zh',
    definition: '',
    part_of_speech: '',
  })

  // Edit word dialog state
  const [editWordDialogOpen, setEditWordDialogOpen] = useState(false)
  const [editWordForm, setEditWordForm] = useState<UpdateWordRequest>({
    frequency: undefined,
    word_type: '',
    syllable_count: undefined,
  })

  // Pronunciation state
  const [pronunciations, setPronunciations] = useState<Pronunciation[]>([])
  const [addPronDialogOpen, setAddPronDialogOpen] = useState(false)
  const [editPronDialogOpen, setEditPronDialogOpen] = useState(false)
  const [deletePronDialogOpen, setDeletePronDialogOpen] = useState(false)
  const [selectedPronunciation, setSelectedPronunciation] = useState<Pronunciation | null>(null)
  const [newPronunciation, setNewPronunciation] = useState<CreatePronunciationRequest>({
    ipa: '',
    dialect: 'US',
    gender: '',
    audio_url: '',
    is_primary: false,
  })
  const [playingAudioId, setPlayingAudioId] = useState<number | null>(null)

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  // Fetch words list
  useEffect(() => {
    if (token && !wordId) {
      setLoading(true)
      listWords(token, { search: search || undefined, limit: 50 })
        .then(setWords)
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load words'))
        .finally(() => setLoading(false))
    }
  }, [token, wordId, search])

  // Fetch word detail
  useEffect(() => {
    if (token && wordId) {
      setDetailLoading(true)
      const id = parseInt(wordId, 10)

      // First get the word from list to get its text
      listWords(token, { limit: 1000 })
        .then((allWords) => {
          const word = allWords.find((w) => w.id === id)
          if (word) {
            // Now lookup the full word detail
            return lookup(word.word).then((detail) => {
              setWordDetail(detail)
              setDefinitions(detail.definitions)
              // Also fetch pronunciations
              return getPronunciations(token, id).then(setPronunciations)
            })
          } else {
            throw new Error('Word not found')
          }
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load word'))
        .finally(() => setDetailLoading(false))
    }
  }, [token, wordId])

  const handleAddDefinition = async () => {
    if (!token || !wordDetail || !newDefinition.definition.trim()) return
    setActionLoading(true)
    try {
      const created = await createDefinition(token, wordDetail.word.id, {
        ...newDefinition,
        definition: newDefinition.definition.trim(),
        part_of_speech: newDefinition.part_of_speech || undefined,
      })
      setDefinitions([...definitions, created])
      setAddDialogOpen(false)
      setNewDefinition({ language: 'zh', definition: '', part_of_speech: '' })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to add definition')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteDefinition = async () => {
    if (!token || !wordDetail || !selectedDefinition) return
    setActionLoading(true)
    try {
      await deleteDefinition(token, wordDetail.word.id, selectedDefinition.id)
      setDefinitions(definitions.filter((d) => d.id !== selectedDefinition.id))
      setDeleteDialogOpen(false)
      setSelectedDefinition(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete definition')
    } finally {
      setActionLoading(false)
    }
  }

  const handleEditDefinition = async () => {
    if (!token || !wordDetail || !selectedDefinition || !newDefinition.definition.trim()) return
    setActionLoading(true)
    try {
      const updated = await updateDefinition(token, wordDetail.word.id, selectedDefinition.id, {
        language: newDefinition.language,
        definition: newDefinition.definition.trim(),
        part_of_speech: newDefinition.part_of_speech || undefined,
      })
      setDefinitions(definitions.map((d) => (d.id === updated.id ? updated : d)))
      setEditDialogOpen(false)
      setSelectedDefinition(null)
      setNewDefinition({ language: 'zh', definition: '', part_of_speech: '' })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update definition')
    } finally {
      setActionLoading(false)
    }
  }

  // Word properties editing
  const openEditWordDialog = () => {
    if (!wordDetail) return
    setEditWordForm({
      frequency: wordDetail.word.frequency ?? undefined,
      word_type: wordDetail.word.word_type || '',
      syllable_count: wordDetail.word.syllable_count ?? undefined,
    })
    setEditWordDialogOpen(true)
  }

  const handleEditWord = async () => {
    if (!token || !wordDetail) return
    setActionLoading(true)
    try {
      const updated = await updateWord(token, wordDetail.word.id, {
        frequency: editWordForm.frequency,
        word_type: editWordForm.word_type || undefined,
        syllable_count: editWordForm.syllable_count,
      })
      setWordDetail({ ...wordDetail, word: updated })
      setEditWordDialogOpen(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update word')
    } finally {
      setActionLoading(false)
    }
  }

  // Pronunciation CRUD
  const handleAddPronunciation = async () => {
    if (!token || !wordDetail || !newPronunciation.ipa.trim()) return
    setActionLoading(true)
    try {
      const created = await createPronunciation(token, wordDetail.word.id, {
        ipa: newPronunciation.ipa.trim(),
        dialect: newPronunciation.dialect || undefined,
        gender: newPronunciation.gender || undefined,
        audio_url: newPronunciation.audio_url || undefined,
        is_primary: newPronunciation.is_primary,
      })
      setPronunciations([...pronunciations, created])
      setAddPronDialogOpen(false)
      setNewPronunciation({ ipa: '', dialect: 'US', gender: '', audio_url: '', is_primary: false })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to add pronunciation')
    } finally {
      setActionLoading(false)
    }
  }

  const openEditPronDialog = (pron: Pronunciation) => {
    setSelectedPronunciation(pron)
    setNewPronunciation({
      ipa: pron.ipa,
      dialect: pron.dialect || 'US',
      gender: pron.gender || '',
      audio_url: pron.audio_url || '',
      is_primary: pron.is_primary ?? false,
    })
    setEditPronDialogOpen(true)
  }

  const handleEditPronunciation = async () => {
    if (!token || !wordDetail || !selectedPronunciation || !newPronunciation.ipa.trim()) return
    setActionLoading(true)
    try {
      const updated = await updatePronunciation(token, wordDetail.word.id, selectedPronunciation.id, {
        ipa: newPronunciation.ipa.trim(),
        dialect: newPronunciation.dialect || undefined,
        gender: newPronunciation.gender || undefined,
        audio_url: newPronunciation.audio_url || undefined,
        is_primary: newPronunciation.is_primary,
      })
      setPronunciations(pronunciations.map((p) => (p.id === updated.id ? updated : p)))
      setEditPronDialogOpen(false)
      setSelectedPronunciation(null)
      setNewPronunciation({ ipa: '', dialect: 'US', gender: '', audio_url: '', is_primary: false })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update pronunciation')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeletePronunciation = async () => {
    if (!token || !wordDetail || !selectedPronunciation) return
    setActionLoading(true)
    try {
      await deletePronunciation(token, wordDetail.word.id, selectedPronunciation.id)
      setPronunciations(pronunciations.filter((p) => p.id !== selectedPronunciation.id))
      setDeletePronDialogOpen(false)
      setSelectedPronunciation(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete pronunciation')
    } finally {
      setActionLoading(false)
    }
  }

  const playAudio = (pron: Pronunciation) => {
    const audioUrl = pron.audio_url || (pron.audio_path ? `/api/dict/pronunciations/${pron.id}/audio` : null)
    if (!audioUrl) return
    setPlayingAudioId(pron.id)
    const audio = new Audio(audioUrl)
    audio.onended = () => setPlayingAudioId(null)
    audio.onerror = () => setPlayingAudioId(null)
    audio.play().catch(() => setPlayingAudioId(null))
  }

  const handleDeleteWord = async () => {
    if (!token || !wordDetail) return
    setActionLoading(true)
    try {
      await deleteWord(token, wordDetail.word.id)
      setDeleteWordDialogOpen(false)
      navigate('/admin/dict')
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete word')
    } finally {
      setActionLoading(false)
    }
  }

  const openEditDialog = (def: Definition) => {
    setSelectedDefinition(def)
    setNewDefinition({
      language: def.language,
      definition: def.definition,
      part_of_speech: def.part_of_speech || '',
    })
    setEditDialogOpen(true)
  }

  // Batch operations for list view
  const toggleSelectWord = (wordId: number) => {
    const newSelected = new Set(selectedWordIds)
    if (newSelected.has(wordId)) {
      newSelected.delete(wordId)
    } else {
      newSelected.add(wordId)
    }
    setSelectedWordIds(newSelected)
  }

  const toggleSelectAll = () => {
    if (selectedWordIds.size === words.length) {
      setSelectedWordIds(new Set())
    } else {
      setSelectedWordIds(new Set(words.map((w) => w.id)))
    }
  }

  const handleBatchDelete = async () => {
    if (!token || selectedWordIds.size === 0) return
    setActionLoading(true)
    try {
      await batchDeleteWords(token, Array.from(selectedWordIds))
      setWords(words.filter((w) => !selectedWordIds.has(w.id)))
      setSelectedWordIds(new Set())
      setBatchDeleteDialogOpen(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete words')
    } finally {
      setActionLoading(false)
    }
  }

  const handleBatchDeactivate = async () => {
    if (!token || selectedWordIds.size === 0) return
    setActionLoading(true)
    try {
      await batchUpdateWords(token, Array.from(selectedWordIds), { is_active: false })
      setWords(words.map((w) => (selectedWordIds.has(w.id) ? { ...w, is_active: false } : w)))
      setSelectedWordIds(new Set())
      setBatchDeactivateDialogOpen(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to deactivate words')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteListWord = async () => {
    if (!token || !wordToDelete) return
    setActionLoading(true)
    try {
      await deleteWord(token, wordToDelete.id)
      setWords(words.filter((w) => w.id !== wordToDelete.id))
      setDeleteListWordDialogOpen(false)
      setWordToDelete(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete word')
    } finally {
      setActionLoading(false)
    }
  }

  // Detail view
  if (wordId) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />

        <main className="max-w-4xl mx-auto px-4 py-6">
          <Button
            variant="ghost"
            className="mb-4 text-gray-600 hover:text-gray-900"
            onClick={() => navigate('/admin/dict')}
          >
            <ArrowLeft className="h-4 w-4 mr-2" />
            返回词典列表
          </Button>

          {error && (
            <div className="mb-4 p-4 bg-red-50 text-red-600 rounded-lg">
              {error}
              <Button variant="ghost" size="sm" className="ml-2" onClick={() => setError(null)}>
                关闭
              </Button>
            </div>
          )}

          {detailLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
            </div>
          ) : wordDetail ? (
            <div className="space-y-6">
              {/* Word Header */}
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <BookOpen className="h-6 w-6 text-blue-600" />
                      <span className="text-2xl">{wordDetail.word.word}</span>
                      {wordDetail.word.difficulty && (
                        <span className="text-sm bg-gray-100 px-2 py-1 rounded">
                          难度: {wordDetail.word.difficulty}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={openEditWordDialog}
                      >
                        <Settings2 className="h-4 w-4 mr-1" />
                        编辑属性
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        className="text-red-600 border-red-200 hover:bg-red-50"
                        onClick={() => setDeleteWordDialogOpen(true)}
                      >
                        <Trash2 className="h-4 w-4 mr-1" />
                        删除单词
                      </Button>
                    </div>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                    <div>
                      <div className="text-gray-500 mb-1">ID</div>
                      <div className="font-mono">{wordDetail.word.id}</div>
                    </div>
                    <div>
                      <div className="text-gray-500 mb-1">词频</div>
                      <div>{wordDetail.word.frequency ?? '-'}</div>
                    </div>
                    <div>
                      <div className="text-gray-500 mb-1">词类</div>
                      <div>{wordDetail.word.word_type || '-'}</div>
                    </div>
                    <div>
                      <div className="text-gray-500 mb-1">音节数</div>
                      <div>{wordDetail.word.syllable_count ?? '-'}</div>
                    </div>
                  </div>
                </CardContent>
              </Card>

              {/* Definitions */}
              <Card>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base flex items-center gap-2">
                      <Languages className="h-4 w-4" /> 释义管理
                    </CardTitle>
                    <Button size="sm" onClick={() => setAddDialogOpen(true)}>
                      <Plus className="h-4 w-4 mr-1" />
                      添加释义
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  {definitions.length > 0 ? (
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead className="w-16">ID</TableHead>
                          <TableHead className="w-20">语言</TableHead>
                          <TableHead className="w-24">词性</TableHead>
                          <TableHead>释义</TableHead>
                          <TableHead className="w-20">操作</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {definitions.map((def) => (
                          <TableRow key={def.id}>
                            <TableCell className="font-mono text-sm">{def.id}</TableCell>
                            <TableCell>
                              <span
                                className={`px-2 py-0.5 rounded text-xs ${
                                  def.language === 'zh'
                                    ? 'bg-red-100 text-red-700'
                                    : 'bg-blue-100 text-blue-700'
                                }`}
                              >
                                {def.language === 'zh' ? '中文' : '英文'}
                              </span>
                            </TableCell>
                            <TableCell className="text-gray-600">
                              {def.part_of_speech || '-'}
                            </TableCell>
                            <TableCell>{def.definition}</TableCell>
                            <TableCell>
                              <div className="flex items-center gap-1">
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="text-gray-600 hover:text-gray-700 hover:bg-gray-50"
                                  onClick={() => openEditDialog(def)}
                                >
                                  <Pencil className="h-4 w-4" />
                                </Button>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="text-red-600 hover:text-red-700 hover:bg-red-50"
                                  onClick={() => {
                                    setSelectedDefinition(def)
                                    setDeleteDialogOpen(true)
                                  }}
                                >
                                  <Trash2 className="h-4 w-4" />
                                </Button>
                              </div>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  ) : (
                    <div className="text-gray-500 text-sm text-center py-8">暂无释义</div>
                  )}
                </CardContent>
              </Card>

              {/* Pronunciations */}
              <Card>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base flex items-center gap-2">
                      <Volume2 className="h-4 w-4" /> 发音管理
                    </CardTitle>
                    <Button size="sm" onClick={() => setAddPronDialogOpen(true)}>
                      <Plus className="h-4 w-4 mr-1" />
                      添加发音
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  {pronunciations.length > 0 ? (
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead className="w-16">ID</TableHead>
                          <TableHead>IPA</TableHead>
                          <TableHead className="w-20">口音</TableHead>
                          <TableHead className="w-20">性别</TableHead>
                          <TableHead className="w-20">音频</TableHead>
                          <TableHead className="w-20">主要</TableHead>
                          <TableHead className="w-24">操作</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {pronunciations.map((pron) => (
                          <TableRow key={pron.id}>
                            <TableCell className="font-mono text-sm">{pron.id}</TableCell>
                            <TableCell className="font-mono">{pron.ipa}</TableCell>
                            <TableCell>
                              <span className="px-2 py-0.5 rounded text-xs bg-blue-100 text-blue-700">
                                {pron.dialect || '-'}
                              </span>
                            </TableCell>
                            <TableCell className="text-gray-600">{pron.gender || '-'}</TableCell>
                            <TableCell>
                              {(pron.audio_url || pron.audio_path) ? (
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() => playAudio(pron)}
                                  disabled={playingAudioId === pron.id}
                                >
                                  <Volume2 className={`h-4 w-4 ${playingAudioId === pron.id ? 'animate-pulse text-blue-600' : ''}`} />
                                </Button>
                              ) : (
                                <span className="text-gray-400">-</span>
                              )}
                            </TableCell>
                            <TableCell>
                              {pron.is_primary ? (
                                <span className="text-green-600 text-xs">是</span>
                              ) : (
                                <span className="text-gray-400 text-xs">否</span>
                              )}
                            </TableCell>
                            <TableCell>
                              <div className="flex items-center gap-1">
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="text-gray-600 hover:text-gray-700 hover:bg-gray-50"
                                  onClick={() => openEditPronDialog(pron)}
                                >
                                  <Pencil className="h-4 w-4" />
                                </Button>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="text-red-600 hover:text-red-700 hover:bg-red-50"
                                  onClick={() => {
                                    setSelectedPronunciation(pron)
                                    setDeletePronDialogOpen(true)
                                  }}
                                >
                                  <Trash2 className="h-4 w-4" />
                                </Button>
                              </div>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  ) : (
                    <div className="text-gray-500 text-sm text-center py-8">暂无发音</div>
                  )}
                </CardContent>
              </Card>
            </div>
          ) : null}
        </main>

        {/* Add Definition Dialog */}
        <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>添加释义</DialogTitle>
              <DialogDescription>
                为单词 "{wordDetail?.word.word}" 添加新的释义
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">语言</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={newDefinition.language}
                  onChange={(e) => setNewDefinition({ ...newDefinition, language: e.target.value })}
                >
                  <option value="zh">中文</option>
                  <option value="en">英文</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">词性（可选）</label>
                <Input
                  placeholder="如：n. / v. / adj."
                  value={newDefinition.part_of_speech || ''}
                  onChange={(e) =>
                    setNewDefinition({ ...newDefinition, part_of_speech: e.target.value })
                  }
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">释义</label>
                <textarea
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm min-h-[100px]"
                  placeholder="输入释义..."
                  value={newDefinition.definition}
                  onChange={(e) => setNewDefinition({ ...newDefinition, definition: e.target.value })}
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setAddDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleAddDefinition}
                disabled={actionLoading || !newDefinition.definition.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                添加
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Definition Dialog */}
        <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除释义</DialogTitle>
              <DialogDescription>
                确定要删除这条释义吗？此操作不可撤销。
              </DialogDescription>
            </DialogHeader>
            {selectedDefinition && (
              <div className="py-4 bg-gray-50 rounded-lg p-4">
                <div className="text-sm text-gray-500 mb-1">
                  {selectedDefinition.language === 'zh' ? '中文' : '英文'}{' '}
                  {selectedDefinition.part_of_speech && `· ${selectedDefinition.part_of_speech}`}
                </div>
                <div>{selectedDefinition.definition}</div>
              </div>
            )}
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDeleteDefinition} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认删除
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Edit Definition Dialog */}
        <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>编辑释义</DialogTitle>
              <DialogDescription>
                修改单词 "{wordDetail?.word.word}" 的释义
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">语言</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={newDefinition.language}
                  onChange={(e) => setNewDefinition({ ...newDefinition, language: e.target.value })}
                >
                  <option value="zh">中文</option>
                  <option value="en">英文</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">词性（可选）</label>
                <Input
                  placeholder="如：n. / v. / adj."
                  value={newDefinition.part_of_speech || ''}
                  onChange={(e) =>
                    setNewDefinition({ ...newDefinition, part_of_speech: e.target.value })
                  }
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">释义</label>
                <textarea
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm min-h-[100px]"
                  placeholder="输入释义..."
                  value={newDefinition.definition}
                  onChange={(e) => setNewDefinition({ ...newDefinition, definition: e.target.value })}
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleEditDefinition}
                disabled={actionLoading || !newDefinition.definition.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                保存
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Word Dialog */}
        <Dialog open={deleteWordDialogOpen} onOpenChange={setDeleteWordDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除单词</DialogTitle>
              <DialogDescription>
                确定要删除单词 "{wordDetail?.word.word}" 吗？此操作将删除该单词及其所有释义，不可撤销。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeleteWordDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDeleteWord} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认删除
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Edit Word Properties Dialog */}
        <Dialog open={editWordDialogOpen} onOpenChange={setEditWordDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>编辑单词属性</DialogTitle>
              <DialogDescription>
                修改单词 "{wordDetail?.word.word}" 的词频、词类、音节数
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">词频 (0-10000)</label>
                <Input
                  type="number"
                  min="0"
                  max="10000"
                  placeholder="词频..."
                  value={editWordForm.frequency ?? ''}
                  onChange={(e) =>
                    setEditWordForm({
                      ...editWordForm,
                      frequency: e.target.value ? parseInt(e.target.value, 10) : undefined,
                    })
                  }
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">词类</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={editWordForm.word_type || ''}
                  onChange={(e) => setEditWordForm({ ...editWordForm, word_type: e.target.value })}
                >
                  <option value="">选择词类...</option>
                  <option value="noun">noun (名词)</option>
                  <option value="verb">verb (动词)</option>
                  <option value="adjective">adjective (形容词)</option>
                  <option value="adverb">adverb (副词)</option>
                  <option value="preposition">preposition (介词)</option>
                  <option value="conjunction">conjunction (连词)</option>
                  <option value="pronoun">pronoun (代词)</option>
                  <option value="interjection">interjection (感叹词)</option>
                  <option value="determiner">determiner (限定词)</option>
                  <option value="phrase">phrase (短语)</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">音节数 (1-20)</label>
                <Input
                  type="number"
                  min="1"
                  max="20"
                  placeholder="音节数..."
                  value={editWordForm.syllable_count ?? ''}
                  onChange={(e) =>
                    setEditWordForm({
                      ...editWordForm,
                      syllable_count: e.target.value ? parseInt(e.target.value, 10) : undefined,
                    })
                  }
                  className="mt-1"
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditWordDialogOpen(false)}>
                取消
              </Button>
              <Button onClick={handleEditWord} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                保存
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Add Pronunciation Dialog */}
        <Dialog open={addPronDialogOpen} onOpenChange={setAddPronDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>添加发音</DialogTitle>
              <DialogDescription>
                为单词 "{wordDetail?.word.word}" 添加新的发音
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">IPA 音标 *</label>
                <Input
                  placeholder="如: /həˈloʊ/"
                  value={newPronunciation.ipa}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, ipa: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">口音</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={newPronunciation.dialect || ''}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, dialect: e.target.value })}
                >
                  <option value="">选择口音...</option>
                  <option value="US">美式 (US)</option>
                  <option value="UK">英式 (UK)</option>
                  <option value="AU">澳式 (AU)</option>
                  <option value="CA">加拿大 (CA)</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">性别</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={newPronunciation.gender || ''}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, gender: e.target.value })}
                >
                  <option value="">选择性别...</option>
                  <option value="male">男声</option>
                  <option value="female">女声</option>
                  <option value="neutral">中性</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">音频 URL（可选）</label>
                <Input
                  placeholder="https://..."
                  value={newPronunciation.audio_url || ''}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, audio_url: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="is_primary_add"
                  className="h-4 w-4 rounded border-gray-300"
                  checked={newPronunciation.is_primary ?? false}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, is_primary: e.target.checked })}
                />
                <label htmlFor="is_primary_add" className="text-sm">设为主要发音</label>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setAddPronDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleAddPronunciation}
                disabled={actionLoading || !newPronunciation.ipa.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                添加
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Edit Pronunciation Dialog */}
        <Dialog open={editPronDialogOpen} onOpenChange={setEditPronDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>编辑发音</DialogTitle>
              <DialogDescription>
                修改单词 "{wordDetail?.word.word}" 的发音
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">IPA 音标 *</label>
                <Input
                  placeholder="如: /həˈloʊ/"
                  value={newPronunciation.ipa}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, ipa: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">口音</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={newPronunciation.dialect || ''}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, dialect: e.target.value })}
                >
                  <option value="">选择口音...</option>
                  <option value="US">美式 (US)</option>
                  <option value="UK">英式 (UK)</option>
                  <option value="AU">澳式 (AU)</option>
                  <option value="CA">加拿大 (CA)</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">性别</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={newPronunciation.gender || ''}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, gender: e.target.value })}
                >
                  <option value="">选择性别...</option>
                  <option value="male">男声</option>
                  <option value="female">女声</option>
                  <option value="neutral">中性</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">音频 URL（可选）</label>
                <Input
                  placeholder="https://..."
                  value={newPronunciation.audio_url || ''}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, audio_url: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="is_primary_edit"
                  className="h-4 w-4 rounded border-gray-300"
                  checked={newPronunciation.is_primary ?? false}
                  onChange={(e) => setNewPronunciation({ ...newPronunciation, is_primary: e.target.checked })}
                />
                <label htmlFor="is_primary_edit" className="text-sm">设为主要发音</label>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditPronDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleEditPronunciation}
                disabled={actionLoading || !newPronunciation.ipa.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                保存
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Pronunciation Dialog */}
        <Dialog open={deletePronDialogOpen} onOpenChange={setDeletePronDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除发音</DialogTitle>
              <DialogDescription>
                确定要删除这条发音吗？此操作不可撤销。
              </DialogDescription>
            </DialogHeader>
            {selectedPronunciation && (
              <div className="py-4 bg-gray-50 rounded-lg p-4">
                <div className="text-sm text-gray-500 mb-1">
                  {selectedPronunciation.dialect || '未知口音'}{' '}
                  {selectedPronunciation.gender && `· ${selectedPronunciation.gender}`}
                </div>
                <div className="font-mono">{selectedPronunciation.ipa}</div>
              </div>
            )}
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeletePronDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDeletePronunciation} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认删除
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    )
  }

  // List view
  return (
    <div className="min-h-screen bg-gray-50">
      <Header />

      <main className="max-w-6xl mx-auto px-4 py-6">
        {error && (
          <div className="mb-4 p-4 bg-red-50 text-red-600 rounded-lg">
            {error}
            <Button variant="ghost" size="sm" className="ml-2" onClick={() => setError(null)}>
              关闭
            </Button>
          </div>
        )}

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <BookOpen className="h-5 w-5" />
                词典管理
              </CardTitle>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  placeholder="搜索单词..."
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  className="pl-9 w-64"
                />
              </div>
            </div>
            {/* Batch Actions Bar */}
            {selectedWordIds.size > 0 && (
              <div className="mt-4 flex items-center gap-3 p-3 bg-blue-50 rounded-lg">
                <span className="text-sm text-blue-700">
                  已选择 {selectedWordIds.size} 个单词
                </span>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-red-600 border-red-200 hover:bg-red-50"
                    onClick={() => setBatchDeleteDialogOpen(true)}
                  >
                    <Trash2 className="h-4 w-4 mr-1" />
                    批量删除
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-orange-600 border-orange-200 hover:bg-orange-50"
                    onClick={() => setBatchDeactivateDialogOpen(true)}
                  >
                    <EyeOff className="h-4 w-4 mr-1" />
                    批量停用
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setSelectedWordIds(new Set())}
                  >
                    取消选择
                  </Button>
                </div>
              </div>
            )}
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
              </div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-12">
                      <input
                        type="checkbox"
                        className="h-4 w-4 rounded border-gray-300"
                        checked={words.length > 0 && selectedWordIds.size === words.length}
                        onChange={toggleSelectAll}
                      />
                    </TableHead>
                    <TableHead className="w-20">ID</TableHead>
                    <TableHead>单词</TableHead>
                    <TableHead className="w-20">状态</TableHead>
                    <TableHead className="w-24">难度</TableHead>
                    <TableHead className="w-24">词频</TableHead>
                    <TableHead className="w-32">创建时间</TableHead>
                    <TableHead className="w-28">操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {words.map((word) => (
                    <TableRow key={word.id} className={selectedWordIds.has(word.id) ? 'bg-blue-50' : ''}>
                      <TableCell>
                        <input
                          type="checkbox"
                          className="h-4 w-4 rounded border-gray-300"
                          checked={selectedWordIds.has(word.id)}
                          onChange={() => toggleSelectWord(word.id)}
                        />
                      </TableCell>
                      <TableCell className="font-mono text-sm">{word.id}</TableCell>
                      <TableCell>
                        <button
                          className="text-blue-600 hover:text-blue-800 hover:underline text-left font-medium"
                          onClick={() => navigate(`/admin/dict/${word.id}`)}
                        >
                          {word.word}
                        </button>
                      </TableCell>
                      <TableCell>
                        {word.is_active === false ? (
                          <span className="inline-flex items-center gap-1 text-xs text-gray-500">
                            <EyeOff className="h-3 w-3" /> 停用
                          </span>
                        ) : (
                          <span className="inline-flex items-center gap-1 text-xs text-green-600">
                            <Eye className="h-3 w-3" /> 启用
                          </span>
                        )}
                      </TableCell>
                      <TableCell>
                        {word.difficulty ? (
                          <span className="px-2 py-0.5 bg-gray-100 rounded text-sm">
                            {word.difficulty}
                          </span>
                        ) : (
                          '-'
                        )}
                      </TableCell>
                      <TableCell>{word.frequency ?? '-'}</TableCell>
                      <TableCell className="text-sm text-gray-500">
                        {new Date(word.created_at).toLocaleDateString()}
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => navigate(`/admin/dict/${word.id}`)}
                          >
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="text-red-600 hover:text-red-700 hover:bg-red-50"
                            onClick={() => {
                              setWordToDelete(word)
                              setDeleteListWordDialogOpen(true)
                            }}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}

            {!loading && words.length === 0 && (
              <div className="text-center py-12 text-gray-500">
                {search ? '未找到匹配的单词' : '暂无单词数据'}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Batch Delete Dialog */}
        <Dialog open={batchDeleteDialogOpen} onOpenChange={setBatchDeleteDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>批量删除单词</DialogTitle>
              <DialogDescription>
                确定要删除选中的 {selectedWordIds.size} 个单词吗？此操作将删除这些单词及其所有释义，不可撤销。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setBatchDeleteDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleBatchDelete} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认删除 ({selectedWordIds.size})
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Batch Deactivate Dialog */}
        <Dialog open={batchDeactivateDialogOpen} onOpenChange={setBatchDeactivateDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>批量停用单词</DialogTitle>
              <DialogDescription>
                确定要停用选中的 {selectedWordIds.size} 个单词吗？停用后这些单词将不会在词典中显示。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setBatchDeactivateDialogOpen(false)}>
                取消
              </Button>
              <Button onClick={handleBatchDeactivate} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认停用 ({selectedWordIds.size})
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Single Word Dialog (from list) */}
        <Dialog open={deleteListWordDialogOpen} onOpenChange={setDeleteListWordDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除单词</DialogTitle>
              <DialogDescription>
                确定要删除单词 "{wordToDelete?.word}" 吗？此操作将删除该单词及其所有释义，不可撤销。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeleteListWordDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDeleteListWord} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认删除
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </main>
    </div>
  )
}
