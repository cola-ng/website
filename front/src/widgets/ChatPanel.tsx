import { useState, useEffect } from 'react'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Textarea } from '../components/ui/textarea'
import { textChatSend, pollChatTurn, createChat, listChats } from '../lib/api'
import { useAuth } from '../lib/auth'

type TurnStatus = 'sending' | 'processing' | 'completed' | 'error'

type Turn = {
  id: number | null
  aiTurnId: number | null
  user: string
  userZh: string
  assistant: string | null
  assistantZh: string | null
  status: TurnStatus
  error: string | null
}

export function ChatPanel() {
  const { token } = useAuth()
  const [message, setMessage] = useState('')
  const [turns, setTurns] = useState<Turn[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [chatId, setChatId] = useState<number | null>(null)
  const [initializing, setInitializing] = useState(true)

  // Initialize chat on mount - get existing or create new
  useEffect(() => {
    if (!token) {
      setInitializing(false)
      return
    }

    const initChat = async () => {
      try {
        // Try to get existing chats first
        const chats = await listChats(token, 1)
        if (chats.length > 0) {
          setChatId(chats[0].id)
        } else {
          // Create a new chat
          const newChat = await createChat(token, 'Chat Session')
          setChatId(newChat.id)
        }
      } catch (e) {
        console.error('Failed to initialize chat:', e)
        setError('Failed to initialize chat')
      } finally {
        setInitializing(false)
      }
    }

    initChat()
  }, [token])

  const send = async () => {
    if (!token || !chatId) return
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
        id: null,
        aiTurnId: null,
        user: trimmed,
        userZh: '',
        assistant: null,
        assistantZh: null,
        status: 'sending' as const,
        error: null,
      },
    ])

    try {
      // Send message - returns user_turn (completed) and ai_turn (processing)
      const response = await textChatSend(token, chatId, trimmed, false)

      // Update with user turn info, show AI as processing
      setTurns((prev) =>
        prev.map((t, i) =>
          i === turnIndex
            ? {
                ...t,
                id: response.user_turn.id,
                aiTurnId: response.ai_turn.id,
                user: response.user_turn.content_en,
                userZh: response.user_turn.content_zh,
                status: 'processing' as const,
              }
            : t
        )
      )

      // Poll for AI turn to complete
      try {
        const completedAiTurn = await pollChatTurn(token, chatId, response.ai_turn.id, 1000, 60)

        // Update with AI response
        setTurns((prev) =>
          prev.map((t, i) =>
            i === turnIndex
              ? {
                  ...t,
                  assistant: completedAiTurn.content_en,
                  assistantZh: completedAiTurn.content_zh,
                  status: completedAiTurn.status as TurnStatus,
                  error: completedAiTurn.error,
                }
              : t
          )
        )

        if (completedAiTurn.status === 'error') {
          setError(completedAiTurn.error || 'AI response failed')
        }
      } catch (pollError) {
        // Polling failed
        setTurns((prev) =>
          prev.map((t, i) =>
            i === turnIndex
              ? {
                  ...t,
                  status: 'error' as const,
                  error: pollError instanceof Error ? pollError.message : 'Polling failed',
                }
              : t
          )
        )
        setError(pollError instanceof Error ? pollError.message : 'Failed to get AI response')
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to send')
      // Update turn status to error
      setTurns((prev) =>
        prev.map((t, i) =>
          i === turnIndex
            ? { ...t, status: 'error' as const, error: e instanceof Error ? e.message : 'Failed' }
            : t
        )
      )
    } finally {
      setLoading(false)
    }
  }

  if (initializing) {
    return (
      <div className="flex items-center justify-center p-8">
        <span className="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
        <span className="ml-2 text-sm text-muted-foreground">Loading...</span>
      </div>
    )
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
                      {t.status === 'sending' || t.status === 'processing' ? (
                        <div className="text-xs text-muted-foreground flex items-center gap-2">
                          <span className="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
                          AI 思考中...
                        </div>
                      ) : t.status === 'error' ? (
                        <div className="text-xs text-destructive">
                          {t.error || '发生错误'}
                        </div>
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
              <Button onClick={send} disabled={loading || !chatId} size="sm">
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
              const isLoading = last.status === 'sending' || last.status === 'processing'
              return (
                <div className="space-y-3">
                  <div className="space-y-2">
                    <div className="text-xs font-medium">状态</div>
                    {isLoading ? (
                      <div className="text-xs text-muted-foreground flex items-center gap-2">
                        <span className="inline-block w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
                        等待 AI 分析...
                      </div>
                    ) : last.status === 'error' ? (
                      <div className="text-xs text-destructive">{last.error || '发生错误'}</div>
                    ) : (
                      <div className="text-xs text-green-600">AI 响应完成</div>
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
