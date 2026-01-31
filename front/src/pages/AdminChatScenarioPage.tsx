import { useState, useEffect } from 'react'
import { Navigate } from 'react-router-dom'
import {
  Loader2,
  Plus,
  Trash2,
  Pencil,
  MessageSquare,
  Sparkles,
  ChevronLeft,
  ChevronRight,
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
  listAdminStages,
  createAdminStage,
  updateAdminStage,
  deleteAdminStage,
  type Stage,
  type CreateStageRequest,
} from '../lib/api'

export function AdminChatScenarioPage() {
  const { token, user } = useAuth()

  // Stage state
  const [stages, setStages] = useState<Stage[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Pagination state
  const [page, setPage] = useState(1)
  const [total, setTotal] = useState(0)
  const [search, setSearch] = useState('')
  const [searchInput, setSearchInput] = useState('')
  const [perPage, setPerPage] = useState(() => getAdminPerPage())

  // Dialog states
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [selectedStage, setSelectedStage] = useState<Stage | null>(null)
  const [formData, setFormData] = useState<CreateStageRequest>({
    code: '',
    name_en: '',
    name_zh: '',
    description_en: '',
    description_zh: '',
    icon_emoji: '',
    display_order: 0,
    difficulty: 1,
    is_active: true,
    ai_prompt: '',
  })
  const [actionLoading, setActionLoading] = useState(false)

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  const totalPages = Math.ceil(total / perPage)

  // Fetch stages
  useEffect(() => {
    if (token) {
      setLoading(true)
      listAdminStages(token, page, perPage, search || undefined)
        .then((res) => {
          setStages(res.stages)
          setTotal(res.total)
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load stages'))
        .finally(() => setLoading(false))
    }
  }, [token, page, perPage, search])

  const refreshStages = () => {
    if (token) {
      listAdminStages(token, page, perPage, search || undefined)
        .then((res) => {
          setStages(res.stages)
          setTotal(res.total)
        })
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load stages'))
    }
  }

  const handleSearch = () => {
    setPage(1)
    setSearch(searchInput)
  }

  const handleSearchKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSearch()
    }
  }

  const handlePerPageChange = (value: number) => {
    setPerPage(value)
    setAdminPerPage(value)
    setPage(1)
  }

  const handleOpenAdd = () => {
    setFormData({
      code: '',
      name_en: '',
      name_zh: '',
      description_en: '',
      description_zh: '',
      icon_emoji: '💬',
      display_order: stages.length,
      difficulty: 1,
      is_active: true,
      ai_prompt: '',
    })
    setAddDialogOpen(true)
  }

  const handleOpenEdit = (stage: Stage) => {
    setSelectedStage(stage)
    setFormData({
      code: stage.code,
      name_en: stage.name_en,
      name_zh: stage.name_zh,
      description_en: stage.description_en || '',
      description_zh: stage.description_zh || '',
      icon_emoji: stage.icon_emoji || '',
      display_order: stage.display_order || 0,
      difficulty: stage.difficulty || 1,
      is_active: stage.is_active ?? true,
      ai_prompt: stage.ai_prompt || '',
    })
    setEditDialogOpen(true)
  }

  const handleOpenDelete = (stage: Stage) => {
    setSelectedStage(stage)
    setDeleteDialogOpen(true)
  }

  const handleCreate = async () => {
    if (!token) return
    setActionLoading(true)
    try {
      await createAdminStage(token, formData)
      setAddDialogOpen(false)
      refreshStages()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create stage')
    } finally {
      setActionLoading(false)
    }
  }

  const handleUpdate = async () => {
    if (!token || !selectedStage) return
    setActionLoading(true)
    try {
      await updateAdminStage(token, selectedStage.id, formData)
      setEditDialogOpen(false)
      refreshStages()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update stage')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDelete = async () => {
    if (!token || !selectedStage) return
    setActionLoading(true)
    try {
      await deleteAdminStage(token, selectedStage.id)
      setDeleteDialogOpen(false)
      refreshStages()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete stage')
    } finally {
      setActionLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Header />

      <main className="max-w-6xl mx-auto px-4 py-6">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <MessageSquare className="h-8 w-8 text-purple-600" />
            <div>
              <h1 className="text-2xl font-bold text-gray-900">唠嗑场景管理</h1>
              <p className="text-sm text-gray-500">管理聊天场景及其 AI 提示词</p>
            </div>
          </div>
          <Button onClick={handleOpenAdd}>
            <Plus className="h-4 w-4 mr-2" />
            添加场景
          </Button>
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-50 text-red-600 rounded-lg">
            {error}
            <Button
              variant="ghost"
              size="sm"
              className="ml-2"
              onClick={() => setError(null)}
            >
              关闭
            </Button>
          </div>
        )}

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>场景列表</CardTitle>
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
                    placeholder="搜索名称或代码..."
                    value={searchInput}
                    onChange={(e) => setSearchInput(e.target.value)}
                    onKeyDown={handleSearchKeyDown}
                    className="w-64"
                  />
                  <Button variant="outline" size="icon" onClick={handleSearch}>
                    <Search className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
              </div>
            ) : (
              <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-12">图标</TableHead>
                    <TableHead>名称</TableHead>
                    <TableHead>代码</TableHead>
                    <TableHead className="w-16">难度</TableHead>
                    <TableHead className="w-16">顺序</TableHead>
                    <TableHead className="w-16 whitespace-nowrap">状态</TableHead>
                    <TableHead className="w-32 whitespace-nowrap">AI 提示词</TableHead>
                    <TableHead className="w-24 text-right whitespace-nowrap">操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {stages.map((stage) => (
                    <TableRow key={stage.id}>
                      <TableCell className="text-2xl">{stage.icon_emoji || '💬'}</TableCell>
                      <TableCell>
                        <div className="font-medium">{stage.name_zh}</div>
                        <div className="text-sm text-gray-500">{stage.name_en}</div>
                      </TableCell>
                      <TableCell className="font-mono text-sm">{stage.code}</TableCell>
                      <TableCell>
                        <span className="inline-flex items-center px-2 py-1 rounded-full text-xs bg-purple-100 text-purple-700">
                          L{stage.difficulty || 1}
                        </span>
                      </TableCell>
                      <TableCell>{stage.display_order}</TableCell>
                      <TableCell className="whitespace-nowrap">
                        {stage.is_active ? (
                          <span className="inline-flex items-center px-2 py-1 rounded-full text-xs bg-green-100 text-green-700">
                            启用
                          </span>
                        ) : (
                          <span className="inline-flex items-center px-2 py-1 rounded-full text-xs bg-gray-100 text-gray-600">
                            禁用
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        {stage.ai_prompt ? (
                          <span className="inline-flex items-center gap-1 text-xs text-purple-600">
                            <Sparkles className="h-3 w-3" />
                            已配置
                          </span>
                        ) : (
                          <span className="text-xs text-gray-400">未配置</span>
                        )}
                      </TableCell>
                      <TableCell className="text-right whitespace-nowrap">
                        <Button
                          variant="ghost"
                          size="icon"
                          title="编辑"
                          onClick={() => handleOpenEdit(stage)}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title="删除"
                          onClick={() => handleOpenDelete(stage)}
                        >
                          <Trash2 className="h-4 w-4 text-red-500" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                  {stages.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={8} className="text-center py-8 text-gray-500">
                        暂无场景数据
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>

              {/* Pagination */}
              <div className="flex items-center justify-between mt-4">
                <div className="text-sm text-gray-500">
                  共 {total} 条记录
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setPage((p) => Math.max(1, p - 1))}
                    disabled={page === 1}
                  >
                    <ChevronLeft className="h-4 w-4" />
                  </Button>
                  <span className="text-sm">
                    {page} / {totalPages || 1}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                    disabled={page >= totalPages}
                  >
                    <ChevronRight className="h-4 w-4" />
                  </Button>
                </div>
              </div>
              </>
            )}
          </CardContent>
        </Card>
      </main>

      {/* Add Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>添加场景</DialogTitle>
            <DialogDescription>创建一个新的聊天场景</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">代码</label>
                <Input
                  value={formData.code}
                  onChange={(e) => setFormData({ ...formData, code: e.target.value })}
                  placeholder="e.g., restaurant"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">图标</label>
                <Input
                  value={formData.icon_emoji}
                  onChange={(e) => setFormData({ ...formData, icon_emoji: e.target.value })}
                  placeholder="e.g., 🍽️"
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">中文名称</label>
                <Input
                  value={formData.name_zh}
                  onChange={(e) => setFormData({ ...formData, name_zh: e.target.value })}
                  placeholder="e.g., 餐厅点餐"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">英文名称</label>
                <Input
                  value={formData.name_en}
                  onChange={(e) => setFormData({ ...formData, name_en: e.target.value })}
                  placeholder="e.g., Restaurant Ordering"
                />
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">中文描述</label>
              <Input
                value={formData.description_zh}
                onChange={(e) => setFormData({ ...formData, description_zh: e.target.value })}
                placeholder="场景描述（中文）"
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">英文描述</label>
              <Input
                value={formData.description_en}
                onChange={(e) => setFormData({ ...formData, description_en: e.target.value })}
                placeholder="Scene description (English)"
              />
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">难度 (1-5)</label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={formData.difficulty}
                  onChange={(e) => setFormData({ ...formData, difficulty: parseInt(e.target.value) || 1 })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">排序</label>
                <Input
                  type="number"
                  value={formData.display_order}
                  onChange={(e) => setFormData({ ...formData, display_order: parseInt(e.target.value) || 0 })}
                />
              </div>
              <div className="flex items-center gap-2 pt-6">
                <input
                  type="checkbox"
                  id="is_active"
                  checked={formData.is_active}
                  onChange={(e) => setFormData({ ...formData, is_active: e.target.checked })}
                  className="rounded"
                />
                <label htmlFor="is_active" className="text-sm font-medium text-gray-700">启用</label>
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">
                <Sparkles className="h-4 w-4 inline mr-1 text-purple-500" />
                AI 系统提示词
              </label>
              <textarea
                className="w-full mt-1 px-3 py-2 border rounded-md text-sm min-h-[150px]"
                value={formData.ai_prompt}
                onChange={(e) => setFormData({ ...formData, ai_prompt: e.target.value })}
                placeholder="输入用于此场景的 AI 系统提示词，例如：You are a friendly waiter at a restaurant. Help the customer order food..."
              />
              <p className="text-xs text-gray-500 mt-1">
                此提示词将在用户选择此场景聊天时发送给 AI
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleCreate} disabled={actionLoading || !formData.code || !formData.name_zh}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              创建
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>编辑场景</DialogTitle>
            <DialogDescription>修改聊天场景信息</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">代码</label>
                <Input
                  value={formData.code}
                  onChange={(e) => setFormData({ ...formData, code: e.target.value })}
                  placeholder="e.g., restaurant"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">图标</label>
                <Input
                  value={formData.icon_emoji}
                  onChange={(e) => setFormData({ ...formData, icon_emoji: e.target.value })}
                  placeholder="e.g., 🍽️"
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">中文名称</label>
                <Input
                  value={formData.name_zh}
                  onChange={(e) => setFormData({ ...formData, name_zh: e.target.value })}
                  placeholder="e.g., 餐厅点餐"
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">英文名称</label>
                <Input
                  value={formData.name_en}
                  onChange={(e) => setFormData({ ...formData, name_en: e.target.value })}
                  placeholder="e.g., Restaurant Ordering"
                />
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">中文描述</label>
              <Input
                value={formData.description_zh}
                onChange={(e) => setFormData({ ...formData, description_zh: e.target.value })}
                placeholder="场景描述（中文）"
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">英文描述</label>
              <Input
                value={formData.description_en}
                onChange={(e) => setFormData({ ...formData, description_en: e.target.value })}
                placeholder="Scene description (English)"
              />
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div>
                <label className="text-sm font-medium text-gray-700">难度 (1-5)</label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={formData.difficulty}
                  onChange={(e) => setFormData({ ...formData, difficulty: parseInt(e.target.value) || 1 })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-700">排序</label>
                <Input
                  type="number"
                  value={formData.display_order}
                  onChange={(e) => setFormData({ ...formData, display_order: parseInt(e.target.value) || 0 })}
                />
              </div>
              <div className="flex items-center gap-2 pt-6">
                <input
                  type="checkbox"
                  id="edit_is_active"
                  checked={formData.is_active}
                  onChange={(e) => setFormData({ ...formData, is_active: e.target.checked })}
                  className="rounded"
                />
                <label htmlFor="edit_is_active" className="text-sm font-medium text-gray-700">启用</label>
              </div>
            </div>
            <div>
              <label className="text-sm font-medium text-gray-700">
                <Sparkles className="h-4 w-4 inline mr-1 text-purple-500" />
                AI 系统提示词
              </label>
              <textarea
                className="w-full mt-1 px-3 py-2 border rounded-md text-sm min-h-[150px]"
                value={formData.ai_prompt}
                onChange={(e) => setFormData({ ...formData, ai_prompt: e.target.value })}
                placeholder="输入用于此场景的 AI 系统提示词..."
              />
              <p className="text-xs text-gray-500 mt-1">
                此提示词将在用户选择此场景聊天时发送给 AI
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleUpdate} disabled={actionLoading}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>删除场景</DialogTitle>
            <DialogDescription>
              确定要删除场景 "{selectedStage?.name_zh}" 吗？此操作不可撤销。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleDelete} disabled={actionLoading}>
              {actionLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              删除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
