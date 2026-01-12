import * as React from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Textarea } from '../components/ui/textarea'
import { chatSend } from '../lib/api'
import { useAuth } from '../lib/auth'

type Turn = {
  user: string
  assistant: string
  corrections: string[]
  suggestions: string[]
}

export function ChatPanel() {
  const { token } = useAuth()
  const [message, setMessage] = React.useState('')
  const [turns, setTurns] = React.useState<Turn[]>([])
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)

  const send = async () => {
    if (!token) return
    const trimmed = message.trim()
    if (!trimmed) return
    setLoading(true)
    setError(null)
    setMessage('')
    try {
      const resp = await chatSend(token, trimmed)
      setTurns((prev) => [
        ...prev,
        {
          user: trimmed,
          assistant: resp.reply,
          corrections: resp.corrections,
          suggestions: resp.suggestions,
        },
      ])
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to send')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="grid gap-6 lg:grid-cols-5">
      <Card className="lg:col-span-3">
        <CardHeader>
          <CardTitle>AI Conversation</CardTitle>
          <CardDescription>
            Speak freely. The coach guides you and tracks improvements.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="h-[420px] overflow-auto rounded-md border p-3">
            {turns.length === 0 ? (
              <div className="text-sm text-muted-foreground">
                Start with something simple: “Hi, I want to practice speaking.”
              </div>
            ) : (
              <div className="space-y-4">
                {turns.map((t, idx) => (
                  <div key={idx} className="space-y-2">
                    <div className="rounded-md bg-muted p-3">
                      <div className="text-xs text-muted-foreground">You</div>
                      <div className="text-sm">{t.user}</div>
                    </div>
                    <div className="rounded-md border p-3">
                      <div className="text-xs text-muted-foreground">Coach</div>
                      <div className="text-sm">{t.assistant}</div>
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
              placeholder="Type your message…"
            />
            <div className="flex items-center gap-2">
              <Button onClick={send} disabled={loading}>
                {loading ? 'Sending…' : 'Send'}
              </Button>
              {error ? <div className="text-sm text-destructive">{error}</div> : null}
            </div>
          </div>
        </CardContent>
      </Card>
      <Card className="lg:col-span-2">
        <CardHeader>
          <CardTitle>Live Feedback</CardTitle>
          <CardDescription>
            Corrections and suggestions update after each turn.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {turns.length === 0 ? (
            <div className="text-sm text-muted-foreground">
              After you send a message, feedback will appear here.
            </div>
          ) : (
            (() => {
              const last = turns[turns.length - 1]
              return (
                <div className="space-y-4">
                  <div className="space-y-2">
                    <div className="text-sm font-medium">Corrections</div>
                    {last.corrections.length === 0 ? (
                      <div className="text-sm text-muted-foreground">No issues spotted.</div>
                    ) : (
                      <ul className="space-y-1 text-sm">
                        {last.corrections.map((c, i) => (
                          <li key={i} className="rounded-md bg-muted p-2">
                            {c}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                  <div className="space-y-2">
                    <div className="text-sm font-medium">Suggestions</div>
                    <ul className="space-y-1 text-sm">
                      {last.suggestions.map((s, i) => (
                        <li key={i} className="rounded-md border p-2">
                          {s}
                        </li>
                      ))}
                    </ul>
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

