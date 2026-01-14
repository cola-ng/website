import * as React from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Button } from '../components/ui/button'
import { AuthCard } from '../widgets/AuthCard'
import { useAuth } from '../lib/auth'
import { desktopCreateCode } from '../lib/api'

export function AuthorizePage() {
  const { token } = useAuth()
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const redirectUri = params.get('redirect_uri') || ''
  const state = params.get('state') || ''

  const [status, setStatus] = React.useState<string | null>(null)

  React.useEffect(() => {
    if (!token) return
    if (!redirectUri || !state) return

    let cancelled = false
    setStatus('Creating desktop login code...')
    desktopCreateCode(token, { redirect_uri: redirectUri, state })
      .then((resp) => {
        if (cancelled) return
        const url = new URL(resp.redirect_uri)
        url.searchParams.set('code', resp.code)
        url.searchParams.set('state', resp.state)
        setStatus('Redirecting back to desktop app...')
        window.location.href = url.toString()
      })
      .catch((e: unknown) => {
        if (cancelled) return
        setStatus(e instanceof Error ? e.message : 'Failed to authorize')
      })

    return () => {
      cancelled = true
    }
  }, [token, redirectUri, state])

  if (!redirectUri || !state) {
    return (
      <div className="min-h-screen bg-background">
        <div className="mx-auto flex min-h-screen max-w-6xl items-center justify-center p-6">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Invalid request</CardTitle>
              <CardDescription>
                Missing redirect_uri or state.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button variant="outline" onClick={() => navigate('/')}>
                Back
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>
    )
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-background">
        <div className="mx-auto flex min-h-screen max-w-6xl items-center justify-center p-6">
          <div className="w-full max-w-md">
            <AuthCard intent="desktop" />
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background">
      <div className="mx-auto flex min-h-screen max-w-6xl items-center justify-center p-6">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle>Desktop authorization</CardTitle>
            <CardDescription>
              Finishing sign-in for your desktop app.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="text-sm text-muted-foreground">{status || 'Working...'}</div>
            <Button variant="outline" onClick={() => navigate('/')}>
              Go to website
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

