import { useState, useEffect } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import {
  Users,
  Shield,
  BookOpen,
  TrendingUp,
  Search,
  ChevronLeft,
  ChevronRight,
  ShieldPlus,
  ShieldOff,
  Loader2,
  Eye,
  CreditCard,
  Wallet,
  Crown,
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
  getAdminStats,
  listAdminUsers,
  getAdminUser,
  grantAdmin,
  revokeAdmin,
  type DashboardStats,
  type UserListItem,
  type UserDetailResponse,
} from '../lib/api'

export function AdminPage() {
  const { token, user } = useAuth()
  const location = useLocation()

  // Derive active section from URL path
  const activeTab = location.pathname === '/admin/users' ? 'users' : 'dashboard'
  const [stats, setStats] = useState<DashboardStats | null>(null)
  const [users, setUsers] = useState<UserListItem[]>([])
  const [totalUsers, setTotalUsers] = useState(0)
  const [page, setPage] = useState(1)
  const [search, setSearch] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Dialog state
  const [grantDialogOpen, setGrantDialogOpen] = useState(false)
  const [revokeDialogOpen, setRevokeDialogOpen] = useState(false)
  const [detailDialogOpen, setDetailDialogOpen] = useState(false)
  const [selectedUser, setSelectedUser] = useState<UserListItem | null>(null)
  const [userDetail, setUserDetail] = useState<UserDetailResponse | null>(null)
  const [detailLoading, setDetailLoading] = useState(false)
  const [grantNotes, setGrantNotes] = useState('')
  const [actionLoading, setActionLoading] = useState(false)

  const perPage = 20

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  // Fetch stats
  useEffect(() => {
    if (token && activeTab === 'dashboard') {
      setLoading(true)
      getAdminStats(token)
        .then(setStats)
        .catch((e) => setError(e.message))
        .finally(() => setLoading(false))
    }
  }, [token, activeTab])

  // Fetch users
  useEffect(() => {
    if (token && activeTab === 'users') {
      setLoading(true)
      listAdminUsers(token, page, perPage, search || undefined)
        .then((res) => {
          setUsers(res.users)
          setTotalUsers(res.total)
        })
        .catch((e) => setError(e.message))
        .finally(() => setLoading(false))
    }
  }, [token, activeTab, page, search])

  const handleGrantAdmin = async () => {
    if (!token || !selectedUser) return
    setActionLoading(true)
    try {
      await grantAdmin(token, selectedUser.id, grantNotes || undefined)
      setGrantDialogOpen(false)
      setSelectedUser(null)
      setGrantNotes('')
      // Refresh users list
      const res = await listAdminUsers(token, page, perPage, search || undefined)
      setUsers(res.users)
      // Also refresh stats
      const statsRes = await getAdminStats(token)
      setStats(statsRes)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to grant admin')
    } finally {
      setActionLoading(false)
    }
  }

  const handleRevokeAdmin = async () => {
    if (!token || !selectedUser) return
    setActionLoading(true)
    try {
      await revokeAdmin(token, selectedUser.id)
      setRevokeDialogOpen(false)
      setSelectedUser(null)
      // Refresh users list
      const res = await listAdminUsers(token, page, perPage, search || undefined)
      setUsers(res.users)
      // Also refresh stats
      const statsRes = await getAdminStats(token)
      setStats(statsRes)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to revoke admin')
    } finally {
      setActionLoading(false)
    }
  }

  const totalPages = Math.ceil(totalUsers / perPage)

  const handleViewUser = async (u: UserListItem) => {
    if (!token) return
    setSelectedUser(u)
    setDetailDialogOpen(true)
    setDetailLoading(true)
    try {
      const detail = await getAdminUser(token, u.id)
      setUserDetail(detail)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load user details')
      setDetailDialogOpen(false)
    } finally {
      setDetailLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Header />

      <main className="max-w-6xl mx-auto px-4 py-6">
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

        {/* Dashboard */}
        {activeTab === 'dashboard' && (
          loading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
            </div>
          ) : stats ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">总用户数</CardTitle>
                  <Users className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{stats.total_users.toLocaleString()}</div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">管理员数</CardTitle>
                  <Shield className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{stats.total_admins}</div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">词典词条</CardTitle>
                  <BookOpen className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{stats.total_words.toLocaleString()}</div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">今日注册</CardTitle>
                  <TrendingUp className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{stats.users_today}</div>
                </CardContent>
              </Card>
            </div>
          ) : null
        )}

        {/* Users */}
        {activeTab === 'users' && (
          <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle>用户列表</CardTitle>
                  <div className="flex items-center gap-2">
                    <div className="relative">
                      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
                      <Input
                        placeholder="搜索用户..."
                        value={search}
                        onChange={(e) => {
                          setSearch(e.target.value)
                          setPage(1)
                        }}
                        className="pl-9 w-64"
                      />
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
                          <TableHead>ID</TableHead>
                          <TableHead>姓名</TableHead>
                          <TableHead>邮箱</TableHead>
                          <TableHead>手机</TableHead>
                          <TableHead>管理员</TableHead>
                          <TableHead>注册时间</TableHead>
                          <TableHead>操作</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {users.map((u) => (
                          <TableRow key={u.id}>
                            <TableCell className="font-mono text-sm">{u.id}</TableCell>
                            <TableCell>
                              <button
                                className="text-blue-600 hover:text-blue-800 hover:underline text-left"
                                onClick={() => handleViewUser(u)}
                              >
                                {u.name}
                              </button>
                            </TableCell>
                            <TableCell>{u.email || '-'}</TableCell>
                            <TableCell>{u.phone || '-'}</TableCell>
                            <TableCell>
                              {u.is_admin ? (
                                <span className="inline-flex items-center gap-1 text-purple-600 text-sm">
                                  <Shield className="h-3 w-3" /> 是
                                </span>
                              ) : (
                                <span className="text-gray-400 text-sm">否</span>
                              )}
                            </TableCell>
                            <TableCell className="text-sm text-gray-500">
                              {new Date(u.created_at).toLocaleDateString()}
                            </TableCell>
                            <TableCell>
                              <div className="flex items-center gap-1">
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="text-gray-600 hover:text-gray-700 hover:bg-gray-50"
                                  onClick={() => handleViewUser(u)}
                                >
                                  <Eye className="h-4 w-4" />
                                </Button>
                                {u.is_admin ? (
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    className="text-red-600 hover:text-red-700 hover:bg-red-50"
                                    onClick={() => {
                                      setSelectedUser(u)
                                      setRevokeDialogOpen(true)
                                    }}
                                  >
                                    <ShieldOff className="h-4 w-4" />
                                  </Button>
                                ) : (
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    className="text-purple-600 hover:text-purple-700 hover:bg-purple-50"
                                    onClick={() => {
                                      setSelectedUser(u)
                                      setGrantDialogOpen(true)
                                    }}
                                  >
                                    <ShieldPlus className="h-4 w-4" />
                                  </Button>
                                )}
                              </div>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>

                    {/* Pagination */}
                    <div className="flex items-center justify-between mt-4">
                      <div className="text-sm text-gray-500">
                        共 {totalUsers} 条记录
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
        )}

      </main>

      {/* Grant Admin Dialog */}
      <Dialog open={grantDialogOpen} onOpenChange={setGrantDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>授予管理员权限</DialogTitle>
            <DialogDescription>
              确定要授予用户 "{selectedUser?.name}" 管理员权限吗？
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <label className="text-sm font-medium">备注（可选）</label>
            <Input
              placeholder="输入备注..."
              value={grantNotes}
              onChange={(e) => setGrantNotes(e.target.value)}
              className="mt-2"
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setGrantDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleGrantAdmin} disabled={actionLoading}>
              {actionLoading ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <ShieldPlus className="h-4 w-4 mr-2" />
              )}
              确认授权
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Revoke Admin Dialog */}
      <Dialog open={revokeDialogOpen} onOpenChange={setRevokeDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>撤销管理员权限</DialogTitle>
            <DialogDescription>
              确定要撤销用户 "{selectedUser?.name}" 的管理员权限吗？此操作不可撤销。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRevokeDialogOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleRevokeAdmin} disabled={actionLoading}>
              {actionLoading ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <ShieldOff className="h-4 w-4 mr-2" />
              )}
              确认撤销
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* User Detail Dialog */}
      <Dialog open={detailDialogOpen} onOpenChange={(open) => {
        setDetailDialogOpen(open)
        if (!open) {
          setUserDetail(null)
          setSelectedUser(null)
        }
      }}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>用户详情</DialogTitle>
            <DialogDescription>
              查看用户 "{selectedUser?.name}" 的详细信息
            </DialogDescription>
          </DialogHeader>
          {detailLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
            </div>
          ) : userDetail ? (
            <div className="space-y-6">
              {/* Basic Info */}
              <div>
                <h3 className="text-sm font-semibold text-gray-900 mb-3 flex items-center gap-2">
                  <Users className="h-4 w-4" /> 基本信息
                </h3>
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-gray-500">用户ID：</span>
                    <span className="font-mono">{userDetail.id}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">姓名：</span>
                    <span>{userDetail.name}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">邮箱：</span>
                    <span>{userDetail.email || '-'}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">手机：</span>
                    <span>{userDetail.phone || '-'}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">注册时间：</span>
                    <span>{new Date(userDetail.created_at).toLocaleString()}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">管理员：</span>
                    {userDetail.is_admin ? (
                      <span className="text-purple-600">是</span>
                    ) : (
                      <span className="text-gray-400">否</span>
                    )}
                  </div>
                </div>
              </div>

              {/* Subscription Info */}
              <div>
                <h3 className="text-sm font-semibold text-gray-900 mb-3 flex items-center gap-2">
                  <Crown className="h-4 w-4" /> 订阅信息
                </h3>
                {userDetail.subscription ? (
                  <div className="bg-amber-50 rounded-lg p-4">
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <span className="text-gray-500">套餐：</span>
                        <span className="font-medium text-amber-700">{userDetail.subscription.plan.name_zh}</span>
                      </div>
                      <div>
                        <span className="text-gray-500">状态：</span>
                        <span className={userDetail.subscription.status === 'active' ? 'text-green-600' : 'text-orange-600'}>
                          {userDetail.subscription.status === 'active' ? '活跃' : userDetail.subscription.status === 'trial' ? '试用中' : userDetail.subscription.status}
                        </span>
                      </div>
                      <div>
                        <span className="text-gray-500">开始时间：</span>
                        <span>{new Date(userDetail.subscription.started_at).toLocaleDateString()}</span>
                      </div>
                      <div>
                        <span className="text-gray-500">到期时间：</span>
                        <span>{userDetail.subscription.expires_at ? new Date(userDetail.subscription.expires_at).toLocaleDateString() : '永久'}</span>
                      </div>
                      <div>
                        <span className="text-gray-500">自动续费：</span>
                        <span>{userDetail.subscription.auto_renew ? '是' : '否'}</span>
                      </div>
                      <div>
                        <span className="text-gray-500">价格：</span>
                        <span>{userDetail.subscription.plan.price_display || `${userDetail.subscription.plan.price_credits} 积分`}</span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-gray-500 text-sm">暂无订阅</div>
                )}
              </div>

              {/* Wallet Info */}
              <div>
                <h3 className="text-sm font-semibold text-gray-900 mb-3 flex items-center gap-2">
                  <Wallet className="h-4 w-4" /> 钱包信息
                </h3>
                {userDetail.wallet ? (
                  <div className="bg-blue-50 rounded-lg p-4">
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <span className="text-gray-500">余额：</span>
                        <span className="font-medium text-blue-700">{userDetail.wallet.balance} 积分</span>
                      </div>
                      <div>
                        <span className="text-gray-500">冻结金额：</span>
                        <span>{userDetail.wallet.frozen_balance} 积分</span>
                      </div>
                      <div>
                        <span className="text-gray-500">累计充值：</span>
                        <span>{userDetail.wallet.total_deposited} 积分</span>
                      </div>
                      <div>
                        <span className="text-gray-500">累计消费：</span>
                        <span>{userDetail.wallet.total_spent} 积分</span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-gray-500 text-sm">暂无钱包</div>
                )}
              </div>

              {/* Recent Payments */}
              <div>
                <h3 className="text-sm font-semibold text-gray-900 mb-3 flex items-center gap-2">
                  <CreditCard className="h-4 w-4" /> 最近支付记录
                </h3>
                {userDetail.recent_payments.length > 0 ? (
                  <div className="space-y-2">
                    {userDetail.recent_payments.map((payment) => (
                      <div key={payment.id} className="bg-gray-50 rounded-lg p-3 text-sm">
                        <div className="flex justify-between items-start">
                          <div>
                            <div className="font-mono text-xs text-gray-400">{payment.order_no}</div>
                            <div className="flex items-center gap-2 mt-1">
                              <span className="text-gray-600">{payment.order_type}</span>
                              <span className={`px-2 py-0.5 rounded text-xs ${
                                payment.status === 'paid' ? 'bg-green-100 text-green-700' :
                                payment.status === 'pending' ? 'bg-yellow-100 text-yellow-700' :
                                'bg-red-100 text-red-700'
                              }`}>
                                {payment.status === 'paid' ? '已支付' : payment.status === 'pending' ? '待支付' : payment.status}
                              </span>
                            </div>
                          </div>
                          <div className="text-right">
                            <div className="font-medium">¥{(payment.amount_cents / 100).toFixed(2)}</div>
                            <div className="text-xs text-gray-400">
                              {payment.provider} · {new Date(payment.created_at).toLocaleDateString()}
                            </div>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-gray-500 text-sm">暂无支付记录</div>
                )}
              </div>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  )
}
