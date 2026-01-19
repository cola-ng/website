import { useEffect, useState, type ReactNode } from 'react'
import { Navigate } from 'react-router-dom'

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { me } from '../lib/api'
import { useAuth } from '../lib/auth'
import { ProfilePanel } from '../widgets/ProfilePanel'

function formatDate(value?: string | null) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

function InfoRow({ label, value }: { label: string; value?: ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5 rounded-lg border bg-card/30 px-2 py-1.5">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-xs font-medium break-all">{value ?? '-'}</div>
    </div>
  )
}

export function MePage() {
  const { token, user, setAuth } = useAuth()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!token) return
    let cancelled = false
    setLoading(true)
    setError(null)

    me(token)
      .then((profile) => {
        if (cancelled) return
        setAuth(token, profile)
      })
      .catch((e: unknown) => {
        if (cancelled) return
        setError(e instanceof Error ? e.message : 'Failed to load profile')
      })
      .finally(() => {
        if (cancelled) return
        setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [token, setAuth])

  if (!token) {
    return <Navigate to="/" replace />
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-6xl space-y-4 p-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-lg">账户详情</CardTitle>
            <CardDescription className="text-sm">与您账户关联的基本信息</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="grid gap-2 md:grid-cols-2">
              <InfoRow label="用户 ID" value={user?.id} />
              <InfoRow label="邮箱" value={user?.email} />
              <InfoRow label="姓名" value={user?.name || '-'} />
              <InfoRow label="电话" value={user?.phone || '-'} />
              <InfoRow label="创建时间" value={formatDate(user?.created_at)} />
              <InfoRow label="更新时间" value={formatDate(user?.updated_at)} />
            </div>
            {loading ? <div className="text-xs text-muted-foreground">加载中...</div> : null}
            {error ? <div className="text-xs text-destructive">{error}</div> : null}
          </CardContent>
        </Card>

        <ProfilePanel />
      </main>
      <Footer />
    </div>
  )
}
