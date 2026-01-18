import { useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { OAuthButton } from '../components/OAuthButton'
import { OAUTH_PROVIDERS } from '../lib/oauth-config'
import { login, oauthBind, oauthLogin, oauthSkip, register } from '../lib/api'
import { useAuth } from '../lib/auth'

export function AuthCard({ intent }: { intent?: 'desktop' }) {
  const { setAuth } = useAuth()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const redirectTo = searchParams.get('redirectTo') || '/'

  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [needsBind, setNeedsBind] = useState<{
    oauthIdentityId: string
    provider: string
    email: string | null
  } | null>(null)

  const title =
    intent === 'desktop' ? '登录以继续' : '欢迎回来'
  const subtitle =
    intent === 'desktop'
      ? '登录后，您将返回到桌面应用。'
      : '与适应您的 AI 教练一起练习英语。'

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
      navigate(redirectTo)
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Request failed')
    } finally {
      setLoading(false)
    }
  }

  const onOauthLogin = async (provider: string, userId: string, email?: string) => {
    setError(null)
    setNeedsBind(null)
    setLoading(true)
    try {
      const resp = await oauthLogin({
        provider,
        provider_user_id: userId,
        email: email || undefined,
      })
      if (resp.status === 'ok') {
        setAuth(resp.access_token, resp.user)
        navigate(redirectTo)
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
        navigate(redirectTo)
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
        email: needsBind.email || undefined,
      })
      if (resp.status === 'ok') {
        setAuth(resp.access_token, resp.user)
        navigate(redirectTo)
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
      <CardHeader className="pb-3">
        <CardTitle className="text-base">{title}</CardTitle>
        <CardDescription className="text-xs">{subtitle}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <Tabs value={mode} onValueChange={(v) => setMode(v as 'login' | 'register')}>
          <TabsList className="w-full">
            <TabsTrigger value="login" className="w-full text-xs">
              登录
            </TabsTrigger>
            <TabsTrigger value="register" className="w-full text-xs">
              注册
            </TabsTrigger>
          </TabsList>
          <TabsContent value="login" className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="email" className="text-xs">邮箱</Label>
              <Input
                id="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                autoComplete="email"
                className="h-8 text-sm"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="password" className="text-xs">密码</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="current-password"
                className="h-8 text-sm"
              />
            </div>
          </TabsContent>
          <TabsContent value="register" className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="name" className="text-xs">姓名</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                autoComplete="name"
                className="h-8 text-sm"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="email2" className="text-xs">邮箱</Label>
              <Input
                id="email2"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                autoComplete="email"
                className="h-8 text-sm"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="password2" className="text-xs">密码</Label>
              <Input
                id="password2"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="new-password"
                className="h-8 text-sm"
              />
            </div>
          </TabsContent>
        </Tabs>

        {error ? <div className="text-xs text-destructive">{error}</div> : null}

        <Button className="w-full" onClick={onSubmit} disabled={loading} size="sm">
          {loading ? '请稍候...' : mode === 'login' ? '登录' : '创建账户'}
        </Button>

        <div className="rounded-md border p-3 space-y-3">
          <div className="grid gap-2 md:grid-cols-1">
            {Object.entries(OAUTH_PROVIDERS).map(([key, config]) =>
              config.enabled ? (
                <OAuthButton
                  key={key}
                  provider={key as keyof typeof OAUTH_PROVIDERS}
                  onLoginStart={onOauthLogin}
                />
              ) : null
            )}
          </div>
        </div>

        {needsBind ? (
          <div className="space-y-2">
            <div className="text-xs">
              未找到关联的 <span className="font-medium">{needsBind.provider}</span> 账户。
            </div>
            <Tabs defaultValue="bind">
              <TabsList className="w-full">
                <TabsTrigger value="bind" className="w-full text-xs">
                  绑定现有账户
                </TabsTrigger>
                <TabsTrigger value="skip" className="w-full text-xs">
                  跳过（创建新账户）
                </TabsTrigger>
              </TabsList>
              <TabsContent value="bind" className="space-y-2">
                <div className="text-xs text-muted-foreground">
                  输入您现有的邮箱/密码进行绑定。
                </div>
                <Button className="w-full" onClick={onOauthBind} disabled={loading} size="sm">
                  绑定
                </Button>
              </TabsContent>
              <TabsContent value="skip" className="space-y-2">
                <div className="text-xs text-muted-foreground">
                  创建一个新账户并关联此登录方式。
                </div>
                <Button className="w-full" onClick={onOauthSkip} disabled={loading} size="sm">
                  创建并继续
                </Button>
              </TabsContent>
            </Tabs>
          </div>
        ) : null}

        <div className="text-xs text-muted-foreground">
          继续使用即表示您同意勇敢学习：犯错误，快速提升。
        </div>
      </CardContent>
    </Card>
  )
}
