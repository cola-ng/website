import * as React from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { login, oauthBind, oauthLogin, oauthSkip, register } from '../lib/api'
import { useAuth } from '../lib/auth'

export function AuthCard({ intent }: { intent?: 'desktop' }) {
  const { setAuth } = useAuth()
  const [mode, setMode] = React.useState<'login' | 'register'>('login')
  const [email, setEmail] = React.useState('')
  const [password, setPassword] = React.useState('')
  const [name, setName] = React.useState('')
  const [error, setError] = React.useState<string | null>(null)
  const [loading, setLoading] = React.useState(false)
  const [oauthProvider, setOauthProvider] = React.useState('google')
  const [oauthProviderUserId, setOauthProviderUserId] = React.useState('')
  const [oauthEmail, setOauthEmail] = React.useState('')
  const [needsBind, setNeedsBind] = React.useState<{
    oauthIdentityId: string
    provider: string
    email: string | null
  } | null>(null)

  const title =
    intent === 'desktop' ? 'Sign in to continue' : 'Welcome back'
  const subtitle =
    intent === 'desktop'
      ? 'After signing in, you will return to your desktop app.'
      : 'Practice English with an AI coach that adapts to you.'

  const onSubmit = async () => {
    setError(null)
    setNeedsBind(null)
    setLoading(true)
    try {
      if (mode === 'login') {
        const resp = await login({ email, password })
        setAuth(resp.access_token, resp.user)
      } else {
        const resp = await register({ email, password, name: name || undefined })
        setAuth(resp.access_token, resp.user)
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Request failed')
    } finally {
      setLoading(false)
    }
  }

  const onOauthLogin = async () => {
    setError(null)
    setNeedsBind(null)
    setLoading(true)
    try {
      const resp = await oauthLogin({
        provider: oauthProvider,
        provider_user_id: oauthProviderUserId,
        email: oauthEmail || undefined,
      })
      if (resp.status === 'ok') {
        setAuth(resp.access_token, resp.user)
        return
      }
      setNeedsBind({
        oauthIdentityId: resp.oauth_identity_id,
        provider: resp.provider,
        email: resp.email,
      })
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Request failed')
    } finally {
      setLoading(false)
    }
  }

  const onOauthBind = async () => {
    if (!needsBind) return
    setError(null)
    setLoading(true)
    try {
      const resp = await oauthBind({
        oauth_identity_id: needsBind.oauthIdentityId,
        email,
        password,
      })
      if (resp.status === 'ok') {
        setAuth(resp.access_token, resp.user)
      } else {
        setNeedsBind({
          oauthIdentityId: resp.oauth_identity_id,
          provider: resp.provider,
          email: resp.email,
        })
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Bind failed')
    } finally {
      setLoading(false)
    }
  }

  const onOauthSkip = async () => {
    if (!needsBind) return
    setError(null)
    setLoading(true)
    try {
      const resp = await oauthSkip({
        oauth_identity_id: needsBind.oauthIdentityId,
        name: name || undefined,
        email: oauthEmail || needsBind.email || undefined,
      })
      if (resp.status === 'ok') {
        setAuth(resp.access_token, resp.user)
      } else {
        setNeedsBind({
          oauthIdentityId: resp.oauth_identity_id,
          provider: resp.provider,
          email: resp.email,
        })
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Skip failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{subtitle}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Tabs value={mode} onValueChange={(v) => setMode(v as 'login' | 'register')}>
          <TabsList className="w-full">
            <TabsTrigger value="login" className="w-full">
              Login
            </TabsTrigger>
            <TabsTrigger value="register" className="w-full">
              Register
            </TabsTrigger>
          </TabsList>
          <TabsContent value="login" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                autoComplete="email"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="current-password"
              />
            </div>
          </TabsContent>
          <TabsContent value="register" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                autoComplete="name"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="email2">Email</Label>
              <Input
                id="email2"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                autoComplete="email"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password2">Password</Label>
              <Input
                id="password2"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="new-password"
              />
            </div>
          </TabsContent>
        </Tabs>

        {error ? <div className="text-sm text-destructive">{error}</div> : null}

        <Button className="w-full" onClick={onSubmit} disabled={loading}>
          {loading ? 'Please wait…' : mode === 'login' ? 'Login' : 'Create account'}
        </Button>

        <div className="rounded-md border p-4 space-y-4">
          <div className="space-y-1">
            <div className="text-sm font-medium">Third-party login</div>
            <div className="text-xs text-muted-foreground">
              If the account is new, choose bind or skip.
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="oauth-provider">Provider</Label>
              <Input
                id="oauth-provider"
                value={oauthProvider}
                onChange={(e) => setOauthProvider(e.target.value)}
                placeholder="google"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="oauth-subject">Provider user id</Label>
              <Input
                id="oauth-subject"
                value={oauthProviderUserId}
                onChange={(e) => setOauthProviderUserId(e.target.value)}
                placeholder="subject / openid sub"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="oauth-email">Email (optional)</Label>
            <Input
              id="oauth-email"
              value={oauthEmail}
              onChange={(e) => setOauthEmail(e.target.value)}
              autoComplete="email"
            />
          </div>

          <Button
            variant="secondary"
            className="w-full"
            onClick={onOauthLogin}
            disabled={loading}
          >
            Continue with {oauthProvider || 'provider'}
          </Button>

          {needsBind ? (
            <div className="space-y-3">
              <div className="text-sm">
                No linked account for <span className="font-medium">{needsBind.provider}</span>.
              </div>
              <Tabs defaultValue="bind">
                <TabsList className="w-full">
                  <TabsTrigger value="bind" className="w-full">
                    Bind existing
                  </TabsTrigger>
                  <TabsTrigger value="skip" className="w-full">
                    Skip (create new)
                  </TabsTrigger>
                </TabsList>
                <TabsContent value="bind" className="space-y-3">
                  <div className="text-xs text-muted-foreground">
                    Enter your existing email/password to bind.
                  </div>
                  <Button className="w-full" onClick={onOauthBind} disabled={loading}>
                    Bind
                  </Button>
                </TabsContent>
                <TabsContent value="skip" className="space-y-3">
                  <div className="text-xs text-muted-foreground">
                    Create a new account and link this login.
                  </div>
                  <Button className="w-full" onClick={onOauthSkip} disabled={loading}>
                    Create & continue
                  </Button>
                </TabsContent>
              </Tabs>
            </div>
          ) : null}
        </div>

        <div className="text-xs text-muted-foreground">
          By continuing, you agree to learn bravely: make mistakes, improve fast.
        </div>
      </CardContent>
    </Card>
  )
}
