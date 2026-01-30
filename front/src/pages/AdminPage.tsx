import { useState, useEffect } from 'react'
import { Navigate, useLocation, useNavigate } from 'react-router-dom'
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
  Cpu,
  MessageSquare,
  Mic,
  Volume2,
} from 'lucide-react'
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts'

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
  grantAdmin,
  revokeAdmin,
  getAdminUserStats,
  getAiUsageToday,
  getTopAiUsers,
  type DashboardStats,
  type UserListItem,
  type DailyUserStats,
  type TodayAiUsage,
  type TopAiUser,
} from '../lib/api'

// Chart type options
type ChartType = 'line' | 'area' | 'bar'

// Colors for pie chart
const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6']

export function AdminPage() {
  const { token, user } = useAuth()
  const location = useLocation()
  const navigate = useNavigate()

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
  const [selectedUser, setSelectedUser] = useState<UserListItem | null>(null)
  const [grantNotes, setGrantNotes] = useState('')
  const [actionLoading, setActionLoading] = useState(false)

  // Chart state
  const [userStats, setUserStats] = useState<DailyUserStats[]>([])
  const [userChartType, setUserChartType] = useState<ChartType>('area')
  const [userChartDays, setUserChartDays] = useState(30)
  const [aiUsage, setAiUsage] = useState<TodayAiUsage | null>(null)
  const [topAiUsers, setTopAiUsers] = useState<TopAiUser[]>([])
  const [chartsLoading, setChartsLoading] = useState(false)

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

  // Fetch chart data
  useEffect(() => {
    if (token && activeTab === 'dashboard') {
      setChartsLoading(true)
      Promise.all([
        getAdminUserStats(token, userChartDays),
        getAiUsageToday(token),
        getTopAiUsers(token, 10),
      ])
        .then(([userStatsData, aiUsageData, topUsersData]) => {
          setUserStats(userStatsData)
          setAiUsage(aiUsageData)
          setTopAiUsers(topUsersData)
        })
        .catch((e) => console.error('Failed to load chart data:', e))
        .finally(() => setChartsLoading(false))
    }
  }, [token, activeTab, userChartDays])

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

  const handleViewUser = (u: UserListItem) => {
    navigate(`/admin/users/${u.id}`)
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
            <div className="space-y-6">
              {/* Stats Cards */}
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

              {/* User Growth Chart */}
              <Card>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base flex items-center gap-2">
                      <TrendingUp className="h-4 w-4" /> 用户增长趋势
                    </CardTitle>
                    <div className="flex items-center gap-2">
                      <select
                        className="rounded-md border border-gray-300 px-2 py-1 text-sm"
                        value={userChartDays}
                        onChange={(e) => setUserChartDays(parseInt(e.target.value, 10))}
                      >
                        <option value={7}>最近 7 天</option>
                        <option value={30}>最近 30 天</option>
                        <option value={90}>最近 90 天</option>
                      </select>
                      <select
                        className="rounded-md border border-gray-300 px-2 py-1 text-sm"
                        value={userChartType}
                        onChange={(e) => setUserChartType(e.target.value as ChartType)}
                      >
                        <option value="line">折线图</option>
                        <option value="area">面积图</option>
                        <option value="bar">柱状图</option>
                      </select>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  {chartsLoading ? (
                    <div className="flex items-center justify-center h-64">
                      <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                    </div>
                  ) : userStats.length > 0 ? (
                    <ResponsiveContainer width="100%" height={300}>
                      {userChartType === 'line' ? (
                        <LineChart data={userStats}>
                          <CartesianGrid strokeDasharray="3 3" />
                          <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                          <YAxis tick={{ fontSize: 12 }} />
                          <Tooltip />
                          <Legend />
                          <Line type="monotone" dataKey="new_users" name="新增用户" stroke="#3b82f6" strokeWidth={2} />
                          <Line type="monotone" dataKey="total_users" name="累计用户" stroke="#10b981" strokeWidth={2} />
                        </LineChart>
                      ) : userChartType === 'area' ? (
                        <AreaChart data={userStats}>
                          <CartesianGrid strokeDasharray="3 3" />
                          <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                          <YAxis tick={{ fontSize: 12 }} />
                          <Tooltip />
                          <Legend />
                          <Area type="monotone" dataKey="new_users" name="新增用户" stroke="#3b82f6" fill="#93c5fd" />
                          <Area type="monotone" dataKey="total_users" name="累计用户" stroke="#10b981" fill="#6ee7b7" />
                        </AreaChart>
                      ) : (
                        <BarChart data={userStats}>
                          <CartesianGrid strokeDasharray="3 3" />
                          <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                          <YAxis tick={{ fontSize: 12 }} />
                          <Tooltip />
                          <Legend />
                          <Bar dataKey="new_users" name="新增用户" fill="#3b82f6" />
                        </BarChart>
                      )}
                    </ResponsiveContainer>
                  ) : (
                    <div className="flex items-center justify-center h-64 text-gray-500">
                      暂无数据
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* AI Usage Section */}
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                {/* AI Usage Summary Cards */}
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                      <Cpu className="h-4 w-4" /> 今日 AI 用量
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    {chartsLoading ? (
                      <div className="flex items-center justify-center h-32">
                        <Loader2 className="h-6 w-6 animate-spin text-gray-400" />
                      </div>
                    ) : aiUsage ? (
                      <div className="space-y-4">
                        <div className="grid grid-cols-2 gap-4">
                          <div className="p-3 bg-blue-50 rounded-lg">
                            <div className="flex items-center gap-2 text-blue-600 mb-1">
                              <MessageSquare className="h-4 w-4" />
                              <span className="text-sm">请求数</span>
                            </div>
                            <div className="text-xl font-bold text-blue-700">
                              {aiUsage.total_requests.toLocaleString()}
                            </div>
                          </div>
                          <div className="p-3 bg-green-50 rounded-lg">
                            <div className="flex items-center gap-2 text-green-600 mb-1">
                              <TrendingUp className="h-4 w-4" />
                              <span className="text-sm">Tokens</span>
                            </div>
                            <div className="text-xl font-bold text-green-700">
                              {aiUsage.total_tokens.toLocaleString()}
                            </div>
                          </div>
                          <div className="p-3 bg-amber-50 rounded-lg">
                            <div className="flex items-center gap-2 text-amber-600 mb-1">
                              <Mic className="h-4 w-4" />
                              <span className="text-sm">ASR 秒数</span>
                            </div>
                            <div className="text-xl font-bold text-amber-700">
                              {aiUsage.total_asr_seconds.toFixed(1)}s
                            </div>
                          </div>
                          <div className="p-3 bg-purple-50 rounded-lg">
                            <div className="flex items-center gap-2 text-purple-600 mb-1">
                              <Volume2 className="h-4 w-4" />
                              <span className="text-sm">TTS 秒数</span>
                            </div>
                            <div className="text-xl font-bold text-purple-700">
                              {aiUsage.total_tts_seconds.toFixed(1)}s
                            </div>
                          </div>
                        </div>

                        {/* Hourly Usage Chart */}
                        {aiUsage.by_hour.length > 0 && (
                          <div className="mt-4">
                            <div className="text-sm font-medium text-gray-700 mb-2">按小时分布</div>
                            <ResponsiveContainer width="100%" height={150}>
                              <BarChart data={aiUsage.by_hour}>
                                <CartesianGrid strokeDasharray="3 3" />
                                <XAxis dataKey="hour" tick={{ fontSize: 10 }} tickFormatter={(h) => `${h}时`} />
                                <YAxis tick={{ fontSize: 10 }} />
                                <Tooltip labelFormatter={(h) => `${h}:00`} />
                                <Bar dataKey="requests" name="请求数" fill="#3b82f6" />
                              </BarChart>
                            </ResponsiveContainer>
                          </div>
                        )}

                        {/* Service Distribution */}
                        {aiUsage.by_service.length > 0 && (
                          <div className="mt-4">
                            <div className="text-sm font-medium text-gray-700 mb-2">按服务类型</div>
                            <div className="flex flex-wrap gap-2">
                              {aiUsage.by_service.map((service, index) => (
                                <div
                                  key={service.service_type}
                                  className="flex items-center gap-2 px-3 py-1.5 rounded-full text-sm"
                                  style={{ backgroundColor: `${COLORS[index % COLORS.length]}20`, color: COLORS[index % COLORS.length] }}
                                >
                                  <span className="font-medium">{service.service_type}</span>
                                  <span className="text-xs opacity-80">{service.requests} 次</span>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    ) : (
                      <div className="text-gray-500 text-center py-8">暂无数据</div>
                    )}
                  </CardContent>
                </Card>

                {/* Top AI Users Table */}
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                      <Users className="h-4 w-4" /> 今日 AI 用量排名
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    {chartsLoading ? (
                      <div className="flex items-center justify-center h-32">
                        <Loader2 className="h-6 w-6 animate-spin text-gray-400" />
                      </div>
                    ) : topAiUsers.length > 0 ? (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="w-12">#</TableHead>
                            <TableHead>用户</TableHead>
                            <TableHead className="text-right">请求数</TableHead>
                            <TableHead className="text-right">Tokens</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {topAiUsers.map((user, index) => (
                            <TableRow key={user.user_id}>
                              <TableCell className="font-medium">
                                {index < 3 ? (
                                  <span className={`inline-flex items-center justify-center w-6 h-6 rounded-full text-white text-xs ${
                                    index === 0 ? 'bg-yellow-500' : index === 1 ? 'bg-gray-400' : 'bg-amber-700'
                                  }`}>
                                    {index + 1}
                                  </span>
                                ) : (
                                  <span className="text-gray-500">{index + 1}</span>
                                )}
                              </TableCell>
                              <TableCell>
                                <button
                                  className="text-blue-600 hover:text-blue-800 hover:underline text-left"
                                  onClick={() => navigate(`/admin/users/${user.user_id}`)}
                                >
                                  {user.user_name}
                                </button>
                              </TableCell>
                              <TableCell className="text-right font-mono text-sm">
                                {user.total_requests.toLocaleString()}
                              </TableCell>
                              <TableCell className="text-right font-mono text-sm">
                                {user.total_tokens.toLocaleString()}
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <div className="text-gray-500 text-center py-8">今日暂无 AI 用量</div>
                    )}
                  </CardContent>
                </Card>
              </div>
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

    </div>
  )
}
