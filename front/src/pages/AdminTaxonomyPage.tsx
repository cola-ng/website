import { useState, useEffect } from 'react'
import { Navigate } from 'react-router-dom'
import {
  Loader2,
  Plus,
  Trash2,
  Pencil,
  FolderTree,
  Layers,
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { useAuth } from '../lib/auth'
import {
  listDomains,
  createDomain,
  updateDomain,
  deleteDomain,
  listCategories,
  createCategory,
  updateCategory,
  deleteCategory,
  type TaxonDomain,
  type TaxonCategory,
  type CreateDomainRequest,
  type CreateCategoryRequest,
} from '../lib/api'

export function AdminTaxonomyPage() {
  const { token, user } = useAuth()

  // Tab state
  const [activeTab, setActiveTab] = useState<'domains' | 'categories'>('domains')

  // Domain state
  const [domains, setDomains] = useState<TaxonDomain[]>([])
  const [domainsLoading, setDomainsLoading] = useState(false)

  // Category state
  const [categories, setCategories] = useState<TaxonCategory[]>([])
  const [categoriesLoading, setCategoriesLoading] = useState(false)
  const [selectedDomainId, setSelectedDomainId] = useState<number | undefined>(undefined)

  // Error state
  const [error, setError] = useState<string | null>(null)

  // Dialog states - Domain
  const [addDomainDialogOpen, setAddDomainDialogOpen] = useState(false)
  const [editDomainDialogOpen, setEditDomainDialogOpen] = useState(false)
  const [deleteDomainDialogOpen, setDeleteDomainDialogOpen] = useState(false)
  const [selectedDomain, setSelectedDomain] = useState<TaxonDomain | null>(null)
  const [domainForm, setDomainForm] = useState<CreateDomainRequest>({
    code: '',
    name_en: '',
    name_zh: '',
  })

  // Dialog states - Category
  const [addCategoryDialogOpen, setAddCategoryDialogOpen] = useState(false)
  const [editCategoryDialogOpen, setEditCategoryDialogOpen] = useState(false)
  const [deleteCategoryDialogOpen, setDeleteCategoryDialogOpen] = useState(false)
  const [selectedCategory, setSelectedCategory] = useState<TaxonCategory | null>(null)
  const [categoryForm, setCategoryForm] = useState<CreateCategoryRequest>({
    code: '',
    name_en: '',
    name_zh: '',
    domain_id: 0,
    parent_id: undefined,
  })

  const [actionLoading, setActionLoading] = useState(false)

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  // Fetch domains
  useEffect(() => {
    if (token) {
      setDomainsLoading(true)
      listDomains(token)
        .then(setDomains)
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load domains'))
        .finally(() => setDomainsLoading(false))
    }
  }, [token])

  // Fetch categories when tab changes or domain filter changes
  useEffect(() => {
    if (token && activeTab === 'categories') {
      setCategoriesLoading(true)
      listCategories(token, { domain_id: selectedDomainId })
        .then(setCategories)
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load categories'))
        .finally(() => setCategoriesLoading(false))
    }
  }, [token, activeTab, selectedDomainId])

  // Domain CRUD handlers
  const handleAddDomain = async () => {
    if (!token || !domainForm.code.trim() || !domainForm.name_en.trim() || !domainForm.name_zh.trim()) return
    setActionLoading(true)
    try {
      const created = await createDomain(token, {
        code: domainForm.code.trim(),
        name_en: domainForm.name_en.trim(),
        name_zh: domainForm.name_zh.trim(),
      })
      setDomains([...domains, created])
      setAddDomainDialogOpen(false)
      setDomainForm({ code: '', name_en: '', name_zh: '' })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create domain')
    } finally {
      setActionLoading(false)
    }
  }

  const openEditDomainDialog = (domain: TaxonDomain) => {
    setSelectedDomain(domain)
    setDomainForm({
      code: domain.code,
      name_en: domain.name_en,
      name_zh: domain.name_zh,
    })
    setEditDomainDialogOpen(true)
  }

  const handleEditDomain = async () => {
    if (!token || !selectedDomain || !domainForm.code.trim() || !domainForm.name_en.trim() || !domainForm.name_zh.trim()) return
    setActionLoading(true)
    try {
      const updated = await updateDomain(token, selectedDomain.id, {
        code: domainForm.code.trim(),
        name_en: domainForm.name_en.trim(),
        name_zh: domainForm.name_zh.trim(),
      })
      setDomains(domains.map((d) => (d.id === updated.id ? updated : d)))
      setEditDomainDialogOpen(false)
      setSelectedDomain(null)
      setDomainForm({ code: '', name_en: '', name_zh: '' })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update domain')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteDomain = async () => {
    if (!token || !selectedDomain) return
    setActionLoading(true)
    try {
      await deleteDomain(token, selectedDomain.id)
      setDomains(domains.filter((d) => d.id !== selectedDomain.id))
      setDeleteDomainDialogOpen(false)
      setSelectedDomain(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete domain')
    } finally {
      setActionLoading(false)
    }
  }

  // Category CRUD handlers
  const openAddCategoryDialog = () => {
    setCategoryForm({
      code: '',
      name_en: '',
      name_zh: '',
      domain_id: domains.length > 0 ? domains[0].id : 0,
      parent_id: undefined,
    })
    setAddCategoryDialogOpen(true)
  }

  const handleAddCategory = async () => {
    if (!token || !categoryForm.code.trim() || !categoryForm.name_en.trim() || !categoryForm.name_zh.trim() || !categoryForm.domain_id) return
    setActionLoading(true)
    try {
      const created = await createCategory(token, {
        code: categoryForm.code.trim(),
        name_en: categoryForm.name_en.trim(),
        name_zh: categoryForm.name_zh.trim(),
        domain_id: categoryForm.domain_id,
        parent_id: categoryForm.parent_id || undefined,
      })
      setCategories([...categories, created])
      setAddCategoryDialogOpen(false)
      setCategoryForm({ code: '', name_en: '', name_zh: '', domain_id: 0, parent_id: undefined })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create category')
    } finally {
      setActionLoading(false)
    }
  }

  const openEditCategoryDialog = (category: TaxonCategory) => {
    setSelectedCategory(category)
    setCategoryForm({
      code: category.code,
      name_en: category.name_en,
      name_zh: category.name_zh,
      domain_id: category.domain_id,
      parent_id: category.parent_id ?? undefined,
    })
    setEditCategoryDialogOpen(true)
  }

  const handleEditCategory = async () => {
    if (!token || !selectedCategory || !categoryForm.code.trim() || !categoryForm.name_en.trim() || !categoryForm.name_zh.trim()) return
    setActionLoading(true)
    try {
      const updated = await updateCategory(token, selectedCategory.id, {
        code: categoryForm.code.trim(),
        name_en: categoryForm.name_en.trim(),
        name_zh: categoryForm.name_zh.trim(),
        domain_id: categoryForm.domain_id,
        parent_id: categoryForm.parent_id ?? null,
      })
      setCategories(categories.map((c) => (c.id === updated.id ? updated : c)))
      setEditCategoryDialogOpen(false)
      setSelectedCategory(null)
      setCategoryForm({ code: '', name_en: '', name_zh: '', domain_id: 0, parent_id: undefined })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update category')
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteCategory = async () => {
    if (!token || !selectedCategory) return
    setActionLoading(true)
    try {
      await deleteCategory(token, selectedCategory.id)
      setCategories(categories.filter((c) => c.id !== selectedCategory.id))
      setDeleteCategoryDialogOpen(false)
      setSelectedCategory(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete category')
    } finally {
      setActionLoading(false)
    }
  }

  // Helper to get domain name by ID
  const getDomainName = (domainId: number) => {
    const domain = domains.find((d) => d.id === domainId)
    return domain ? domain.name_zh : '-'
  }

  // Helper to get category name by ID
  const getCategoryName = (categoryId: number | null) => {
    if (!categoryId) return '-'
    const category = categories.find((c) => c.id === categoryId)
    return category ? category.name_zh : '-'
  }

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
            <CardTitle className="flex items-center gap-2">
              <FolderTree className="h-5 w-5" />
              分类管理
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'domains' | 'categories')}>
              <TabsList className="mb-4">
                <TabsTrigger value="domains" className="flex items-center gap-2">
                  <Layers className="h-4 w-4" />
                  域 (Domains)
                </TabsTrigger>
                <TabsTrigger value="categories" className="flex items-center gap-2">
                  <FolderTree className="h-4 w-4" />
                  类别 (Categories)
                </TabsTrigger>
              </TabsList>

              {/* Domains Tab */}
              <TabsContent value="domains">
                <div className="flex justify-end mb-4">
                  <Button size="sm" onClick={() => setAddDomainDialogOpen(true)}>
                    <Plus className="h-4 w-4 mr-1" />
                    添加域
                  </Button>
                </div>

                {domainsLoading ? (
                  <div className="flex items-center justify-center py-12">
                    <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                  </div>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="w-16">ID</TableHead>
                        <TableHead className="w-32">代码</TableHead>
                        <TableHead>英文名称</TableHead>
                        <TableHead>中文名称</TableHead>
                        <TableHead className="w-40">创建时间</TableHead>
                        <TableHead className="w-24">操作</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {domains.map((domain) => (
                        <TableRow key={domain.id}>
                          <TableCell className="font-mono text-sm">{domain.id}</TableCell>
                          <TableCell className="font-mono text-sm">{domain.code}</TableCell>
                          <TableCell>{domain.name_en}</TableCell>
                          <TableCell>{domain.name_zh}</TableCell>
                          <TableCell className="text-sm text-gray-500">
                            {new Date(domain.created_at).toLocaleDateString()}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center gap-1">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => openEditDomainDialog(domain)}
                              >
                                <Pencil className="h-4 w-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                className="text-red-600 hover:text-red-700 hover:bg-red-50"
                                onClick={() => {
                                  setSelectedDomain(domain)
                                  setDeleteDomainDialogOpen(true)
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

                {!domainsLoading && domains.length === 0 && (
                  <div className="text-center py-12 text-gray-500">暂无域数据</div>
                )}
              </TabsContent>

              {/* Categories Tab */}
              <TabsContent value="categories">
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center gap-2">
                    <label className="text-sm font-medium">筛选域:</label>
                    <select
                      className="rounded-md border border-gray-300 px-3 py-1.5 text-sm"
                      value={selectedDomainId ?? ''}
                      onChange={(e) => setSelectedDomainId(e.target.value ? parseInt(e.target.value, 10) : undefined)}
                    >
                      <option value="">全部</option>
                      {domains.map((d) => (
                        <option key={d.id} value={d.id}>{d.name_zh}</option>
                      ))}
                    </select>
                  </div>
                  <Button size="sm" onClick={openAddCategoryDialog} disabled={domains.length === 0}>
                    <Plus className="h-4 w-4 mr-1" />
                    添加类别
                  </Button>
                </div>

                {categoriesLoading ? (
                  <div className="flex items-center justify-center py-12">
                    <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                  </div>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="w-16">ID</TableHead>
                        <TableHead className="w-32">代码</TableHead>
                        <TableHead>英文名称</TableHead>
                        <TableHead>中文名称</TableHead>
                        <TableHead className="w-28">所属域</TableHead>
                        <TableHead className="w-28">父类别</TableHead>
                        <TableHead className="w-24">操作</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {categories.map((category) => (
                        <TableRow key={category.id}>
                          <TableCell className="font-mono text-sm">{category.id}</TableCell>
                          <TableCell className="font-mono text-sm">{category.code}</TableCell>
                          <TableCell>{category.name_en}</TableCell>
                          <TableCell>{category.name_zh}</TableCell>
                          <TableCell>
                            <span className="px-2 py-0.5 rounded text-xs bg-blue-100 text-blue-700">
                              {getDomainName(category.domain_id)}
                            </span>
                          </TableCell>
                          <TableCell className="text-sm text-gray-500">
                            {getCategoryName(category.parent_id)}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center gap-1">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => openEditCategoryDialog(category)}
                              >
                                <Pencil className="h-4 w-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                className="text-red-600 hover:text-red-700 hover:bg-red-50"
                                onClick={() => {
                                  setSelectedCategory(category)
                                  setDeleteCategoryDialogOpen(true)
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

                {!categoriesLoading && categories.length === 0 && (
                  <div className="text-center py-12 text-gray-500">
                    {domains.length === 0 ? '请先添加域' : '暂无类别数据'}
                  </div>
                )}
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

        {/* Add Domain Dialog */}
        <Dialog open={addDomainDialogOpen} onOpenChange={setAddDomainDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>添加域</DialogTitle>
              <DialogDescription>创建一个新的分类域</DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">代码 *</label>
                <Input
                  placeholder="如: academic"
                  value={domainForm.code}
                  onChange={(e) => setDomainForm({ ...domainForm, code: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">英文名称 *</label>
                <Input
                  placeholder="如: Academic"
                  value={domainForm.name_en}
                  onChange={(e) => setDomainForm({ ...domainForm, name_en: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">中文名称 *</label>
                <Input
                  placeholder="如: 学术"
                  value={domainForm.name_zh}
                  onChange={(e) => setDomainForm({ ...domainForm, name_zh: e.target.value })}
                  className="mt-1"
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setAddDomainDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleAddDomain}
                disabled={actionLoading || !domainForm.code.trim() || !domainForm.name_en.trim() || !domainForm.name_zh.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                添加
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Edit Domain Dialog */}
        <Dialog open={editDomainDialogOpen} onOpenChange={setEditDomainDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>编辑域</DialogTitle>
              <DialogDescription>修改分类域的信息</DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">代码 *</label>
                <Input
                  placeholder="如: academic"
                  value={domainForm.code}
                  onChange={(e) => setDomainForm({ ...domainForm, code: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">英文名称 *</label>
                <Input
                  placeholder="如: Academic"
                  value={domainForm.name_en}
                  onChange={(e) => setDomainForm({ ...domainForm, name_en: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">中文名称 *</label>
                <Input
                  placeholder="如: 学术"
                  value={domainForm.name_zh}
                  onChange={(e) => setDomainForm({ ...domainForm, name_zh: e.target.value })}
                  className="mt-1"
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditDomainDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleEditDomain}
                disabled={actionLoading || !domainForm.code.trim() || !domainForm.name_en.trim() || !domainForm.name_zh.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                保存
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Domain Dialog */}
        <Dialog open={deleteDomainDialogOpen} onOpenChange={setDeleteDomainDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除域</DialogTitle>
              <DialogDescription>
                确定要删除域 "{selectedDomain?.name_zh}" 吗？如果该域下有类别，将无法删除。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeleteDomainDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDeleteDomain} disabled={actionLoading}>
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                确认删除
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Add Category Dialog */}
        <Dialog open={addCategoryDialogOpen} onOpenChange={setAddCategoryDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>添加类别</DialogTitle>
              <DialogDescription>创建一个新的分类类别</DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">代码 *</label>
                <Input
                  placeholder="如: physics"
                  value={categoryForm.code}
                  onChange={(e) => setCategoryForm({ ...categoryForm, code: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">英文名称 *</label>
                <Input
                  placeholder="如: Physics"
                  value={categoryForm.name_en}
                  onChange={(e) => setCategoryForm({ ...categoryForm, name_en: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">中文名称 *</label>
                <Input
                  placeholder="如: 物理学"
                  value={categoryForm.name_zh}
                  onChange={(e) => setCategoryForm({ ...categoryForm, name_zh: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">所属域 *</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={categoryForm.domain_id}
                  onChange={(e) => setCategoryForm({ ...categoryForm, domain_id: parseInt(e.target.value, 10) })}
                >
                  {domains.map((d) => (
                    <option key={d.id} value={d.id}>{d.name_zh}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">父类别（可选）</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={categoryForm.parent_id ?? ''}
                  onChange={(e) => setCategoryForm({ ...categoryForm, parent_id: e.target.value ? parseInt(e.target.value, 10) : undefined })}
                >
                  <option value="">无（顶级类别）</option>
                  {categories
                    .filter((c) => c.domain_id === categoryForm.domain_id)
                    .map((c) => (
                      <option key={c.id} value={c.id}>{c.name_zh}</option>
                    ))}
                </select>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setAddCategoryDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleAddCategory}
                disabled={actionLoading || !categoryForm.code.trim() || !categoryForm.name_en.trim() || !categoryForm.name_zh.trim() || !categoryForm.domain_id}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                添加
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Edit Category Dialog */}
        <Dialog open={editCategoryDialogOpen} onOpenChange={setEditCategoryDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>编辑类别</DialogTitle>
              <DialogDescription>修改分类类别的信息</DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <label className="text-sm font-medium">代码 *</label>
                <Input
                  placeholder="如: physics"
                  value={categoryForm.code}
                  onChange={(e) => setCategoryForm({ ...categoryForm, code: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">英文名称 *</label>
                <Input
                  placeholder="如: Physics"
                  value={categoryForm.name_en}
                  onChange={(e) => setCategoryForm({ ...categoryForm, name_en: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">中文名称 *</label>
                <Input
                  placeholder="如: 物理学"
                  value={categoryForm.name_zh}
                  onChange={(e) => setCategoryForm({ ...categoryForm, name_zh: e.target.value })}
                  className="mt-1"
                />
              </div>
              <div>
                <label className="text-sm font-medium">所属域 *</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={categoryForm.domain_id}
                  onChange={(e) => setCategoryForm({ ...categoryForm, domain_id: parseInt(e.target.value, 10) })}
                >
                  {domains.map((d) => (
                    <option key={d.id} value={d.id}>{d.name_zh}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-sm font-medium">父类别（可选）</label>
                <select
                  className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                  value={categoryForm.parent_id ?? ''}
                  onChange={(e) => setCategoryForm({ ...categoryForm, parent_id: e.target.value ? parseInt(e.target.value, 10) : undefined })}
                >
                  <option value="">无（顶级类别）</option>
                  {categories
                    .filter((c) => c.domain_id === categoryForm.domain_id && c.id !== selectedCategory?.id)
                    .map((c) => (
                      <option key={c.id} value={c.id}>{c.name_zh}</option>
                    ))}
                </select>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditCategoryDialogOpen(false)}>
                取消
              </Button>
              <Button
                onClick={handleEditCategory}
                disabled={actionLoading || !categoryForm.code.trim() || !categoryForm.name_en.trim() || !categoryForm.name_zh.trim()}
              >
                {actionLoading ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                保存
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Category Dialog */}
        <Dialog open={deleteCategoryDialogOpen} onOpenChange={setDeleteCategoryDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>删除类别</DialogTitle>
              <DialogDescription>
                确定要删除类别 "{selectedCategory?.name_zh}" 吗？如果该类别有子类别或被使用，将无法删除。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeleteCategoryDialogOpen(false)}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDeleteCategory} disabled={actionLoading}>
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
