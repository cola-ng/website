import { useEffect, useState } from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { updateMe } from '../lib/api'
import { useAuth } from '../lib/auth'

export function ProfilePanel() {
  const { token, user, setAuth } = useAuth()
  const [name, setName] = useState(user?.name || '')
  const [phone, setPhone] = useState(user?.phone || '')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    setName(user?.name || '')
    setPhone(user?.phone || '')
  }, [user?.name, user?.phone])

  const save = async () => {
    if (!token || !user) return
    setLoading(true)
    setError(null)
    setSaved(false)
    try {
      const next = await updateMe(token, {
        name: name || null,
        phone: phone || null,
      })
      setAuth(token, next)
      setSaved(true)
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to save')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base">个人资料</CardTitle>
        <CardDescription className="text-xs">管理您的学习身份</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-3 md:grid-cols-2">
          <div className="space-y-1.5">
            <Label htmlFor="profile-name" className="text-xs">姓名</Label>
            <Input
              id="profile-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="name"
              className="h-8 text-sm"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="profile-phone" className="text-xs">电话</Label>
            <Input
              id="profile-phone"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              autoComplete="tel"
              className="h-8 text-sm"
            />
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button onClick={save} disabled={loading} size="sm">
            {loading ? '保存中...' : '保存'}
          </Button>
          {saved ? <div className="text-xs text-muted-foreground">已保存</div> : null}
          {error ? <div className="text-xs text-destructive">{error}</div> : null}
        </div>
      </CardContent>
    </Card>
  )
}

