import { useState, useEffect } from 'react'
import { Navigate } from 'react-router-dom'
import {
  Loader2,
  Plus,
  Trash2,
  Pencil,
  BookOpen,
  Volume2,
  ChevronRight,
  ChevronLeft,
  ArrowLeft,
  Search,
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
import { getAdminPerPage, setAdminPerPage, PER_PAGE_OPTIONS } from '../lib/utils'
import {
  listAdminReadSubjects,
  createAdminReadSubject,
  updateAdminReadSubject,
  deleteAdminReadSubject,
  listAdminReadSentences,
  createAdminReadSentence,
  updateAdminReadSentence,
  deleteAdminReadSentence,
  type ReadSubject,
  type ReadSentence,
  type CreateReadSubjectRequest,
  type CreateReadSentenceRequest,
} from '../lib/api'

export function AdminReadPage() {
  const { token, user } = useAuth()

  // View state: 'subjects' or 'sentences'
  const [view, setView] = useState<'subjects' | 'sentences'>('subjects')
  const [selectedSubject, setSelectedSubject] = useState<ReadSubject | null>(null)

  // Subject state
  const [subjects, setSubjects] = useState<ReadSubject[]>([])
  const [subjectsLoading, setSubjectsLoading] = useState(false)

  // Subject pagination state
  const [subjectsPage, setSubjectsPage] = useState(1)
  const [subjectsTotal, setSubjectsTotal] = useState(0)
  const [subjectsSearch, setSubjectsSearch] = useState('')
  const [subjectsSearchInput, setSubjectsSearchInput] = useState('')

  // Sentence state
  const [sentences, setSentences] = useState<ReadSentence[]>([])
  const [sentencesLoading, setSentencesLoading] = useState(false)

  // Sentence pagination state
  const [sentencesPage, setSentencesPage] = useState(1)
  const [sentencesTotal, setSentencesTotal] = useState(0)
  const [sentencesSearch, setSentencesSearch] = useState('')
  const [sentencesSearchInput, setSentencesSearchInput] = useState('')

  const [perPage, setPerPage] = useState(() => getAdminPerPage())

  // Error state
  const [error, setError] = useState<string | null>(null)

  // Subject dialog states
  const [addSubjectDialogOpen, setAddSubjectDialogOpen] = useState(false)
  const [editSubjectDialogOpen, setEditSubjectDialogOpen] = useState(false)
  const [deleteSubjectDialogOpen, setDeleteSubjectDialogOpen] = useState(false)
  const [subjectToEdit, setSubjectToEdit] = useState<ReadSubject | null>(null)
  const [subjectForm, setSubjectForm] = useState<CreateReadSubjectRequest>({
    code: '',
    title_en: '',
    title_zh: '',
    description_en: '',
    description_zh: '',
    icon_emoji: '📖',
    difficulty: 1,
    subject_type: 'general',
  })

  // Sentence dialog states
  const [addSentenceDialogOpen, setAddSentenceDialogOpen] = useState(false)
  const [editSentenceDialogOpen, setEditSentenceDialogOpen] = useState(false)
  const [deleteSentenceDialogOpen, setDeleteSentenceDialogOpen] = useState(false)
  const [sentenceToEdit, setSentenceToEdit] = useState<ReadSentence | null>(null)
  const [sentenceForm, setSentenceForm] = useState<CreateReadSentenceRequest>({
    subject_id: 0,
    sentence_order: 0,
    content_en: '',
    content_zh: '',
    phonetic_transcription: '',
    audio_path: '',
    difficulty: 1,
  })

  const [actionLoading, setActionLoading] = useState(false)

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  const subjectsTotalPages = Math.ceil(subjectsTotal / perPage)
  const sentencesTotalPages = Math.ceil(sentencesTotal / perPage)

  // Fetch subjects
  useEffect(() => {
    if (token) {
      setSubjectsLoading(true)
      listAdminReadSubjects(token, subjectsPage, perPage, subjectsSearch || undefined)
        .then((res) => {
          setSubjects(res.subjects)
          setSubjectsTotal(res.total)
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load subjects'))
        .finally(() => setSubjectsLoading(false))
    }
  }, [token, subjectsPage, perPage, subjectsSearch])

  // Fetch sentences when a subject is selected
  useEffect(() => {
    if (token && selectedSubject) {
      setSentencesLoading(true)
      listAdminReadSentences(token, sentencesPage, perPage, selectedSubject.id, sentencesSearch || undefined)
        .then((res) => {
          setSentences(res.sentences)
          setSentencesTotal(res.total)
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load sentences'))
        .finally(() => setSentencesLoading(false))
    }
  }, [token, selectedSubject, sentencesPage, perPage, sentencesSearch])

  const refreshSubjects = () => {
    if (token) {
      listAdminReadSubjects(token, subjectsPage, perPage, subjectsSearch || undefined)
        .then((res) => {
          setSubjects(res.subjects)
          setSubjectsTotal(res.total)
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load subjects'))
    }
  }

  const refreshSentences = () => {
    if (token && selectedSubject) {
      listAdminReadSentences(token, sentencesPage, perPage, selectedSubject.id, sentencesSearch || undefined)
        .then((res) => {
          setSentences(res.sentences)
          setSentencesTotal(res.total)
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load sentences'))
    }
  }

  const handleSubjectsSearch = () => {
    setSubjectsPage(1)
    setSubjectsSearch(subjectsSearchInput)
  }

  const handleSubjectsSearchKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSubjectsSearch()
    }
  }

  const handleSentencesSearch = () => {
    setSentencesPage(1)
    setSentencesSearch(sentencesSearchInput)
  }

  const handleSentencesSearchKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSentencesSearch()
    }
  }

  const handlePerPageChange = (value: number) => {
    setPerPage(value)
    setAdminPerPage(value)
    setSubjectsPage(1)
    setSentencesPage(1)
  }

  // Subject handlers
  const handleOpenAddSubject = () => {
    setSubjectForm({
      code: '',
      title_en: '',
      title_zh: '',
      description_en: '',
      description_zh: '',
      icon_emoji: '📖',
      difficulty: 1,
      subject_type: 'general',
    })
    setAddSubjectDialogOpen(true)
  }

  const handleOpenEditSubject = (subject: ReadSubject) => {
    setSubjectToEdit(subject)
    setSubjectForm({
      code: subject.code,
      title_en: subject.title_en,
      title_zh: subject.title_zh,
      description_en: subject.description_en || '',
      description_zh: subject.description_zh || '',
      icon_emoji: subject.icon_emoji || '📖',
      difficulty: subject.difficulty || 1,
      subject_type: subject.subject_type || 'general',
    })
    setEditSubjectDialogOpen(true)
  }

  const handleOpenDeleteSubject = (subject: ReadSubject) => {
    setSubjectToEdit(subject)
    setDeleteSubjectDialogOpen(true)
  }

  const handleCreateSubject = async () => {
    if (!token) return
    setActionLoading(true)
    try {
      await createAdminReadSubject(token, subjectForm)
      setAddSubjectDialogOpen(false)
      refreshSubjects()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create subject')
    } finally {
      setActionLoading(false)
    }
  }

  const handleUpdateSubject = async () => {
    if (!token || !subjectToEdit) return
    setActionLoading(true)
    try {
      await updateAdminReadSubject(token, subjectToEdit.id, subjectForm)
      setEditSubjectDialogOpen(false)
      refreshSubjects()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update subject')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteSubject = async () => {
    if (!token || !subjectToEdit) return
    setActionLoading(true)
    try {
      await deleteAdminReadSubject(token, subjectToEdit.id)
      setDeleteSubjectDialogOpen(false)
      refreshSubjects()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete subject')
    } finally {
      setActionLoading(false)
    }
  }

  // Sentence handlers
  const handleViewSentences = (subject: ReadSubject) => {
    setSelectedSubject(subject)
    setSentencesPage(1)
    setSentencesSearch('')
    setSentencesSearchInput('')
    setView('sentences')
  }

  const handleBackToSubjects = () => {
    setSelectedSubject(null)
    setView('subjects')
    setSentences([])
  }

  const handleOpenAddSentence = () => {
    if (!selectedSubject) return
    setSentenceForm({
      subject_id: selectedSubject.id,
      sentence_order: sentences.length + 1,
      content_en: '',
      content_zh: '',
      phonetic_transcription: '',
      audio_path: '',
      difficulty: 1,
    })
    setAddSentenceDialogOpen(true)
  }

  const handleOpenEditSentence = (sentence: ReadSentence) => {
    setSentenceToEdit(sentence)
    setSentenceForm({
      subject_id: sentence.subject_id,
      sentence_order: sentence.sentence_order,
      content_en: sentence.content_en,
      content_zh: sentence.content_zh,
      phonetic_transcription: sentence.phonetic_transcription || '',
      audio_path: sentence.audio_path || '',
      difficulty: sentence.difficulty || 1,
    })
    setEditSentenceDialogOpen(true)
  }

  const handleOpenDeleteSentence = (sentence: ReadSentence) => {
    setSentenceToEdit(sentence)
    setDeleteSentenceDialogOpen(true)
  }

  const handleCreateSentence = async () => {
    if (!token) return
    setActionLoading(true)
    try {
      await createAdminReadSentence(token, sentenceForm)
      setAddSentenceDialogOpen(false)
      refreshSentences()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create sentence')
    } finally {
      setActionLoading(false)
    }
  }

  const handleUpdateSentence = async () => {
    if (!token || !sentenceToEdit) return
    setActionLoading(true)
    try {
      await updateAdminReadSentence(token, sentenceToEdit.id, sentenceForm)
      setEditSentenceDialogOpen(false)
      refreshSentences()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update sentence')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteSentence = async () => {
    if (!token || !sentenceToEdit) return
    setActionLoading(true)
    try {
      await deleteAdminReadSentence(token, sentenceToEdit.id)
      setDeleteSentenceDialogOpen(false)
      refreshSentences()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete sentence')
    } finally {
      setActionLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Header />

      <main className="max-w-6xl mx-auto px-4 py-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            {view === 'sentences' && (
              <Button variant="ghost" size="icon" onClick={handleBackToSubjects}>
                <ArrowLeft className="h-5 w-5" />
              </Button>
            )}
            <BookOpen className="h-8 w-8 text-blue-600" />
            <div>
              <h1 className="text-2xl font-bold text-gray-900">
                {view === 'subjects' ? '跟读管理' : `${selectedSubject?.title_zh || '跟读'} - 句子管理`}
              </h1>
              <p className="text-sm text-gray-500">
                {view === 'subjects' ? '管理跟读主题和句子' : `共 ${sentences.length} 个句子`}
              </p>
            </div>
          </div>
          {view === 'subjects' ? (
            <Button onClick={handleOpenAddSubject}>
              <Plus className="h-4 w-4 mr-2" />
              添加主题
            </Button>
          ) : (
            <Button onClick={handleOpenAddSentence}>
              <Plus className="h-4 w-4 mr-2" />
              添加句子
            </Button>
          )}
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-50 text-red-600 rounded-lg">
            {error}
            <Button variant="ghost" size="sm" className="ml-2" onClick={() => setError(null)}>
              关闭
            </Button>
          </div>
        )}

        {/* Subjects View */}
        {view === 'subjects' && (
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>跟读主题列表</CardTitle>
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-gray-500">每页</span>
                    <select
                      value={perPage}
                      onChange={(e) => handlePerPageChange(Number(e.target.value))}
                      className="h-9 px-2 border rounded-md text-sm bg-white"
                    >
                      {PER_PAGE_OPTIONS.map((opt) => (
                        <option key={opt} value={opt}>{opt} 条</option>
                      ))}
                    </select>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      placeholder="搜索标题或代码..."
                      value={subjectsSearchInput}
                      onChange={(e) => setSubjectsSearchInput(e.target.value)}
                      onKeyDown={handleSubjectsSearchKeyDown}
                      className="w-64"
                    />
                    <Button variant="outline" size="icon" onClick={handleSubjectsSearch}>
                      <Search className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {subjectsLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                </div>
              ) : (
                <>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-12">图标</TableHead>
                      <TableHead>标题</TableHead>
                      <TableHead>代码</TableHead>
                      <TableHead className="w-20">类型</TableHead>
                      <TableHead className="w-16">难度</TableHead>
                      <TableHead className="w-32 text-right whitespace-nowrap">操作</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {subjects.map((subject) => (
                      <TableRow key={subject.id}>
                        <TableCell className="text-2xl">{subject.icon_emoji || '📖'}</TableCell>
                        <TableCell>
                          <div className="font-medium">{subject.title_zh}</div>
                          <div className="text-sm text-gray-500">{subject.title_en}</div>
                        </TableCell>
                        <TableCell className="font-mono text-sm">{subject.code}</TableCell>
                        <TableCell>
                          <span className="inline-flex items-center px-2 py-1 rounded-full text-xs bg-blue-100 text-blue-700">
                            {subject.subject_type || 'general'}
                          </span>
                        </TableCell>
                        <TableCell>
                          <span className="inline-flex items-center px-2 py-1 rounded-full text-xs bg-purple-100 text-purple-700">
                            L{subject.difficulty || 1}
                          </span>
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          <Button
                            variant="ghost"
                            size="sm"
                            title="查看句子"
                            onClick={() => handleViewSentences(subject)}
                          >
                            句子 <ChevronRight className="h-4 w-4 ml-1" />
                          </Button>
                          <Button variant="ghost" size="icon" title="编辑" onClick={() => handleOpenEditSubject(subject)}>
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button variant="ghost" size="icon" title="删除" onClick={() => handleOpenDeleteSubject(subject)}>
                            <Trash2 className="h-4 w-4 text-red-500" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                    {subjects.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={6} className="text-center py-8 text-gray-500">
                          暂无主题数据
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>

                {/* Pagination */}
                <div className="flex items-center justify-between mt-4">
                  <div className="text-sm text-gray-500">
                    共 {subjectsTotal} 条记录
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setSubjectsPage((p) => Math.max(1, p - 1))}
                      disabled={subjectsPage === 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                    </Button>
                    <span className="text-sm">
                      {subjectsPage} / {subjectsTotalPages || 1}
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setSubjectsPage((p) => Math.min(subjectsTotalPages, p + 1))}
                      disabled={subjectsPage >= subjectsTotalPages}
                    >
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
                </>
              )}
            </CardContent>
          </Card>
        )}

        {/* Sentences View */}
        {view === 'sentences' && (
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>句子列表</CardTitle>
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-gray-500">每页</span>
                    <select
                      value={perPage}
                      onChange={(e) => handlePerPageChange(Number(e.target.value))}
                      className="h-9 px-2 border rounded-md text-sm bg-white"
                    >
                      {PER_PAGE_OPTIONS.map((opt) => (
                        <option key={opt} value={opt}>{opt} 条</option>
                      ))}
                    </select>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      placeholder="搜索句子内容..."
                      value={sentencesSearchInput}
                      onChange={(e) => setSentencesSearchInput(e.target.value)}
                      onKeyDown={handleSentencesSearchKeyDown}
                      className="w-64"
                    />
                    <Button variant="outline" size="icon" onClick={handleSentencesSearch}>
                      <Search className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {sentencesLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                </div>
              ) : (
                <>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-12">#</TableHead>
                      <TableHead>英文</TableHead>
                      <TableHead>中文</TableHead>
                      <TableHead className="w-16">难度</TableHead>
                      <TableHead className="w-16">音频</TableHead>
                      <TableHead className="w-24 text-right whitespace-nowrap">操作</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {sentences.map((sentence) => (
                      <TableRow key={sentence.id}>
                        <TableCell className="font-mono text-sm text-gray-500">
                          {sentence.sentence_order}
                        </TableCell>
                        <TableCell>
                          <div className="font-medium">{sentence.content_en}</div>
                          {sentence.phonetic_transcription && (
                            <div className="text-xs text-gray-400">{sentence.phonetic_transcription}</div>
                          )}
                        </TableCell>
                        <TableCell className="text-gray-600">{sentence.content_zh}</TableCell>
                        <TableCell>
                          <span className="inline-flex items-center px-2 py-1 rounded-full text-xs bg-purple-100 text-purple-700">
                            L{sentence.difficulty || 1}
                          </span>
                        </TableCell>
                        <TableCell>
                          {sentence.audio_path ? (
                            <Volume2 className="h-4 w-4 text-green-500" title="有音频" />
                          ) : (
                            <span className="text-xs text-gray-400">无</span>
                          )}
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          <Button variant="ghost" size="icon" title="编辑" onClick={() => handleOpenEditSentence(sentence)}>
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button variant="ghost" size="icon" title="删除" onClick={() => handleOpenDeleteSentence(sentence)}>
                            <Trash2 className="h-4 w-4 text-red-500" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                    {sentences.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={6} className="text-center py-8 text-gray-500">
                          暂无句子数据
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>

                {/* Pagination */}
                <div className="flex items-center justify-between mt-4">
                  <div className="text-sm text-gray-500">
                    共 {sentencesTotal} 条记录
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setSentencesPage((p) => Math.max(1, p - 1))}
                      disabled={sentencesPage === 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                    </Button>
                    <span className="text-sm">
                      {sentencesPage} / {sentencesTotalPages || 1}
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setSentencesPage((p) => Math.min(sentencesTotalPages, p + 1))}
                      disabled={sentencesPage >= sentencesTotalPages}
                    >
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
                </>
              )}
            </CardContent>
          </Card>
        )}
      </main>

      {/* Add Subject Dialog */}
      <Dialog open={addSubjectDialogOpen} onOpenChange={setAddSubjectDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>添加跟读主题</DialogTitle>
            <DialogDescription>创建一个新的跟读主题</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">代码</label>
                <Input
                  value={subjectForm.code}
                  onChange={(e) => setSubjectForm({ ...subjectForm, code: e.target.value })}
                  placeholder="e.g., daily-greetings"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">图标</label>
                <Input
                  value={subjectForm.icon_emoji}
                  onChange={(e) => setSubjectForm({ ...subjectForm, icon_emoji: e.target.value })}
                  placeholder="e.g., 👋"
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">中文标题</label>
                <Input
                  value={subjectForm.title_zh}
                  onChange={(e) => setSubjectForm({ ...subjectForm, title_zh: e.target.value })}
                  placeholder="e.g., 日常问候"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">英文标题</label>
                <Input
                  value={subjectForm.title_en}
                  onChange={(e) => setSubjectForm({ ...subjectForm, title_en: e.target.value })}
                  placeholder="e.g., Daily Greetings"
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">类型</label>
                <Input
                  value={subjectForm.subject_type}
                  onChange={(e) => setSubjectForm({ ...subjectForm, subject_type: e.target.value })}
                  placeholder="e.g., general, business"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">难度 (1-5)</label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={subjectForm.difficulty}
                  onChange={(e) => setSubjectForm({ ...subjectForm, difficulty: parseInt(e.target.value) || 1 })}
                />
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">中文描述</label>
              <Input
                value={subjectForm.description_zh}
                onChange={(e) => setSubjectForm({ ...subjectForm, description_zh: e.target.value })}
                placeholder="主题描述（中文）"
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">英文描述</label>
              <Input
                value={subjectForm.description_en}
                onChange={(e) => setSubjectForm({ ...subjectForm, description_en: e.target.value })}
                placeholder="Subject description (English)"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddSubjectDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleCreateSubject} disabled={actionLoading || !subjectForm.code || !subjectForm.title_zh}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              创建
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Subject Dialog */}
      <Dialog open={editSubjectDialogOpen} onOpenChange={setEditSubjectDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>编辑跟读主题</DialogTitle>
            <DialogDescription>修改跟读主题信息</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">代码</label>
                <Input
                  value={subjectForm.code}
                  onChange={(e) => setSubjectForm({ ...subjectForm, code: e.target.value })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">图标</label>
                <Input
                  value={subjectForm.icon_emoji}
                  onChange={(e) => setSubjectForm({ ...subjectForm, icon_emoji: e.target.value })}
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">中文标题</label>
                <Input
                  value={subjectForm.title_zh}
                  onChange={(e) => setSubjectForm({ ...subjectForm, title_zh: e.target.value })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">英文标题</label>
                <Input
                  value={subjectForm.title_en}
                  onChange={(e) => setSubjectForm({ ...subjectForm, title_en: e.target.value })}
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">类型</label>
                <Input
                  value={subjectForm.subject_type}
                  onChange={(e) => setSubjectForm({ ...subjectForm, subject_type: e.target.value })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">难度 (1-5)</label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={subjectForm.difficulty}
                  onChange={(e) => setSubjectForm({ ...subjectForm, difficulty: parseInt(e.target.value) || 1 })}
                />
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">中文描述</label>
              <Input
                value={subjectForm.description_zh}
                onChange={(e) => setSubjectForm({ ...subjectForm, description_zh: e.target.value })}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">英文描述</label>
              <Input
                value={subjectForm.description_en}
                onChange={(e) => setSubjectForm({ ...subjectForm, description_en: e.target.value })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditSubjectDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleUpdateSubject} disabled={actionLoading}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Subject Dialog */}
      <Dialog open={deleteSubjectDialogOpen} onOpenChange={setDeleteSubjectDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>删除跟读主题</DialogTitle>
            <DialogDescription>
              确定要删除主题 "{subjectToEdit?.title_zh}" 吗？此操作不可撤销，所有相关句子也将被删除。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteSubjectDialogOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleDeleteSubject} disabled={actionLoading}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              删除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Sentence Dialog */}
      <Dialog open={addSentenceDialogOpen} onOpenChange={setAddSentenceDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>添加句子</DialogTitle>
            <DialogDescription>为 "{selectedSubject?.title_zh}" 添加一个新句子</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">顺序</label>
                <Input
                  type="number"
                  value={sentenceForm.sentence_order}
                  onChange={(e) => setSentenceForm({ ...sentenceForm, sentence_order: parseInt(e.target.value) || 0 })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">难度 (1-5)</label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={sentenceForm.difficulty}
                  onChange={(e) => setSentenceForm({ ...sentenceForm, difficulty: parseInt(e.target.value) || 1 })}
                />
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">英文句子</label>
              <textarea
                className="w-full mt-1 px-3 py-2 border rounded-md text-sm"
                rows={2}
                value={sentenceForm.content_en}
                onChange={(e) => setSentenceForm({ ...sentenceForm, content_en: e.target.value })}
                placeholder="Hello, how are you today?"
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">中文翻译</label>
              <textarea
                className="w-full mt-1 px-3 py-2 border rounded-md text-sm"
                rows={2}
                value={sentenceForm.content_zh}
                onChange={(e) => setSentenceForm({ ...sentenceForm, content_zh: e.target.value })}
                placeholder="你好，你今天怎么样？"
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">音标</label>
              <Input
                value={sentenceForm.phonetic_transcription}
                onChange={(e) => setSentenceForm({ ...sentenceForm, phonetic_transcription: e.target.value })}
                placeholder="/həˈloʊ, haʊ ɑr ju təˈdeɪ/"
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">音频路径</label>
              <Input
                value={sentenceForm.audio_path}
                onChange={(e) => setSentenceForm({ ...sentenceForm, audio_path: e.target.value })}
                placeholder="/audio/sentences/hello.mp3"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddSentenceDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleCreateSentence} disabled={actionLoading || !sentenceForm.content_en || !sentenceForm.content_zh}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              创建
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Sentence Dialog */}
      <Dialog open={editSentenceDialogOpen} onOpenChange={setEditSentenceDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>编辑句子</DialogTitle>
            <DialogDescription>修改句子信息</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">顺序</label>
                <Input
                  type="number"
                  value={sentenceForm.sentence_order}
                  onChange={(e) => setSentenceForm({ ...sentenceForm, sentence_order: parseInt(e.target.value) || 0 })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">难度 (1-5)</label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={sentenceForm.difficulty}
                  onChange={(e) => setSentenceForm({ ...sentenceForm, difficulty: parseInt(e.target.value) || 1 })}
                />
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">英文句子</label>
              <textarea
                className="w-full mt-1 px-3 py-2 border rounded-md text-sm"
                rows={2}
                value={sentenceForm.content_en}
                onChange={(e) => setSentenceForm({ ...sentenceForm, content_en: e.target.value })}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">中文翻译</label>
              <textarea
                className="w-full mt-1 px-3 py-2 border rounded-md text-sm"
                rows={2}
                value={sentenceForm.content_zh}
                onChange={(e) => setSentenceForm({ ...sentenceForm, content_zh: e.target.value })}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">音标</label>
              <Input
                value={sentenceForm.phonetic_transcription}
                onChange={(e) => setSentenceForm({ ...sentenceForm, phonetic_transcription: e.target.value })}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">音频路径</label>
              <Input
                value={sentenceForm.audio_path}
                onChange={(e) => setSentenceForm({ ...sentenceForm, audio_path: e.target.value })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditSentenceDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleUpdateSentence} disabled={actionLoading}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Sentence Dialog */}
      <Dialog open={deleteSentenceDialogOpen} onOpenChange={setDeleteSentenceDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>删除句子</DialogTitle>
            <DialogDescription>
              确定要删除这个句子吗？此操作不可撤销。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteSentenceDialogOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleDeleteSentence} disabled={actionLoading}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              删除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
