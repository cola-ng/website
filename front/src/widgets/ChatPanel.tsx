import { useState } from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Textarea } from '../components/ui/textarea'
import { textChatSend, type TextIssue } from '../lib/api'
import { useAuth } from '../lib/auth'

type Turn = {
  user: string
  assistant: string
  assistantZh: string
  issues: TextIssue[]
}

export function ChatPanel() {
  const { token } = useAuth()
  const [message, setMessage] = useState('')
  const [turns, setTurns] = useState<Turn[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const send = async () => {
    if (!token) return
    const trimmed = message.trim()
    if (!trimmed) return
    setLoading(true)
    setError(null)
    setMessage('')
    try {
      const resp = await textChatSend(token, trimmed, false)
      setTurns((prev) => [
        ...prev,
        {
          user: trimmed,
          assistant: resp.ai_text_en,
          assistantZh: resp.ai_text_zh,
          issues: resp.issues,
        },
      ])
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to send')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="grid gap-4 lg:grid-cols-5">
      <Card className="lg:col-span-3">
        <CardHeader className="pb-3">
          <CardTitle className="text-base">AI 对话</CardTitle>
          <CardDescription className="text-xs">
            自由对话。教练会引导您并跟踪改进。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="h-[300px] overflow-auto rounded-md border p-2">
            {turns.length === 0 ? (
              <div className="text-xs text-muted-foreground">
                从简单开始："嗨，我想练习口语。"
              </div>
            ) : (
              <div className="space-y-3">
                {turns.map((t, idx) => (
                  <div key={idx} className="space-y-2">
                    <div className="rounded-md bg-muted p-2">
                      <div className="text-xs text-muted-foreground">您</div>
                      <div className="text-xs">{t.user}</div>
                    </div>
                    <div className="rounded-md border p-2">
                      <div className="text-xs text-muted-foreground">教练</div>
                      <div className="text-xs">{t.assistant}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
          <div className="space-y-2">
            <Textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="输入您的消息..."
              className="h-16 text-xs"
            />
            <div className="flex items-center gap-2">
              <Button onClick={send} disabled={loading} size="sm">
                {loading ? '发送中...' : '发送'}
              </Button>
              {error ? <div className="text-xs text-destructive">{error}</div> : null}
            </div>
          </div>
        </CardContent>
      </Card>
      <Card className="lg:col-span-2">
        <CardHeader className="pb-3">
          <CardTitle className="text-base">实时反馈</CardTitle>
          <CardDescription className="text-xs">
            修正和建议会在每轮对话后更新。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {turns.length === 0 ? (
            <div className="text-xs text-muted-foreground">
              发送消息后，反馈将显示在这里。
            </div>
          ) : (
            (() => {
              const last = turns[turns.length - 1]
              return (
                <div className="space-y-3">
                  <div className="space-y-2">
                    <div className="text-xs font-medium">改进建议</div>
                    {last.issues.length === 0 ? (
                      <div className="text-xs text-muted-foreground">没有发现问题。</div>
                    ) : (
                      <ul className="space-y-1 text-xs">
                        {last.issues.map((issue, i) => (
                          <li key={i} className="rounded-md bg-muted p-2 space-y-1">
                            <div>
                              <span className="text-red-500 line-through">{issue.original}</span>
                              <span className="mx-1">→</span>
                              <span className="text-green-600 font-medium">{issue.suggested}</span>
                            </div>
                            <div className="text-muted-foreground">{issue.description_zh}</div>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </div>
              )
            })()
          )}
        </CardContent>
      </Card>
    </div>
  )
}