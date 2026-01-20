import { useState, useRef, useEffect, useCallback } from 'react'
import { Send, Mic, MicOff, Plus, Volume2, MessageSquare, Settings2, ChevronDown, FileDown, ClipboardList, Loader2, Square } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'
import { voiceChatSend, textChatSend, textToSpeech, type HistoryMessage } from '../lib/api'

interface Correction {
  original: string
  corrected: string
  explanation: string
}

interface Message {
  id: string
  role: 'user' | 'assistant'
  contentEn: string
  contentZh: string
  hasAudio?: boolean
  audioBase64?: string
  timestamp: Date
  corrections?: Correction[]
}

interface Conversation {
  id: string
  title: string
  lastMessage: string
  timestamp: Date
  messages: Message[]
}

// Mock data for conversations
const mockConversations: Conversation[] = [
  {
    id: '1',
    title: '旅行计划讨论',
    lastMessage: "That sounds like a great trip!",
    timestamp: new Date(Date.now() - 1000 * 60 * 30),
    messages: [
      {
        id: '1-1',
        role: 'assistant',
        contentEn: "Hi! I heard you're planning a trip. Where are you thinking of going?",
        contentZh: "嗨！我听说你在计划旅行。你打算去哪里？",
        timestamp: new Date(Date.now() - 1000 * 60 * 35),
      },
      {
        id: '1-2',
        role: 'user',
        contentEn: "I want to visit Japan next month.",
        contentZh: "我想下个月去日本。",
        hasAudio: true,
        timestamp: new Date(Date.now() - 1000 * 60 * 33),
        corrections: [
          {
            original: "I want to visit",
            corrected: "I'm planning to visit",
            explanation: "使用 'planning to' 表达计划更自然，比 'want to' 更正式"
          }
        ]
      },
      {
        id: '1-3',
        role: 'assistant',
        contentEn: "That sounds like a great trip! Japan is beautiful in spring.",
        contentZh: "听起来是个很棒的旅行！日本的春天很美。",
        timestamp: new Date(Date.now() - 1000 * 60 * 30),
      },
    ],
  },
  {
    id: '2',
    title: '工作面试准备',
    lastMessage: "Let's practice some common questions.",
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 2),
    messages: [
      {
        id: '2-1',
        role: 'assistant',
        contentEn: "Hello! I understand you have a job interview coming up. How can I help you prepare?",
        contentZh: "你好！我了解到你即将有一场工作面试。我能帮你准备什么？",
        timestamp: new Date(Date.now() - 1000 * 60 * 60 * 2),
      },
    ],
  },
  {
    id: '3',
    title: '餐厅点餐练习',
    lastMessage: "Would you like to see the menu?",
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 24),
    messages: [
      {
        id: '3-1',
        role: 'assistant',
        contentEn: "Welcome to our restaurant! Would you like to see the menu?",
        contentZh: "欢迎来到我们的餐厅！您想看看菜单吗？",
        timestamp: new Date(Date.now() - 1000 * 60 * 60 * 24),
      },
    ],
  },
]

export function ConversationPage() {
  const { token } = useAuth()
  const [conversations, setConversations] = useState<Conversation[]>(mockConversations)
  const [activeConversationId, setActiveConversationId] = useState<string>(mockConversations[0].id)
  const [input, setInput] = useState('')
  const [isRecording, setIsRecording] = useState(false)
  const [showAudioSettings, setShowAudioSettings] = useState(false)
  const [selectedMic, setSelectedMic] = useState('default')
  const [selectedSpeaker, setSelectedSpeaker] = useState('default')
  const [reportMode, setReportMode] = useState(false)
  const [isProcessing, setIsProcessing] = useState(false)
  const [isPlayingAudio, setIsPlayingAudio] = useState<string | null>(null)
  const [recordingDuration, setRecordingDuration] = useState(0)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const audioSettingsRef = useRef<HTMLDivElement>(null)
  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const audioElementRef = useRef<HTMLAudioElement | null>(null)

  // Display settings: 'both' | 'en' | 'zh'
  const [botLang, setBotLang] = useState<'both' | 'en' | 'zh'>('both')
  const [userLang, setUserLang] = useState<'both' | 'en' | 'zh'>('both')

  // Derived display settings
  const showBotEn = botLang === 'both' || botLang === 'en'
  const showBotZh = botLang === 'both' || botLang === 'zh'
  const showUserEn = userLang === 'both' || userLang === 'en'
  const showUserZh = userLang === 'both' || userLang === 'zh'

  const activeConversation = conversations.find(c => c.id === activeConversationId)
  const messages = activeConversation?.messages || []
  const prevMessagesLengthRef = useRef(messages.length)

  // Only scroll when new messages are added, not on initial load
  useEffect(() => {
    if (messages.length > prevMessagesLengthRef.current) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
    }
    prevMessagesLengthRef.current = messages.length
  }, [messages.length])

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      // Reset to single line height first to get accurate scrollHeight
      textareaRef.current.style.height = '36px'
      const scrollHeight = textareaRef.current.scrollHeight
      // Only expand if content exceeds single line
      if (scrollHeight > 36) {
        textareaRef.current.style.height = Math.min(scrollHeight, 120) + 'px'
      }
    }
  }, [input])

  // Close audio settings when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (audioSettingsRef.current && !audioSettingsRef.current.contains(event.target as Node)) {
        setShowAudioSettings(false)
      }
    }
    if (showAudioSettings) {
      document.addEventListener('mousedown', handleClickOutside)
    }
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [showAudioSettings])

  // Helper function to convert conversation history for API
  const getConversationHistory = useCallback((): HistoryMessage[] => {
    if (!activeConversation) return []
    // Get last 10 messages for context
    return activeConversation.messages.slice(-10).map(msg => ({
      role: msg.role,
      content: msg.contentEn,
    }))
  }, [activeConversation])

  // Helper function to play audio from base64
  const playAudioFromBase64 = useCallback((base64: string, messageId: string) => {
    if (audioElementRef.current) {
      audioElementRef.current.pause()
    }

    const audio = new Audio(`data:audio/wav;base64,${base64}`)
    audioElementRef.current = audio
    setIsPlayingAudio(messageId)

    audio.onended = () => {
      setIsPlayingAudio(null)
    }
    audio.onerror = () => {
      setIsPlayingAudio(null)
      console.error('Audio playback failed')
    }
    audio.play().catch(err => {
      console.error('Failed to play audio:', err)
      setIsPlayingAudio(null)
    })
  }, [])

  // Stop audio playback
  const stopAudio = useCallback(() => {
    if (audioElementRef.current) {
      audioElementRef.current.pause()
      audioElementRef.current = null
    }
    setIsPlayingAudio(null)
  }, [])

  const handleSend = async () => {
    if (!input.trim() || !activeConversation || !token || isProcessing) return

    const messageText = input.trim()
    setInput('')
    setIsProcessing(true)

    // Add user message immediately
    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      contentEn: messageText,
      contentZh: messageText,
      timestamp: new Date(),
    }

    setConversations(prev => prev.map(c => {
      if (c.id === activeConversationId) {
        return {
          ...c,
          messages: [...c.messages, userMessage],
          lastMessage: messageText,
          timestamp: new Date(),
        }
      }
      return c
    }))

    try {
      // Call API for text chat
      const history = getConversationHistory()
      const response = await textChatSend(token, messageText, history, true)

      // Add AI response
      const aiMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        contentEn: response.ai_text,
        contentZh: response.ai_text_zh || response.ai_text,
        hasAudio: !!response.ai_audio_base64,
        audioBase64: response.ai_audio_base64 || undefined,
        timestamp: new Date(),
      }

      setConversations(prev => prev.map(c => {
        if (c.id === activeConversationId) {
          // Also update user message with corrections if any
          const updatedMessages = c.messages.map(m =>
            m.id === userMessage.id && response.corrections.length > 0
              ? { ...m, corrections: response.corrections }
              : m
          )
          return {
            ...c,
            messages: [...updatedMessages, aiMessage],
            lastMessage: aiMessage.contentEn,
            timestamp: new Date(),
          }
        }
        return c
      }))

      // Auto-play AI response
      if (response.ai_audio_base64) {
        playAudioFromBase64(response.ai_audio_base64, aiMessage.id)
      }
    } catch (err) {
      console.error('Chat error:', err)
      // Add error message
      const errorMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        contentEn: 'Sorry, I encountered an error. Please try again.',
        contentZh: '抱歉，发生了错误。请重试。',
        timestamp: new Date(),
      }
      setConversations(prev => prev.map(c => {
        if (c.id === activeConversationId) {
          return {
            ...c,
            messages: [...c.messages, errorMessage],
            lastMessage: errorMessage.contentEn,
            timestamp: new Date(),
          }
        }
        return c
      }))
    } finally {
      setIsProcessing(false)
    }
  }

  // Convert Blob to base64
  const blobToBase64 = (blob: Blob): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onloadend = () => {
        const base64 = reader.result as string
        // Remove data URL prefix (e.g., "data:audio/webm;base64,")
        const base64Data = base64.split(',')[1]
        resolve(base64Data)
      }
      reader.onerror = reject
      reader.readAsDataURL(blob)
    })
  }

  // Convert audio blob to WAV format using Web Audio API
  const convertToWav = async (audioBlob: Blob): Promise<Blob> => {
    const audioContext = new AudioContext()
    const arrayBuffer = await audioBlob.arrayBuffer()

    try {
      const audioBuffer = await audioContext.decodeAudioData(arrayBuffer)

      // Create WAV file
      const numberOfChannels = audioBuffer.numberOfChannels
      const sampleRate = audioBuffer.sampleRate
      const length = audioBuffer.length

      // Create buffer for WAV file
      const wavBuffer = new ArrayBuffer(44 + length * numberOfChannels * 2)
      const view = new DataView(wavBuffer)

      // Write WAV header
      const writeString = (offset: number, str: string) => {
        for (let i = 0; i < str.length; i++) {
          view.setUint8(offset + i, str.charCodeAt(i))
        }
      }

      writeString(0, 'RIFF')
      view.setUint32(4, 36 + length * numberOfChannels * 2, true)
      writeString(8, 'WAVE')
      writeString(12, 'fmt ')
      view.setUint32(16, 16, true) // Subchunk1Size
      view.setUint16(20, 1, true) // AudioFormat (PCM)
      view.setUint16(22, numberOfChannels, true)
      view.setUint32(24, sampleRate, true)
      view.setUint32(28, sampleRate * numberOfChannels * 2, true) // ByteRate
      view.setUint16(32, numberOfChannels * 2, true) // BlockAlign
      view.setUint16(34, 16, true) // BitsPerSample
      writeString(36, 'data')
      view.setUint32(40, length * numberOfChannels * 2, true)

      // Write audio data
      const offset = 44
      for (let i = 0; i < length; i++) {
        for (let channel = 0; channel < numberOfChannels; channel++) {
          const sample = audioBuffer.getChannelData(channel)[i]
          // Convert float to 16-bit PCM
          const intSample = Math.max(-1, Math.min(1, sample))
          view.setInt16(offset + (i * numberOfChannels + channel) * 2, intSample < 0 ? intSample * 0x8000 : intSample * 0x7FFF, true)
        }
      }

      return new Blob([wavBuffer], { type: 'audio/wav' })
    } finally {
      await audioContext.close()
    }
  }

  // Start recording
  const startRecording = async () => {
    if (!token || !activeConversation) return

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/mp4'
      })

      mediaRecorderRef.current = mediaRecorder
      audioChunksRef.current = []

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data)
        }
      }

      mediaRecorder.onstop = async () => {
        // Stop all tracks
        stream.getTracks().forEach(track => track.stop())

        // Clear recording timer
        if (recordingTimerRef.current) {
          clearInterval(recordingTimerRef.current)
          recordingTimerRef.current = null
        }
        setRecordingDuration(0)

        // Process audio
        const audioBlob = new Blob(audioChunksRef.current, { type: mediaRecorder.mimeType })
        if (audioBlob.size === 0) {
          console.error('No audio recorded')
          return
        }

        setIsProcessing(true)

        try {
          // Convert to WAV format for BigModel ASR API
          const wavBlob = await convertToWav(audioBlob)
          const audioBase64 = await blobToBase64(wavBlob)
          const history = getConversationHistory()
          const response = await voiceChatSend(token, audioBase64, history)

          // Add user message with transcribed text
          const userMessage: Message = {
            id: Date.now().toString(),
            role: 'user',
            contentEn: response.user_text || '(Audio message)',
            contentZh: response.user_text || '(语音消息)',
            hasAudio: true,
            timestamp: new Date(),
            corrections: response.corrections,
          }

          // Add AI response
          const aiMessage: Message = {
            id: (Date.now() + 1).toString(),
            role: 'assistant',
            contentEn: response.ai_text,
            contentZh: response.ai_text_zh || response.ai_text,
            hasAudio: !!response.ai_audio_base64,
            audioBase64: response.ai_audio_base64 || undefined,
            timestamp: new Date(),
          }

          setConversations(prev => prev.map(c => {
            if (c.id === activeConversationId) {
              return {
                ...c,
                messages: [...c.messages, userMessage, aiMessage],
                lastMessage: aiMessage.contentEn,
                timestamp: new Date(),
              }
            }
            return c
          }))

          // Auto-play AI response
          if (response.ai_audio_base64) {
            playAudioFromBase64(response.ai_audio_base64, aiMessage.id)
          }
        } catch (err) {
          console.error('Voice chat error:', err)
          const errorMessage: Message = {
            id: (Date.now() + 1).toString(),
            role: 'assistant',
            contentEn: 'Sorry, I could not process your voice message. Please try again.',
            contentZh: '抱歉，无法处理您的语音消息。请重试。',
            timestamp: new Date(),
          }
          setConversations(prev => prev.map(c => {
            if (c.id === activeConversationId) {
              return {
                ...c,
                messages: [...c.messages, errorMessage],
                lastMessage: errorMessage.contentEn,
                timestamp: new Date(),
              }
            }
            return c
          }))
        } finally {
          setIsProcessing(false)
        }
      }

      mediaRecorder.start()
      setIsRecording(true)
      setRecordingDuration(0)

      // Start recording timer
      recordingTimerRef.current = setInterval(() => {
        setRecordingDuration(prev => prev + 1)
      }, 1000)
    } catch (err) {
      console.error('Failed to start recording:', err)
      alert('无法访问麦克风。请确保已授予麦克风权限。')
    }
  }

  // Stop recording
  const stopRecording = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop()
    }
    setIsRecording(false)
  }

  const toggleRecording = () => {
    if (!isRecording) {
      startRecording()
    } else {
      stopRecording()
    }
  }

  const handleNewConversation = () => {
    const newConversation: Conversation = {
      id: Date.now().toString(),
      title: '新对话',
      lastMessage: '',
      timestamp: new Date(),
      messages: [
        {
          id: Date.now().toString() + '-1',
          role: 'assistant',
          contentEn: "Hi there! I'm your AI English conversation partner. What would you like to talk about today?",
          contentZh: "你好！我是你的AI英语对话伙伴。今天你想聊些什么？",
          timestamp: new Date(),
        },
      ],
    }

    setConversations(prev => [newConversation, ...prev])
    setActiveConversationId(newConversation.id)
  }

  const playAudio = (messageId: string) => {
    // If already playing this message, stop it
    if (isPlayingAudio === messageId) {
      stopAudio()
      return
    }

    // Find the message and play its audio
    const message = activeConversation?.messages.find(m => m.id === messageId)
    if (message?.audioBase64) {
      playAudioFromBase64(message.audioBase64, messageId)
    } else if (message?.hasAudio && token) {
      // If message has audio but no base64, try to get TTS for the text
      textToSpeech(token, message.contentEn)
        .then(response => {
          playAudioFromBase64(response.audio_base64, messageId)
        })
        .catch(err => {
          console.error('Failed to get TTS:', err)
        })
    }
  }

  const exportToPdf = () => {
    // Simple PDF export using print dialog
    const printWindow = window.open('', '_blank')
    if (!printWindow || !activeConversation) return

    const messagesHtml = activeConversation.messages.map(msg => {
      const isUser = msg.role === 'user'
      let correctionsHtml = ''
      if (reportMode && isUser && msg.corrections && msg.corrections.length > 0) {
        correctionsHtml = `
          <div style="margin-top: 8px; padding: 8px 12px; background: #fef3c7; border-radius: 8px; font-size: 13px;">
            <div style="font-weight: 600; color: #92400e; margin-bottom: 4px;">改进建议:</div>
            ${msg.corrections.map(c => `
              <div style="margin-bottom: 4px;">
                <span style="color: #dc2626; text-decoration: line-through;">${c.original}</span>
                → <span style="color: #16a34a; font-weight: 500;">${c.corrected}</span>
                <div style="color: #78716c; font-size: 12px; margin-top: 2px;">${c.explanation}</div>
              </div>
            `).join('')}
          </div>
        `
      }
      return `
        <div style="margin-bottom: 16px; text-align: ${isUser ? 'right' : 'left'};">
          <div style="display: inline-block; max-width: 70%; padding: 12px 16px; border-radius: 16px; background: ${isUser ? '#f97316' : '#f3f4f6'}; color: ${isUser ? 'white' : '#111827'};">
            <p style="margin: 0;">${msg.contentEn}</p>
            <p style="margin: 8px 0 0 0; opacity: 0.8; font-size: 14px;">${msg.contentZh}</p>
          </div>
          ${correctionsHtml}
        </div>
      `
    }).join('')

    printWindow.document.write(`
      <!DOCTYPE html>
      <html>
      <head>
        <title>${activeConversation.title} - 对话记录</title>
        <style>
          body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 40px; max-width: 800px; margin: 0 auto; }
          h1 { color: #111827; border-bottom: 2px solid #f97316; padding-bottom: 8px; }
          .meta { color: #6b7280; font-size: 14px; margin-bottom: 24px; }
        </style>
      </head>
      <body>
        <h1>${activeConversation.title}</h1>
        <div class="meta">导出时间: ${new Date().toLocaleString()}${reportMode ? ' | 报告模式' : ''}</div>
        ${messagesHtml}
      </body>
      </html>
    `)
    printWindow.document.close()
    printWindow.print()
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-6xl p-4">
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
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50 flex flex-col">
      <Header />

      <main className="flex-1 mx-auto w-full max-w-6xl p-4">
        <div className="bg-white rounded-xl shadow-lg overflow-hidden h-[calc(100vh-180px)] flex">
          {/* Left Sidebar - Conversation History */}
          <div className="w-72 border-r flex flex-col bg-gray-50">
            <div className="p-4 border-b bg-white">
              <Button onClick={handleNewConversation} className="w-full gap-2">
                <Plus className="h-4 w-4" />
                新对话
              </Button>
            </div>
            <div className="flex-1 overflow-y-auto">
              {conversations.map((conv) => (
                <div
                  key={conv.id}
                  onClick={() => setActiveConversationId(conv.id)}
                  className={cn(
                    'p-4 border-b cursor-pointer hover:bg-white transition-colors',
                    activeConversationId === conv.id && 'bg-white border-l-4 border-l-orange-500'
                  )}
                >
                  <div className="flex items-start gap-3">
                    <div className="w-10 h-10 rounded-full bg-orange-100 flex items-center justify-center flex-shrink-0">
                      <MessageSquare className="h-5 w-5 text-orange-600" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-medium text-gray-900 truncate">{conv.title}</h3>
                      <p className="text-sm text-gray-500 truncate">{conv.lastMessage || '开始对话...'}</p>
                      <p className="text-xs text-gray-400 mt-1">
                        {conv.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                      </p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Right Panel - Chat Window */}
          <div className="flex-1 flex flex-col">
            {/* Chat Header with Display Settings */}
            <div className="border-b px-6 py-3 bg-white">
              <div className="flex items-center justify-between">
                <h2 className="font-semibold text-gray-900">
                  {activeConversation?.title || '日常唠嗑'}
                </h2>
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-3 text-sm">
                    {/* Audio Settings Button with Popover */}
                    <div className="relative" ref={audioSettingsRef}>
                      <button
                        onClick={() => setShowAudioSettings(!showAudioSettings)}
                        className={cn(
                          'p-1.5 rounded-md transition-all',
                          showAudioSettings
                            ? 'bg-orange-100 text-orange-600'
                            : 'text-gray-400 hover:bg-gray-100 hover:text-gray-600'
                        )}
                        title="音频设置"
                      >
                        <Settings2 className="h-4 w-4" />
                      </button>

                      {/* Audio Settings Popover */}
                      {showAudioSettings && (
                        <div className="absolute top-full right-0 mt-2 w-64 bg-white rounded-xl shadow-lg border p-4 z-10">
                          <div className="text-sm font-medium text-gray-900 mb-3">音频设置</div>

                          {/* Microphone Selection */}
                          <div className="mb-3">
                            <label className="text-xs text-gray-500 mb-1 block">麦克风</label>
                            <div className="relative">
                              <select
                                value={selectedMic}
                                onChange={(e) => setSelectedMic(e.target.value)}
                                className="w-full px-3 py-2 pr-8 text-sm border rounded-lg bg-gray-50 focus:outline-none focus:ring-2 focus:ring-orange-500 appearance-none"
                              >
                                <option value="default">默认麦克风</option>
                                <option value="mic1">内置麦克风</option>
                                <option value="mic2">外接麦克风</option>
                              </select>
                              <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400 pointer-events-none" />
                            </div>
                          </div>

                          {/* Speaker Selection */}
                          <div>
                            <label className="text-xs text-gray-500 mb-1 block">扬声器</label>
                            <div className="relative">
                              <select
                                value={selectedSpeaker}
                                onChange={(e) => setSelectedSpeaker(e.target.value)}
                                className="w-full px-3 py-2 pr-8 text-sm border rounded-lg bg-gray-50 focus:outline-none focus:ring-2 focus:ring-orange-500 appearance-none"
                              >
                                <option value="default">默认扬声器</option>
                                <option value="speaker1">内置扬声器</option>
                                <option value="speaker2">外接音响</option>
                              </select>
                              <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400 pointer-events-none" />
                            </div>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Report Mode Toggle Button */}
                    <button
                      onClick={() => setReportMode(!reportMode)}
                      className={cn(
                        'p-1.5 rounded-md transition-all',
                        reportMode
                          ? 'bg-amber-100 text-amber-600'
                          : 'text-gray-400 hover:bg-gray-100 hover:text-gray-600'
                      )}
                      title={reportMode ? '关闭报告模式' : '开启报告模式'}
                    >
                      <ClipboardList className="h-4 w-4" />
                    </button>

                    {/* Export PDF Button */}
                    <button
                      onClick={exportToPdf}
                      className="p-1.5 rounded-md text-gray-400 hover:bg-gray-100 hover:text-gray-600 transition-all"
                      title="导出 PDF"
                    >
                      <FileDown className="h-4 w-4" />
                    </button>
                    <div className="flex items-center gap-2 border-l pl-3">
                      <span className="text-gray-500 text-xs">AI:</span>
                      <div className="flex rounded-lg border border-gray-200 overflow-hidden">
                        {(['both', 'en', 'zh'] as const).map((lang) => (
                          <button
                            key={lang}
                            onClick={() => setBotLang(lang)}
                            className={cn(
                              'px-2.5 py-1 text-xs font-medium transition-colors',
                              botLang === lang
                                ? 'bg-orange-500 text-white'
                                : 'bg-white text-gray-600 hover:bg-orange-50'
                            )}
                          >
                            {lang === 'both' ? '双语' : lang === 'en' ? '英' : '中'}
                          </button>
                        ))}
                      </div>
                    </div>
                    <div className="flex items-center gap-2 border-l pl-3">
                      <span className="text-gray-500 text-xs">我:</span>
                      <div className="flex rounded-lg border border-gray-200 overflow-hidden">
                        {(['both', 'en', 'zh'] as const).map((lang) => (
                          <button
                            key={lang}
                            onClick={() => setUserLang(lang)}
                            className={cn(
                              'px-2.5 py-1 text-xs font-medium transition-colors',
                              userLang === lang
                                ? 'bg-orange-500 text-white'
                                : 'bg-white text-gray-600 hover:bg-orange-50'
                            )}
                          >
                            {lang === 'both' ? '双语' : lang === 'en' ? '英' : '中'}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Messages */}
            <div className="flex-1 overflow-y-auto p-6 space-y-4">
              {messages.map((message) => {
                const isUser = message.role === 'user'
                const showEn = isUser ? showUserEn : showBotEn
                const showZh = isUser ? showUserZh : showBotZh

                if (!showEn && !showZh) return null

                return (
                  <div
                    key={message.id}
                    className={cn(
                      'flex flex-col',
                      isUser ? 'items-end' : 'items-start'
                    )}
                  >
                    <div
                      className={cn(
                        'max-w-[70%] rounded-2xl px-4 py-3',
                        isUser
                          ? 'bg-orange-500 text-white'
                          : 'bg-gray-100 text-gray-900'
                      )}
                    >
                      {showEn && (
                        <p className="text-sm">{message.contentEn}</p>
                      )}
                      {showEn && showZh && (
                        <div className={cn(
                          'my-2 border-t',
                          isUser ? 'border-orange-400/30' : 'border-gray-200'
                        )} />
                      )}
                      {showZh && (
                        <p className={cn(
                          'text-sm',
                          isUser ? 'text-orange-100' : 'text-gray-600'
                        )}>
                          {message.contentZh}
                        </p>
                      )}
                      <div className={cn(
                        'flex items-center justify-between mt-2 text-xs',
                        isUser ? 'text-orange-200' : 'text-gray-400'
                      )}>
                        <span>
                          {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </span>
                        {(message.hasAudio || message.audioBase64) && (
                          <button
                            onClick={() => playAudio(message.id)}
                            className={cn(
                              'p-1 rounded hover:bg-black/10 transition-colors',
                              isUser ? 'hover:bg-white/20' : 'hover:bg-gray-200',
                              isPlayingAudio === message.id && 'bg-black/10'
                            )}
                            title={isPlayingAudio === message.id ? '停止播放' : '播放语音'}
                          >
                            {isPlayingAudio === message.id ? (
                              <Square className="h-4 w-4" />
                            ) : (
                              <Volume2 className="h-4 w-4" />
                            )}
                          </button>
                        )}
                      </div>
                    </div>
                    {/* Corrections display in report mode */}
                    {reportMode && isUser && message.corrections && message.corrections.length > 0 && (
                      <div className="max-w-[70%] mt-2 px-3 py-2 bg-amber-50 border border-amber-200 rounded-xl">
                        <div className="text-xs font-medium text-amber-700 mb-1.5 flex items-center gap-1">
                          <ClipboardList className="h-3 w-3" />
                          改进建议
                        </div>
                        {message.corrections.map((correction, idx) => (
                          <div key={idx} className="text-sm mb-1.5 last:mb-0">
                            <span className="text-red-500 line-through">{correction.original}</span>
                            <span className="text-gray-400 mx-1">→</span>
                            <span className="text-green-600 font-medium">{correction.corrected}</span>
                            <p className="text-xs text-gray-500 mt-0.5">{correction.explanation}</p>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )
              })}
              <div ref={messagesEndRef} />
            </div>

            {/* Input Area */}
            <div className="border-t p-4 bg-white">
              {/* Row 1: Mic button */}
              <div className="flex items-center justify-center mb-4">
                {/* Large Mic Button */}
                <button
                  onClick={toggleRecording}
                  disabled={isProcessing}
                  className={cn(
                    'h-16 w-16 rounded-full flex items-center justify-center transition-all flex-shrink-0',
                    isProcessing
                      ? 'bg-gray-400 text-white cursor-not-allowed'
                      : isRecording
                        ? 'bg-red-500 text-white animate-pulse shadow-lg shadow-red-200'
                        : 'bg-orange-500 text-white hover:bg-orange-600 shadow-lg shadow-orange-200'
                  )}
                  title={isProcessing ? '处理中...' : isRecording ? '停止录音' : '开始录音'}
                >
                  {isProcessing ? (
                    <Loader2 className="h-7 w-7 animate-spin" />
                  ) : isRecording ? (
                    <MicOff className="h-7 w-7" />
                  ) : (
                    <Mic className="h-7 w-7" />
                  )}
                </button>
              </div>

              {/* Recording/Processing indicator */}
              {(isRecording || isProcessing) && (
                <div className={cn(
                  'mb-3 flex items-center justify-center gap-2',
                  isProcessing ? 'text-gray-500' : 'text-red-500'
                )}>
                  {isProcessing ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      <span className="text-sm">正在处理语音...</span>
                    </>
                  ) : (
                    <>
                      <span className="w-2 h-2 bg-red-500 rounded-full animate-pulse" />
                      <span className="text-sm">
                        正在录音 {Math.floor(recordingDuration / 60)}:{(recordingDuration % 60).toString().padStart(2, '0')} - 点击话筒停止
                      </span>
                    </>
                  )}
                </div>
              )}

              {/* Row 2: Text input and send button */}
              <div className="flex items-center gap-2">
                <textarea
                  ref={textareaRef}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="输入文字消息... (Shift+Enter 换行)"
                  rows={1}
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent resize-none overflow-y-auto text-sm"
                  style={{ height: '36px', maxHeight: '120px' }}
                />
                <Button
                  onClick={handleSend}
                  disabled={!input.trim() || isProcessing}
                  className="h-9 px-4"
                >
                  {isProcessing ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Send className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  )
}
