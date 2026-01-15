import * as React from 'react'
import { Navigate, Link } from 'react-router-dom'

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Button } from '../components/ui/button'
import { me } from '../lib/api'
import { useAuth } from '../lib/auth'
import { ProfilePanel } from '../widgets/ProfilePanel'

function formatDate(value?: string | null) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

function InfoRow({ label, value }: { label: string; value?: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1 rounded-lg border bg-card/30 px-3 py-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-sm font-medium break-all">{value ?? '-'}</div>
    </div>
  )
}

export function MePage() {
  const { token, user, setAuth, clear } = useAuth()
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)

  React.useEffect(() => {
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
    <div className="min-h-screen bg-background">
      <header className="border-b">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 rounded-lg bg-primary" />
            <div>
              <div className="text-sm font-semibold leading-tight">Personal profile</div>
              <div className="text-xs text-muted-foreground leading-tight">
                View and update your account information
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" asChild>
              <Link to="/">Home</Link>
            </Button>
            <Button variant="outline" onClick={clear}>
              Log out
            </Button>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl space-y-6 p-6">
        <Card>
          <CardHeader>
            <CardTitle>Account details</CardTitle>
            <CardDescription>Basic information linked to your account.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-3 md:grid-cols-2">
              <InfoRow label="User ID" value={user?.id} />
              <InfoRow label="Email" value={user?.email} />
              <InfoRow label="Name" value={user?.name || '-'} />
              <InfoRow label="Phone" value={user?.phone || '-'} />
              <InfoRow label="Created at" value={formatDate(user?.created_at)} />
              <InfoRow label="Updated at" value={formatDate(user?.updated_at)} />
            </div>
            {loading ? <div className="text-sm text-muted-foreground">Loading…</div> : null}
            {error ? <div className="text-sm text-destructive">{error}</div> : null}
          </CardContent>
        </Card>

        <ProfilePanel />
      </main>
    </div>
  )
}
