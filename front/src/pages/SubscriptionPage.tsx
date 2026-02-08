import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  Wallet,
  Crown,
  Check,
  Loader2,
  AlertCircle,
  Sparkles,
  ArrowRight,
} from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
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
  getWallet,
  listSubscriptionPlans,
  getCurrentSubscription,
  subscribe,
  cancelSubscription,
  type WalletInfo,
  type SubscriptionPlan,
  type CurrentSubscription,
} from '../lib/api'
import { cn } from '../lib/utils'

export function SubscriptionPage() {
  const { token } = useAuth()
  const navigate = useNavigate()

  const [wallet, setWallet] = useState<WalletInfo | null>(null)
  const [plans, setPlans] = useState<SubscriptionPlan[]>([])
  const [currentSub, setCurrentSub] = useState<CurrentSubscription | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Dialog state
  const [subscribeDialogOpen, setSubscribeDialogOpen] = useState(false)
  const [cancelDialogOpen, setCancelDialogOpen] = useState(false)
  const [selectedPlan, setSelectedPlan] = useState<SubscriptionPlan | null>(null)
  const [actionLoading, setActionLoading] = useState(false)

  useEffect(() => {
    if (!token) {
      navigate('/login?redirectTo=/subscription')
      return
    }

    const fetchData = async () => {
      setLoading(true)
      try {
        const [walletData, plansData, subData] = await Promise.all([
          getWallet(token),
          listSubscriptionPlans(token),
          getCurrentSubscription(token),
        ])
        setWallet(walletData)
        setPlans(plansData)
        setCurrentSub(subData)
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load data')
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [token, navigate])

  const handleSubscribe = async () => {
    if (!token || !selectedPlan) return

    setActionLoading(true)
    try {
      await subscribe(token, selectedPlan.code)
      // Refresh subscription status
      const subData = await getCurrentSubscription(token)
      setCurrentSub(subData)
      setSubscribeDialogOpen(false)
      setSelectedPlan(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to subscribe')
    } finally {
      setActionLoading(false)
    }
  }

  const handleCancel = async () => {
    if (!token) return

    setActionLoading(true)
    try {
      await cancelSubscription(token)
      // Refresh subscription status
      const subData = await getCurrentSubscription(token)
      setCurrentSub(subData)
      setCancelDialogOpen(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to cancel subscription')
    } finally {
      setActionLoading(false)
    }
  }

  const formatCredits = (credits: number) => {
    return `¥${(credits / 100).toFixed(2)}`
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Header />

      <main className="max-w-4xl mx-auto px-4 py-8">
        <div className="mb-8 text-center">
          <h1 className="text-3xl font-bold text-gray-900 flex items-center justify-center gap-2">
            <Crown className="h-8 w-8 text-amber-500" />
            订阅会员
          </h1>
          <p className="text-gray-500 mt-2">解锁全部功能，无限学习</p>
        </div>

        {error && (
          <div className="mb-6 p-4 bg-red-50 text-red-600 rounded-lg flex items-center gap-2">
            <AlertCircle className="h-5 w-5" />
            {error}
            <Button
              variant="ghost"
              size="sm"
              className="ml-auto"
              onClick={() => setError(null)}
            >
              关闭
            </Button>
          </div>
        )}

        {/* Wallet Balance */}
        {wallet && (
          <Card className="mb-8">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <div className="flex items-center gap-2">
                <Wallet className="h-5 w-5 text-green-600" />
                <CardTitle className="text-lg">我的钱包</CardTitle>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex items-baseline gap-2">
                <span className="text-3xl font-bold text-green-600">
                  {formatCredits(wallet.account.balance)}
                </span>
                <span className="text-gray-500">可用余额</span>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Current Subscription */}
        {currentSub?.has_subscription && currentSub.subscription && (
          <Card className="mb-8 border-amber-200 bg-amber-50/50">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Sparkles className="h-5 w-5 text-amber-500" />
                  <CardTitle className="text-lg">当前订阅</CardTitle>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-xl font-bold text-gray-900">
                    {currentSub.subscription.plan.name_zh}
                  </p>
                  {currentSub.days_remaining !== null && (
                    <p className="text-sm text-gray-500">
                      剩余 {currentSub.days_remaining} 天
                    </p>
                  )}
                </div>
                {currentSub.subscription.status !== 'cancelled' && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setCancelDialogOpen(true)}
                  >
                    取消订阅
                  </Button>
                )}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Subscription Plans */}
        <div className="grid md:grid-cols-3 gap-6">
          {plans.map((plan) => {
            const isCurrentPlan =
              currentSub?.subscription?.plan.code === plan.code
            const isPro = plan.tier === 'pro'
            const features = (plan.features as string[]) || []

            return (
              <Card
                key={plan.id}
                className={cn(
                  'relative overflow-hidden transition-all',
                  isPro && 'border-amber-300 shadow-lg',
                  isCurrentPlan && 'ring-2 ring-amber-500'
                )}
              >
                {isPro && plan.billing_cycle === 'yearly' && (
                  <div className="absolute top-0 right-0 bg-gradient-to-r from-amber-500 to-orange-500 text-white text-xs font-bold px-3 py-1 rounded-bl-lg">
                    最划算
                  </div>
                )}
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    {isPro && <Crown className="h-5 w-5 text-amber-500" />}
                    {plan.name_zh}
                  </CardTitle>
                  <CardDescription>{plan.description_zh}</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="mb-4">
                    <span className="text-3xl font-bold">
                      {plan.price_display || '免费'}
                    </span>
                  </div>

                  <ul className="space-y-2 mb-6">
                    {features.map((feature, i) => (
                      <li key={i} className="flex items-center gap-2 text-sm">
                        <Check className="h-4 w-4 text-green-500 flex-shrink-0" />
                        <span>{feature}</span>
                      </li>
                    ))}
                  </ul>

                  {isCurrentPlan ? (
                    <Button disabled className="w-full">
                      当前方案
                    </Button>
                  ) : plan.tier === 'free' ? (
                    <Button
                      variant="outline"
                      className="w-full"
                      onClick={() => {
                        setSelectedPlan(plan)
                        setSubscribeDialogOpen(true)
                      }}
                    >
                      使用免费版
                    </Button>
                  ) : (
                    <Button
                      className="w-full bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-600 hover:to-orange-600"
                      onClick={() => {
                        setSelectedPlan(plan)
                        setSubscribeDialogOpen(true)
                      }}
                    >
                      立即订阅
                      <ArrowRight className="h-4 w-4 ml-1" />
                    </Button>
                  )}
                </CardContent>
              </Card>
            )
          })}
        </div>
      </main>

      {/* Subscribe Dialog */}
      <Dialog open={subscribeDialogOpen} onOpenChange={setSubscribeDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认订阅</DialogTitle>
            <DialogDescription>
              您将订阅 "{selectedPlan?.name_zh}"
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <div className="text-2xl font-bold text-center">
              {selectedPlan?.price_display || '免费'}
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSubscribeDialogOpen(false)}>
              取消
            </Button>
            <Button
              onClick={handleSubscribe}
              disabled={actionLoading}
              className="bg-gradient-to-r from-amber-500 to-orange-500"
            >
              {actionLoading ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : null}
              确认订阅
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Cancel Dialog */}
      <Dialog open={cancelDialogOpen} onOpenChange={setCancelDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>取消订阅</DialogTitle>
            <DialogDescription>
              确定要取消订阅吗？取消后您将失去专业版功能。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCancelDialogOpen(false)}>
              保留订阅
            </Button>
            <Button
              variant="destructive"
              onClick={handleCancel}
              disabled={actionLoading}
            >
              {actionLoading ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : null}
              确认取消
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
