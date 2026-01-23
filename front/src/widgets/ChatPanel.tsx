import { useState } from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Textarea } from '../components/ui/textarea'
import { textChatSendAsync, pollChatResponse, type TextIssue, type ResponseStatus } from '../lib/api'
import { useAuth } from '../lib/auth'

type Turn = {
  user: string
  userZh: string
  assistant: string | null
  assistantZh: string | null
  issues: TextIssue[]
  status: ResponseStatus | 'sending'
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

    // Add pending turn immediately
    const turnIndex = turns.length
    setTurns((prev) => [
      ...prev,
      {
        user: trimmed,
        userZh: '',
        assistant: null,
        assistantZh: null,
        issues: [],
        status: 'sending' as const,
      },
    ])

    try {
      // Send message and get user translation immediately
      const sendResp = await textChatSendAsync(token, trimmed, false)

      // Update with user translation, show pending AI response
      setTurns((prev) =>
        prev.map((t, i) =>
          i === turnIndex
            ? {
                ...t,
                user: sendResp.user_text_en,
                userZh: sendResp.user_text_zh,
                status: 'pending' as const,
              }
            : t
        )
      )

      // Poll for AI response
      const pollResp = await pollChatResponse(token, sendResp.turn_id)

      // Update with AI response
      setTurns((prev) =>
        prev.map((t, i) =>
          i === turnIndex
            ? {
                ...t,
                assistant: pollResp.ai_text_en,
                assistantZh: pollResp.ai_text_zh,
                issues: pollResp.issues || [],
                status: pollResp.status,
              }
            : t
        )
      )

      if (pollResp.status === 'error') {
        setError(pollResp.error_message || 'AI response failed')
      } else if (pollResp.status === 'timeout') {
        setError(pollResp.error_message || 'Server response too slow, please retry')
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to send')
      // Update turn status to error
      setTurns((prev) =>
        prev.map((t, i) =>
          i === turnIndex ? { ...t, status: 'error' as const } : t
        )
      )
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
                      {t.userZh && t.userZh !== t.user && (
                        <div className="text-xs text-muted-foreground mt-1">{t.userZh}</div>
                      )}
                    </div>
                    <div className="rounded-md border p-2">
                      <div className="text-xs text-muted-foreground">教练</div>
                      {t.status === 'sending' || t.status === 'pending' || t.status === 'processing' ? (
                        <div className="text-xs text-muted-foreground flex items-center gap-2">
                          <span className="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
                          {t.status === 'sending' ? '发送中...' : 'AI 思考中...'}
                        </div>
                      ) : t.status === 'timeout' ? (
                        <div className="text-xs text-orange-500 flex items-center gap-2">
                          <span>服务器响应过慢</span>
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-6 text-xs"
                            onClick={() => {
                              // Retry logic could be added here
                            }}
                          >
                            重试
                          </Button>
                        </div>
                      ) : t.status === 'error' ? (
                        <div className="text-xs text-destructive">发生错误</div>
                      ) : (
                        <div className="text-xs">{t.assistant}</div>
                      )}
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
              const isLoading = last.status === 'sending' || last.status === 'pending' || last.status === 'processing'
              return (
                <div className="space-y-3">
                  <div className="space-y-2">
                    <div className="text-xs font-medium">改进建议</div>
                    {isLoading ? (
                      <div className="text-xs text-muted-foreground flex items-center gap-2">
                        <span className="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
                        等待 AI 分析...
                      </div>
                    ) : last.issues.length === 0 ? (
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