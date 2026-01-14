import * as React from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { updateMe } from '../lib/api'
import { useAuth } from '../lib/auth'

export function ProfilePanel() {
  const { token, user, setAuth } = useAuth()
  const [name, setName] = React.useState(user?.name || '')
  const [phone, setPhone] = React.useState(user?.phone || '')
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  const [saved, setSaved] = React.useState(false)

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
      <CardHeader>
        <CardTitle>Profile</CardTitle>
        <CardDescription>Manage your learning identity.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="profile-name">Name</Label>
            <Input
              id="profile-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="name"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="profile-phone">Phone</Label>
            <Input
              id="profile-phone"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              autoComplete="tel"
            />
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Button onClick={save} disabled={loading}>
            {loading ? 'Saving…' : 'Save'}
          </Button>
          {saved ? <div className="text-sm text-muted-foreground">Saved.</div> : null}
          {error ? <div className="text-sm text-destructive">{error}</div> : null}
        </div>
      </CardContent>
    </Card>
  )
}

