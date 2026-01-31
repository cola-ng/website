import { useState, useEffect } from 'react'
import { Navigate, useNavigate, useLocation } from 'react-router-dom'
import {
  DollarSign,
  ShoppingCart,
  CreditCard,
  Users,
  TrendingUp,
  Search,
  ChevronLeft,
  ChevronRight,
  Loader2,
  Eye,
  CheckCircle,
  XCircle,
  Clock,
  AlertCircle,
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
import { useAuth } from '../lib/auth'
import {
  getFinanceStats,
  listOrders,
  listSubscriptions,
  type FinanceStats,
  type OrderListItem,
  type SubscriptionListItem,
} from '../lib/api'

// Format cents to display currency
function formatCurrency(cents: number, currency: string = 'CNY'): string {
  const amount = cents / 100
  if (currency === 'CNY') {
    return `¥${amount.toFixed(2)}`
  }
  return `$${amount.toFixed(2)}`
}

// Format relative time
function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSec = Math.floor(diffMs / 1000)
  const diffMin = Math.floor(diffSec / 60)
  const diffHour = Math.floor(diffMin / 60)
  const diffDay = Math.floor(diffHour / 24)

  if (diffSec < 60) return '刚刚'
  if (diffMin < 60) return `${diffMin}分钟前`
  if (diffHour < 24) return `${diffHour}小时前`
  if (diffDay < 30) return `${diffDay}天前`
  return date.toLocaleDateString('zh-CN')
}

// Format full date
function formatFullDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString('zh-CN')
}

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const configs: Record<string, { color: string; icon: React.ReactNode; label: string }> = {
    paid: { color: 'bg-green-100 text-green-700', icon: <CheckCircle className="h-3 w-3" />, label: '已支付' },
    pending: { color: 'bg-yellow-100 text-yellow-700', icon: <Clock className="h-3 w-3" />, label: '待支付' },
    failed: { color: 'bg-red-100 text-red-700', icon: <XCircle className="h-3 w-3" />, label: '失败' },
    expired: { color: 'bg-gray-100 text-gray-700', icon: <AlertCircle className="h-3 w-3" />, label: '已过期' },
    refunded: { color: 'bg-purple-100 text-purple-700', icon: <AlertCircle className="h-3 w-3" />, label: '已退款' },
    active: { color: 'bg-green-100 text-green-700', icon: <CheckCircle className="h-3 w-3" />, label: '生效中' },
    trial: { color: 'bg-blue-100 text-blue-700', icon: <Clock className="h-3 w-3" />, label: '试用中' },
    cancelled: { color: 'bg-gray-100 text-gray-700', icon: <XCircle className="h-3 w-3" />, label: '已取消' },
  }
  const config = configs[status] || { color: 'bg-gray-100 text-gray-700', icon: null, label: status }

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${config.color}`}>
      {config.icon}
      {config.label}
    </span>
  )
}

export function AdminFinancePage() {
  const { token, user } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()

  // Determine initial tab based on URL
  const getInitialTab = (): 'overview' | 'orders' | 'subscriptions' => {
    if (location.pathname.endsWith('/orders')) return 'orders'
    if (location.pathname.endsWith('/subscriptions')) return 'subscriptions'
    return 'overview'
  }

  // Tab state
  const [activeTab, setActiveTab] = useState<'overview' | 'orders' | 'subscriptions'>(getInitialTab())

  // Handle tab change with URL update
  const handleTabChange = (tab: 'overview' | 'orders' | 'subscriptions') => {
    setActiveTab(tab)
    if (tab === 'overview') {
      navigate('/admin/finance')
    } else {
      navigate(`/admin/finance/${tab}`)
    }
  }

  // Stats state
  const [stats, setStats] = useState<FinanceStats | null>(null)
  const [statsLoading, setStatsLoading] = useState(false)

  // Orders state
  const [orders, setOrders] = useState<OrderListItem[]>([])
  const [ordersTotal, setOrdersTotal] = useState(0)
  const [ordersPage, setOrdersPage] = useState(1)
  const [ordersSearch, setOrdersSearch] = useState('')
  const [ordersStatus, setOrdersStatus] = useState<string>('')
  const [ordersLoading, setOrdersLoading] = useState(false)

  // Subscriptions state
  const [subscriptions, setSubscriptions] = useState<SubscriptionListItem[]>([])
  const [subsTotal, setSubsTotal] = useState(0)
  const [subsPage, setSubsPage] = useState(1)
  const [subsSearch, setSubsSearch] = useState('')
  const [subsStatus, setSubsStatus] = useState<string>('')
  const [subsLoading, setSubsLoading] = useState(false)

  const [error, setError] = useState<string | null>(null)

  const perPage = 20

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  // Fetch stats
  useEffect(() => {
    if (token && activeTab === 'overview') {
      setStatsLoading(true)
      getFinanceStats(token)
        .then(setStats)
        .catch((e) => setError(e.message))
        .finally(() => setStatsLoading(false))
    }
  }, [token, activeTab])

  // Fetch orders
  useEffect(() => {
    if (token && activeTab === 'orders') {
      setOrdersLoading(true)
      listOrders(token, {
        page: ordersPage,
        per_page: perPage,
        status: ordersStatus || undefined,
        search: ordersSearch || undefined,
      })
        .then((res) => {
          setOrders(res.orders)
          setOrdersTotal(res.total)
        })
        .catch((e) => setError(e.message))
        .finally(() => setOrdersLoading(false))
    }
  }, [token, activeTab, ordersPage, ordersStatus, ordersSearch])

  // Fetch subscriptions
  useEffect(() => {
    if (token && activeTab === 'subscriptions') {
      setSubsLoading(true)
      listSubscriptions(token, {
        page: subsPage,
        per_page: perPage,
        status: subsStatus || undefined,
        search: subsSearch || undefined,
      })
        .then((res) => {
          setSubscriptions(res.subscriptions)
          setSubsTotal(res.total)
        })
        .catch((e) => setError(e.message))
        .finally(() => setSubsLoading(false))
    }
  }, [token, activeTab, subsPage, subsStatus, subsSearch])

  const ordersTotalPages = Math.ceil(ordersTotal / perPage)
  const subsTotalPages = Math.ceil(subsTotal / perPage)

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

        {/* Tabs */}
        <div className="flex gap-2 mb-6">
          <Button
            variant={activeTab === 'overview' ? 'default' : 'outline'}
            onClick={() => handleTabChange('overview')}
          >
            <DollarSign className="h-4 w-4 mr-2" />
            概览
          </Button>
          <Button
            variant={activeTab === 'orders' ? 'default' : 'outline'}
            onClick={() => handleTabChange('orders')}
          >
            <ShoppingCart className="h-4 w-4 mr-2" />
            订单
          </Button>
          <Button
            variant={activeTab === 'subscriptions' ? 'default' : 'outline'}
            onClick={() => handleTabChange('subscriptions')}
          >
            <CreditCard className="h-4 w-4 mr-2" />
            订阅
          </Button>
        </div>

        {/* Overview Tab */}
        {activeTab === 'overview' && (
          statsLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
            </div>
          ) : stats ? (
            <div className="space-y-6">
              {/* Stats Cards */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">总收入</CardTitle>
                    <DollarSign className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold text-green-600">
                      {formatCurrency(stats.total_revenue_cents)}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      共 {stats.total_orders} 笔订单
                    </p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">今日收入</CardTitle>
                    <TrendingUp className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {formatCurrency(stats.revenue_today_cents)}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {stats.orders_today} 笔订单
                    </p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">本月收入</CardTitle>
                    <DollarSign className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {formatCurrency(stats.revenue_month_cents)}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {stats.orders_month} 笔订单
                    </p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">活跃订阅</CardTitle>
                    <Users className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">{stats.active_subscriptions}</div>
                    <p className="text-xs text-muted-foreground">
                      试用中: {stats.trial_subscriptions}
                    </p>
                  </CardContent>
                </Card>
              </div>

              {/* Orders by Status */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">订单状态分布</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-4">
                    {stats.orders_by_status.map((item) => (
                      <div key={item.status} className="flex items-center gap-2">
                        <StatusBadge status={item.status} />
                        <span className="text-lg font-semibold">{item.count}</span>
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>
            </div>
          ) : null
        )}

        {/* Orders Tab */}
        {activeTab === 'orders' && (
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between flex-wrap gap-4">
                <CardTitle>订单列表</CardTitle>
                <div className="flex items-center gap-2">
                  <select
                    className="rounded-md border border-gray-300 px-3 py-2 text-sm"
                    value={ordersStatus}
                    onChange={(e) => {
                      setOrdersStatus(e.target.value)
                      setOrdersPage(1)
                    }}
                  >
                    <option value="">全部状态</option>
                    <option value="paid">已支付</option>
                    <option value="pending">待支付</option>
                    <option value="failed">失败</option>
                    <option value="expired">已过期</option>
                    <option value="refunded">已退款</option>
                  </select>
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
                    <Input
                      placeholder="搜索订单号..."
                      value={ordersSearch}
                      onChange={(e) => {
                        setOrdersSearch(e.target.value)
                        setOrdersPage(1)
                      }}
                      className="pl-9 w-48"
                    />
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {ordersLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                </div>
              ) : (
                <>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>订单号</TableHead>
                        <TableHead>用户</TableHead>
                        <TableHead>类型</TableHead>
                        <TableHead>金额</TableHead>
                        <TableHead>状态</TableHead>
                        <TableHead>创建时间</TableHead>
                        <TableHead>操作</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {orders.map((order) => (
                        <TableRow key={order.id}>
                          <TableCell className="font-mono text-sm">{order.order_no}</TableCell>
                          <TableCell>
                            <button
                              className="text-blue-600 hover:text-blue-800 hover:underline text-left"
                              onClick={() => navigate(`/admin/users/${order.user_id}`)}
                            >
                              {order.user_name}
                            </button>
                          </TableCell>
                          <TableCell className="text-sm">
                            {order.order_type === 'subscription' ? '订阅' : order.order_type === 'recharge' ? '充值' : order.order_type}
                          </TableCell>
                          <TableCell className="font-medium">
                            {formatCurrency(order.amount_cents, order.currency)}
                          </TableCell>
                          <TableCell>
                            <StatusBadge status={order.status} />
                          </TableCell>
                          <TableCell
                            className="text-sm text-gray-500 cursor-help"
                            title={formatFullDate(order.created_at)}
                          >
                            {formatRelativeTime(order.created_at)}
                          </TableCell>
                          <TableCell>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => navigate(`/admin/users/${order.user_id}`)}
                            >
                              <Eye className="h-4 w-4" />
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>

                  {/* Pagination */}
                  <div className="flex items-center justify-between mt-4">
                    <div className="text-sm text-gray-500">
                      共 {ordersTotal} 条记录
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setOrdersPage((p) => Math.max(1, p - 1))}
                        disabled={ordersPage === 1}
                      >
                        <ChevronLeft className="h-4 w-4" />
                      </Button>
                      <span className="text-sm">
                        {ordersPage} / {ordersTotalPages || 1}
                      </span>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setOrdersPage((p) => Math.min(ordersTotalPages, p + 1))}
                        disabled={ordersPage >= ordersTotalPages}
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

        {/* Subscriptions Tab */}
        {activeTab === 'subscriptions' && (
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between flex-wrap gap-4">
                <CardTitle>订阅列表</CardTitle>
                <div className="flex items-center gap-2">
                  <select
                    className="rounded-md border border-gray-300 px-3 py-2 text-sm"
                    value={subsStatus}
                    onChange={(e) => {
                      setSubsStatus(e.target.value)
                      setSubsPage(1)
                    }}
                  >
                    <option value="">全部状态</option>
                    <option value="active">生效中</option>
                    <option value="trial">试用中</option>
                    <option value="expired">已过期</option>
                    <option value="cancelled">已取消</option>
                  </select>
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
                    <Input
                      placeholder="搜索用户..."
                      value={subsSearch}
                      onChange={(e) => {
                        setSubsSearch(e.target.value)
                        setSubsPage(1)
                      }}
                      className="pl-9 w-48"
                    />
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {subsLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                </div>
              ) : (
                <>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>用户</TableHead>
                        <TableHead>套餐</TableHead>
                        <TableHead>状态</TableHead>
                        <TableHead>开始时间</TableHead>
                        <TableHead>到期时间</TableHead>
                        <TableHead>自动续费</TableHead>
                        <TableHead>操作</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {subscriptions.map((sub) => (
                        <TableRow key={sub.id}>
                          <TableCell>
                            <button
                              className="text-blue-600 hover:text-blue-800 hover:underline text-left"
                              onClick={() => navigate(`/admin/users/${sub.user_id}`)}
                            >
                              {sub.user_name}
                            </button>
                            {sub.user_email && (
                              <div className="text-xs text-gray-500">{sub.user_email}</div>
                            )}
                          </TableCell>
                          <TableCell>
                            <div className="font-medium">{sub.plan_name}</div>
                            <div className="text-xs text-gray-500">{sub.plan_tier}</div>
                          </TableCell>
                          <TableCell>
                            <StatusBadge status={sub.status} />
                          </TableCell>
                          <TableCell
                            className="text-sm text-gray-500 cursor-help"
                            title={formatFullDate(sub.started_at)}
                          >
                            {formatRelativeTime(sub.started_at)}
                          </TableCell>
                          <TableCell className="text-sm">
                            {sub.expires_at ? (
                              <span
                                className="cursor-help"
                                title={formatFullDate(sub.expires_at)}
                              >
                                {new Date(sub.expires_at).toLocaleDateString('zh-CN')}
                              </span>
                            ) : (
                              <span className="text-gray-400">-</span>
                            )}
                          </TableCell>
                          <TableCell>
                            {sub.auto_renew ? (
                              <span className="text-green-600 text-sm">是</span>
                            ) : (
                              <span className="text-gray-400 text-sm">否</span>
                            )}
                          </TableCell>
                          <TableCell>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => navigate(`/admin/users/${sub.user_id}`)}
                            >
                              <Eye className="h-4 w-4" />
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>

                  {/* Pagination */}
                  <div className="flex items-center justify-between mt-4">
                    <div className="text-sm text-gray-500">
                      共 {subsTotal} 条记录
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setSubsPage((p) => Math.max(1, p - 1))}
                        disabled={subsPage === 1}
                      >
                        <ChevronLeft className="h-4 w-4" />
                      </Button>
                      <span className="text-sm">
                        {subsPage} / {subsTotalPages || 1}
                      </span>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setSubsPage((p) => Math.min(subsTotalPages, p + 1))}
                        disabled={subsPage >= subsTotalPages}
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
    </div>
  )
}
