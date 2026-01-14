import * as React from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { login, register } from '../lib/api'
import { useAuth } from '../lib/auth'

export function AuthCard({ intent }: { intent?: 'desktop' }) {
  const { setAuth } = useAuth()
  const [mode, setMode] = React.useState<'login' | 'register'>('login')
  const [email, setEmail] = React.useState('')
  const [password, setPassword] = React.useState('')
  const [name, setName] = React.useState('')
  const [error, setError] = React.useState<string | null>(null)
  const [loading, setLoading] = React.useState(false)

  const title =
    intent === 'desktop' ? 'Sign in to continue' : 'Welcome back'
  const subtitle =
    intent === 'desktop'
      ? 'After signing in, you will return to your desktop app.'
      : 'Practice English with an AI coach that adapts to you.'

  const onSubmit = async () => {
    setError(null)
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

        <div className="text-xs text-muted-foreground">
          By continuing, you agree to learn bravely: make mistakes, improve fast.
        </div>
      </CardContent>
    </Card>
  )
}

