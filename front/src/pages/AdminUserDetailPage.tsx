import { useState, useEffect } from 'react'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import {
  Users,
  Shield,
  Loader2,
  CreditCard,
  Wallet,
  Crown,
  ArrowLeft,
  ShieldPlus,
  ShieldOff,
  Activity,
} from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card'
import { Input } from '../components/ui/input'
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
  getAdminUser,
  grantAdmin,
  revokeAdmin,
  type UserDetailResponse,
} from '../lib/api'

export function AdminUserDetailPage() {
  const { token, user } = useAuth()
  const navigate = useNavigate()
  const { userId } = useParams<{ userId: string }>()

  const [userDetail, setUserDetail] = useState<UserDetailResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Dialog state for admin actions
  const [grantDialogOpen, setGrantDialogOpen] = useState(false)
  const [revokeDialogOpen, setRevokeDialogOpen] = useState(false)
  const [grantNotes, setGrantNotes] = useState('')
  const [actionLoading, setActionLoading] = useState(false)

  // Redirect non-admin users
  if (!token || !user?.is_admin) {
    return <Navigate to="/" replace />
  }

  useEffect(() => {
    if (token && userId) {
      setLoading(true)
      getAdminUser(token, parseInt(userId, 10))
        .then(setUserDetail)
        .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load user details'))
        .finally(() => setLoading(false))
    }
  }, [token, userId])

  const handleGrantAdmin = async () => {
    if (!token || !userDetail) return
    setActionLoading(true)
    try {
      await grantAdmin(token, userDetail.id, grantNotes || undefined)
      setGrantDialogOpen(false)
      setGrantNotes('')
      // Refresh user detail
      const detail = await getAdminUser(token, userDetail.id)
      setUserDetail(detail)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to grant admin')
    } finally {
      setActionLoading(false)
    }
  }

  const handleRevokeAdmin = async () => {
    if (!token || !userDetail) return
    setActionLoading(true)
    try {
      await revokeAdmin(token, userDetail.id)
      setRevokeDialogOpen(false)
      // Refresh user detail
      const detail = await getAdminUser(token, userDetail.id)
      setUserDetail(detail)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to revoke admin')
    } finally {
      setActionLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Header />

      <main className="max-w-4xl mx-auto px-4 py-6">
        {/* Back button */}
        <Button
          variant="ghost"
          className="mb-4 text-gray-600 hover:text-gray-900"
          onClick={() => navigate('/admin/users')}
        >
          <ArrowLeft className="h-4 w-4 mr-2" />
          返回用户列表
        </Button>

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

        {loading ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
          </div>
        ) : userDetail ? (
          <div className="space-y-6">
            {/* Header Card with basic info and actions */}
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="flex items-center gap-3">
                    <div className="h-12 w-12 bg-gray-200 rounded-full flex items-center justify-center text-xl font-semibold text-gray-600">
                      {userDetail.name.charAt(0).toUpperCase()}
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        {userDetail.name}
                        {userDetail.is_admin && (
                          <span className="inline-flex items-center gap-1 text-purple-600 text-sm bg-purple-50 px-2 py-0.5 rounded">
                            <Shield className="h-3 w-3" /> 管理员
                          </span>
                        )}
                      </div>
                      <div className="text-sm font-normal text-gray-500">
                        ID: {userDetail.id}
                      </div>
                    </div>
                  </CardTitle>
                  <div className="flex items-center gap-2">
                    {userDetail.is_admin ? (
                      <Button
                        variant="outline"
                        className="text-red-600 border-red-200 hover:bg-red-50"
                        onClick={() => setRevokeDialogOpen(true)}
                      >
                        <ShieldOff className="h-4 w-4 mr-2" />
                        撤销管理员
                      </Button>
                    ) : (
                      <Button
                        variant="outline"
                        className="text-purple-600 border-purple-200 hover:bg-purple-50"
                        onClick={() => setGrantDialogOpen(true)}
                      >
                        <ShieldPlus className="h-4 w-4 mr-2" />
                        授予管理员
                      </Button>
                    )}
                  </div>
                </div>
              </CardHeader>
            </Card>

            {/* Basic Info */}
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <Users className="h-4 w-4" /> 基本信息
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-6 text-sm">
                  <div>
                    <div className="text-gray-500 mb-1">用户ID</div>
                    <div className="font-mono">{userDetail.id}</div>
                  </div>
                  <div>
                    <div className="text-gray-500 mb-1">姓名</div>
                    <div>{userDetail.name}</div>
                  </div>
                  <div>
                    <div className="text-gray-500 mb-1">邮箱</div>
                    <div>{userDetail.email || '-'}</div>
                  </div>
                  <div>
                    <div className="text-gray-500 mb-1">手机</div>
                    <div>{userDetail.phone || '-'}</div>
                  </div>
                  <div>
                    <div className="text-gray-500 mb-1">注册时间</div>
                    <div>{new Date(userDetail.created_at).toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-gray-500 mb-1">管理员状态</div>
                    <div>
                      {userDetail.is_admin ? (
                        <span className="text-purple-600">是</span>
                      ) : (
                        <span className="text-gray-400">否</span>
                      )}
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Subscription Info */}
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <Crown className="h-4 w-4" /> 订阅信息
                </CardTitle>
              </CardHeader>
              <CardContent>
                {userDetail.subscription ? (
                  <div className="bg-amber-50 rounded-lg p-4">
                    <div className="grid grid-cols-2 md:grid-cols-3 gap-6 text-sm">
                      <div>
                        <div className="text-gray-500 mb-1">套餐</div>
                        <div className="font-medium text-amber-700">{userDetail.subscription.plan.name_zh}</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">状态</div>
                        <div className={userDetail.subscription.status === 'active' ? 'text-green-600' : 'text-orange-600'}>
                          {userDetail.subscription.status === 'active' ? '活跃' : userDetail.subscription.status === 'trial' ? '试用中' : userDetail.subscription.status}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">开始时间</div>
                        <div>{new Date(userDetail.subscription.started_at).toLocaleDateString()}</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">到期时间</div>
                        <div>{userDetail.subscription.expires_at ? new Date(userDetail.subscription.expires_at).toLocaleDateString() : '永久'}</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">自动续费</div>
                        <div>{userDetail.subscription.auto_renew ? '是' : '否'}</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">价格</div>
                        <div>{userDetail.subscription.plan.price_display || `${userDetail.subscription.plan.price_credits} 积分`}</div>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-gray-500 text-sm">暂无订阅</div>
                )}
              </CardContent>
            </Card>

            {/* Wallet Info */}
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <Wallet className="h-4 w-4" /> 钱包信息
                </CardTitle>
              </CardHeader>
              <CardContent>
                {userDetail.wallet ? (
                  <div className="bg-blue-50 rounded-lg p-4">
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-6 text-sm">
                      <div>
                        <div className="text-gray-500 mb-1">余额</div>
                        <div className="font-medium text-blue-700">{userDetail.wallet.balance} 积分</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">冻结金额</div>
                        <div>{userDetail.wallet.frozen_balance} 积分</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">累计充值</div>
                        <div>{userDetail.wallet.total_deposited} 积分</div>
                      </div>
                      <div>
                        <div className="text-gray-500 mb-1">累计消费</div>
                        <div>{userDetail.wallet.total_spent} 积分</div>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-gray-500 text-sm">暂无钱包</div>
                )}
              </CardContent>
            </Card>

            {/* Recent Payments */}
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <CreditCard className="h-4 w-4" /> 最近支付记录
                </CardTitle>
              </CardHeader>
              <CardContent>
                {userDetail.recent_payments && userDetail.recent_payments.length > 0 ? (
                  <div className="space-y-3">
                    {userDetail.recent_payments.map((payment) => (
                      <div key={payment.id} className="bg-gray-50 rounded-lg p-4 text-sm">
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
                            <div className="font-medium text-lg">{(payment.amount_cents / 100).toFixed(2)}</div>
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
              </CardContent>
            </Card>

            {/* Recent Activities */}
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <Activity className="h-4 w-4" /> 最近活动记录
                </CardTitle>
              </CardHeader>
              <CardContent>
                {userDetail.recent_activities && userDetail.recent_activities.length > 0 ? (
                  <div className="space-y-2">
                    {userDetail.recent_activities.map((activity) => (
                      <div key={activity.id} className="flex items-center justify-between py-2 border-b border-gray-100 last:border-0">
                        <div className="flex items-center gap-3">
                          <span className={`px-2 py-0.5 rounded text-xs ${
                            activity.activity_type === 'login' ? 'bg-blue-100 text-blue-700' :
                            activity.activity_type === 'register' ? 'bg-green-100 text-green-700' :
                            activity.activity_type === 'payment' ? 'bg-amber-100 text-amber-700' :
                            activity.activity_type === 'subscription' ? 'bg-purple-100 text-purple-700' :
                            activity.activity_type === 'admin_grant' ? 'bg-indigo-100 text-indigo-700' :
                            activity.activity_type === 'admin_revoke' ? 'bg-red-100 text-red-700' :
                            'bg-gray-100 text-gray-700'
                          }`}>
                            {activity.activity_type === 'login' ? '登录' :
                             activity.activity_type === 'register' ? '注册' :
                             activity.activity_type === 'payment' ? '支付' :
                             activity.activity_type === 'subscription' ? '订阅' :
                             activity.activity_type === 'admin_grant' ? '授予管理员' :
                             activity.activity_type === 'admin_revoke' ? '撤销管理员' :
                             activity.activity_type}
                          </span>
                          <span className="text-sm text-gray-700">
                            {activity.description || activity.activity_type}
                          </span>
                        </div>
                        <div className="text-xs text-gray-400 flex items-center gap-2">
                          {activity.ip_address && (
                            <span className="font-mono">{activity.ip_address}</span>
                          )}
                          <span>{new Date(activity.created_at).toLocaleString()}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-gray-500 text-sm">暂无活动记录</div>
                )}
              </CardContent>
            </Card>
          </div>
        ) : null}
      </main>

      {/* Grant Admin Dialog */}
      <Dialog open={grantDialogOpen} onOpenChange={setGrantDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>授予管理员权限</DialogTitle>
            <DialogDescription>
              确定要授予用户 "{userDetail?.name}" 管理员权限吗？
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
              确定要撤销用户 "{userDetail?.name}" 的管理员权限吗？此操作不可撤销。
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
