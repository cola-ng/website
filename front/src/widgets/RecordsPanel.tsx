import * as React from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { listRecords } from '../lib/api'
import { useAuth } from '../lib/auth'

export function RecordsPanel() {
  const { token } = useAuth()
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  const [records, setRecords] = React.useState<Array<{
    id: string
    record_type: string
    created_at: string
    content: unknown
  }>>([])

  const load = React.useCallback(async () => {
    if (!token) return
    setLoading(true)
    setError(null)
    try {
      const resp = await listRecords(token, 50)
      setRecords(
        resp.map((r) => ({
          id: r.id,
          record_type: r.record_type,
          created_at: r.created_at,
          content: r.content,
        }))
      )
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to load')
    } finally {
      setLoading(false)
    }
  }, [token])

  React.useEffect(() => {
    void load()
  }, [load])

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between space-y-0 pb-3">
        <div className="space-y-0.5">
          <CardTitle className="text-base">学习记录</CardTitle>
          <CardDescription className="text-xs">
            对话轮次、错误和未来的复习计划将显示在这里。
          </CardDescription>
        </div>
        <Button variant="outline" onClick={load} disabled={loading} size="sm">
          {loading ? '刷新中...' : '刷新'}
        </Button>
      </CardHeader>
      <CardContent>
        {error ? <div className="text-xs text-destructive">{error}</div> : null}
        <div className="mt-3 space-y-2">
          {records.length === 0 ? (
            <div className="text-xs text-muted-foreground">
              暂无记录。发送聊天消息以创建记录。
            </div>
          ) : (
            records.map((r) => (
              <div key={r.id} className="rounded-md border p-2">
                <div className="flex items-center justify-between">
                  <div className="text-xs font-medium">{r.record_type}</div>
                  <div className="text-xs text-muted-foreground">
                    {new Date(r.created_at).toLocaleString()}
                  </div>
                </div>
                <pre className="mt-1 overflow-auto rounded-md bg-muted p-1.5 text-xs">
                  {JSON.stringify(r.content, null, 2)}
                </pre>
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  )
}

