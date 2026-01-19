import { useState } from 'react'
import { Send, Mic, MicOff, Volume2, RotateCcw, Lightbulb, ChevronDown } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'

interface Message {
  id: string
  role: 'user' | 'assistant'
  content: string
  correction?: string
  timestamp: Date
}

export function ConversationPage() {
  const { token } = useAuth()
  const [messages, setMessages] = useState<Message[]>([
    {
      id: '1',
      role: 'assistant',
      content: "Hi there! I'm your AI English conversation partner. What would you like to talk about today? We could discuss travel, food, work, hobbies, or anything else you're interested in!",
      timestamp: new Date(),
    },
  ])
  const [input, setInput] = useState('')
  const [isRecording, setIsRecording] = useState(false)
  const [selectedScene, setSelectedScene] = useState('自由对话')

  const scenes = ['自由对话', '酒店入住', '餐厅点餐', '商务会议', '旅游问路']

  const handleSend = () => {
    if (!input.trim()) return

    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      content: input,
      timestamp: new Date(),
    }

    setMessages((prev) => [...prev, userMessage])
    setInput('')

    // Simulate AI response
    setTimeout(() => {
      const aiMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: "That's a great point! Let me think about how to respond to that. In English, when we want to express agreement, we often say 'I totally agree' or 'That's exactly what I think'. How about you try using one of these phrases in your next response?",
        timestamp: new Date(),
      }
      setMessages((prev) => [...prev, aiMessage])
    }, 1000)
  }

  const toggleRecording = () => {
    setIsRecording(!isRecording)
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-4xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">💬</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">日常唠嗑</h1>
            <p className="text-gray-600 mb-6">
              与 AI 进行真实的英语对话练习，提升口语表达能力
            </p>
            <Button asChild>
              <a href="/login?redirectTo=/conversation">登录开始对话</a>
            </Button>
          </div>
        </div>
        <Footer />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-4xl p-4">
        <div className="bg-white rounded-xl shadow-lg overflow-hidden">
          {/* Header */}
          <div className="border-b px-6 py-4">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-xl font-semibold text-gray-900">日常唠嗑</h1>
                <p className="text-sm text-gray-500">与 AI 进行自然的英语对话</p>
              </div>
              <div className="flex items-center gap-2">
                <div className="relative">
                  <Button variant="outline" size="sm" className="gap-2">
                    <span>{selectedScene}</span>
                    <ChevronDown className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </div>
          </div>

          {/* Messages */}
          <div className="h-[400px] overflow-y-auto p-6 space-y-4">
            {messages.map((message) => (
              <div
                key={message.id}
                className={cn(
                  'flex',
                  message.role === 'user' ? 'justify-end' : 'justify-start'
                )}
              >
                <div
                  className={cn(
                    'max-w-[80%] rounded-2xl px-4 py-3',
                    message.role === 'user'
                      ? 'bg-orange-500 text-white'
                      : 'bg-gray-100 text-gray-900'
                  )}
                >
                  <p className="text-sm">{message.content}</p>
                  {message.correction && (
                    <div className="mt-2 pt-2 border-t border-orange-400/30">
                      <p className="text-xs opacity-80">
                        <Lightbulb className="h-3 w-3 inline mr-1" />
                        建议: {message.correction}
                      </p>
                    </div>
                  )}
                  <div className={cn(
                    'text-xs mt-1',
                    message.role === 'user' ? 'text-orange-100' : 'text-gray-400'
                  )}>
                    {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  </div>
                </div>
              </div>
            ))}
          </div>

          {/* Quick Actions */}
          <div className="border-t border-b px-6 py-3 bg-gray-50">
            <div className="flex items-center gap-2 text-sm">
              <span className="text-gray-500">快捷短语:</span>
              {['Could you repeat that?', "I'm not sure I understand", 'Let me think...'].map((phrase) => (
                <button
                  key={phrase}
                  onClick={() => setInput(phrase)}
                  className="px-3 py-1 bg-white border rounded-full text-gray-600 hover:bg-orange-50 hover:border-orange-200 transition-colors"
                >
                  {phrase}
                </button>
              ))}
            </div>
          </div>

          {/* Input */}
          <div className="p-4">
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="icon"
                onClick={toggleRecording}
                className={cn(
                  isRecording && 'bg-red-100 border-red-300 text-red-600'
                )}
              >
                {isRecording ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
              </Button>
              <input
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleSend()}
                placeholder="输入你想说的话..."
                className="flex-1 px-4 py-2 border rounded-full focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent"
              />
              <Button onClick={handleSend} disabled={!input.trim()}>
                <Send className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>

        {/* Tips Card */}
        <div className="mt-4 bg-white rounded-xl shadow-lg p-6">
          <h2 className="font-semibold text-gray-900 mb-3">
            <Lightbulb className="h-5 w-5 inline mr-2 text-orange-500" />
            对话技巧
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="bg-orange-50 rounded-lg p-4">
              <div className="font-medium text-gray-800 mb-1">🎯 主动提问</div>
              <p className="text-sm text-gray-600">用 "What do you think about..." 来延续话题</p>
            </div>
            <div className="bg-orange-50 rounded-lg p-4">
              <div className="font-medium text-gray-800 mb-1">🔄 改述练习</div>
              <p className="text-sm text-gray-600">尝试用不同方式表达同一个意思</p>
            </div>
            <div className="bg-orange-50 rounded-lg p-4">
              <div className="font-medium text-gray-800 mb-1">📝 记录新词</div>
              <p className="text-sm text-gray-600">遇到生词及时查询并收藏</p>
            </div>
          </div>
        </div>
      </main>
      <Footer />
    </div>
  )
}
