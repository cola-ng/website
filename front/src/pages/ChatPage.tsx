import { useState, useRef, useEffect, useCallback } from 'react'
import { Send, Mic, MicOff, MessageCircle, Map, Volume2, FileDown, ClipboardList, Loader2, X, MoreVertical, Pin, Pencil, Trash2 } from 'lucide-react'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'
import { voiceChatSend, textChatSend, textToSpeech, updateChatTitle, createChat, listChats, pollChatTurn, getChatTurns, getChatIssues, type TextIssue, type ChatTurn } from '../lib/api'

interface Message {
  id: string
  role: 'user' | 'assistant'
  contentEn: string
  contentZh: string
  hasAudio?: boolean
  audioPath?: string  // Server path for audio file (e.g., "learn/audios/123/ai_xxx.wav")
  audioBase64?: string
  timestamp: Date
  issues?: TextIssue[]
}

interface Chat {
  id: string
  serverId?: number   // Server-side chat ID (if synced)
  title: string
  contextId?: number  // Optional context ID for context-based chats
  icon?: string       // Emoji icon for context-based chats
  lastMessage: string
  timestamp: Date
  messages: Message[]
  // Pagination state
  hasMorePrev?: boolean  // Has older messages to load
  hasMoreNext?: boolean  // Has newer messages to load
  firstId?: number       // First message ID (for loading older)
  lastId?: number        // Last message ID (for loading newer)
}

// Context matches backend asset_contexts table
interface Context {
  id: number
  name_en: string
  name_zh: string
  description_en: string | null
  description_zh: string | null
  icon_emoji: string | null
  display_order: number | null
  difficulty: number | null
  is_active: boolean | null
  created_at: string
}

export function ChatPage() {
  const { token } = useAuth()
  const [chats, setChats] = useState<Chat[]>([])
  const [activeChatId, setActiveChatId] = useState<string | null>(null)
  const [chatsLoading, setChatsLoading] = useState(true)
  const [input, setInput] = useState('')
  const [isRecording, setIsRecording] = useState(false)
  const [reportMode, setReportMode] = useState(false)
  const [isVoiceProcessing, setIsVoiceProcessing] = useState(false)
  const [isTextProcessing, setIsTextProcessing] = useState(false)
  const [isPlayingAudio, setIsPlayingAudio] = useState<string | null>(null)
  const [recordingDuration, setRecordingDuration] = useState(0)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const mediaRecorderRef = useRef<MediaRecorder | null>(null)
  const audioChunksRef = useRef<Blob[]>([])
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const audioElementRef = useRef<HTMLAudioElement | null>(null)

  // Context selection dialog state
  const [showContextDialog, setShowContextDialog] = useState(false)
  const [contexts, setContexts] = useState<Context[]>([])
  const [contextsLoading, setContextsLoading] = useState(false)

  // Chat menu state
  const [menuOpenId, setMenuOpenId] = useState<string | null>(null)
  const [renameDialogId, setRenameDialogId] = useState<string | null>(null)
  const [renameValue, setRenameValue] = useState('')

  // Display settings - independent toggles for each language (persisted to localStorage)
  const [showBotEn, setShowBotEn] = useState(() => {
    const saved = localStorage.getItem('conv_showBotEn')
    return saved !== null ? saved === 'true' : true
  })
  const [showBotZh, setShowBotZh] = useState(() => {
    const saved = localStorage.getItem('conv_showBotZh')
    return saved !== null ? saved === 'true' : true
  })
  const [showUserEn, setShowUserEn] = useState(() => {
    const saved = localStorage.getItem('conv_showUserEn')
    return saved !== null ? saved === 'true' : true
  })
  const [showUserZh, setShowUserZh] = useState(() => {
    const saved = localStorage.getItem('conv_showUserZh')
    return saved !== null ? saved === 'true' : true
  })

  // Track which language was last deselected (for blur behavior when both are off)
  // Default to 'en' so English text is shown blurred when both are off
  const [lastDeselectedBot, setLastDeselectedBot] = useState<'en' | 'zh'>('en')
  const [lastDeselectedUser, setLastDeselectedUser] = useState<'en' | 'zh'>('en')

  // Toggle handlers that track the last deselected language
  const toggleBotEn = () => {
    if (showBotEn) setLastDeselectedBot('en')
    setShowBotEn(!showBotEn)
  }
  const toggleBotZh = () => {
    if (showBotZh) setLastDeselectedBot('zh')
    setShowBotZh(!showBotZh)
  }
  const toggleUserEn = () => {
    if (showUserEn) setLastDeselectedUser('en')
    setShowUserEn(!showUserEn)
  }
  const toggleUserZh = () => {
    if (showUserZh) setLastDeselectedUser('zh')
    setShowUserZh(!showUserZh)
  }

  // Persist display settings to localStorage
  useEffect(() => {
    localStorage.setItem('conv_showBotEn', String(showBotEn))
    localStorage.setItem('conv_showBotZh', String(showBotZh))
    localStorage.setItem('conv_showUserEn', String(showUserEn))
    localStorage.setItem('conv_showUserZh', String(showUserZh))
  }, [showBotEn, showBotZh, showUserEn, showUserZh])

  // Load chats from server on mount
  useEffect(() => {
    if (!token) {
      setChatsLoading(false)
      return
    }

    setChatsLoading(true)
    listChats(token)
      .then(async (serverChats) => {
        const loadedChats: Chat[] = serverChats.map((chat) => ({
          id: chat.id.toString(),
          serverId: chat.id,
          title: chat.title,
          contextId: chat.context_id ?? undefined,
          lastMessage: '',
          timestamp: new Date(chat.created_at),
          messages: [],
        }))
        setChats(loadedChats)

        // Auto-load turns for the first chat
        if (loadedChats.length > 0) {
          const firstChat = loadedChats[0]
          setActiveChatId(firstChat.id)

          // Fetch turns and issues for the first chat
          if (firstChat.serverId) {
            try {
              const [turnsResponse, issues] = await Promise.all([
                getChatTurns(token, firstChat.serverId, { limit: 50 }),
                getChatIssues(token, firstChat.serverId),
              ])

              // Create a map of turn_id -> issues for quick lookup
              const issuesByTurnId: Record<number, TextIssue[]> = {}
              for (const ann of issues) {
                const turnId = ann.chat_turn_id
                if (!issuesByTurnId[turnId]) {
                  issuesByTurnId[turnId] = []
                }
                issuesByTurnId[turnId].push({
                  type: ann.issue_type,
                  original: ann.original_text || '',
                  suggested: ann.suggested_text || '',
                  description_en: ann.description_en || '',
                  description_zh: ann.description_zh || '',
                  severity: ann.severity || 'low',
                  start_position: ann.start_position,
                  end_position: ann.end_position,
                })
              }

              const messages: Message[] = turnsResponse.items
                .filter((turn: ChatTurn) => turn.status === 'completed' && (turn.content_en || turn.content_zh))
                .map((turn: ChatTurn) => ({
                  id: turn.id.toString(),
                  role: turn.speaker === 'user' ? 'user' : 'assistant' as const,
                  contentEn: turn.content_en || '',
                  contentZh: turn.content_zh || '',
                  hasAudio: !!turn.audio_path,
                  audioPath: turn.audio_path || undefined,
                  timestamp: new Date(turn.created_at),
                  issues: issuesByTurnId[turn.id],
                }))

              setChats(prev => prev.map(c => {
                if (c.id === firstChat.id) {
                  return {
                    ...c,
                    messages,
                    lastMessage: messages.length > 0 ? messages[messages.length - 1].contentEn : '',
                    hasMorePrev: turnsResponse.has_prev,
                    hasMoreNext: turnsResponse.has_next,
                    firstId: turnsResponse.first_id ?? undefined,
                    lastId: turnsResponse.last_id ?? undefined,
                  }
                }
                return c
              }))
            } catch (err) {
              console.error('Failed to fetch initial chat turns:', err)
            }
          }
        }
      })
      .catch((err) => {
        console.error('Failed to load chats:', err)
      })
      .finally(() => {
        setChatsLoading(false)
      })
  }, [token])

  // Fetch contexts when dialog is opened
  useEffect(() => {
    if (showContextDialog && contexts.length === 0) {
      setContextsLoading(true)
      fetch('/api/asset/contexts')
        .then(res => res.ok ? res.json() : Promise.reject('Failed to fetch contexts'))
        .then((data: Context[]) => setContexts(data))
        .catch(err => console.error('Failed to fetch contexts:', err))
        .finally(() => setContextsLoading(false))
    }
  }, [showContextDialog, contexts.length])

  const activeChat = chats.find(c => c.id === activeChatId)
  const messages = activeChat?.messages || []
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
    if (!input.trim() || !activeChat || !token || isTextProcessing) return

    const messageText = input.trim()
    setInput('')
    setIsTextProcessing(true)

    // Add user message immediately
    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      contentEn: messageText,
      contentZh: messageText,
      timestamp: new Date(),
    }

    setChats(prev => prev.map(c => {
      if (c.id === activeChatId) {
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
      // Call API for text chat (history is managed server-side)
      const chatId = activeChat.serverId
      if (!chatId) {
        throw new Error('Chat not synced with server')
      }
      const sendResponse = await textChatSend(token, chatId, messageText, true)

      // Update user message with content from server
      setChats(prev => prev.map(c => {
        if (c.id === activeChatId) {
          return {
            ...c,
            messages: c.messages.map(m =>
              m.id === userMessage.id
                ? {
                  ...m,
                  contentEn: sendResponse.user_turn.content_en || messageText,
                  contentZh: sendResponse.user_turn.content_zh || messageText,
                }
                : m
            ),
          }
        }
        return c
      }))

      // Poll for AI turn completion
      const completedAiTurn = await pollChatTurn(token, chatId, sendResponse.ai_turn.id, 1000, 60)

      // Use issues directly from send response
      const userIssues = sendResponse.issues

      // Add AI response and update user message with issues
      const aiMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        contentEn: completedAiTurn.content_en,
        contentZh: completedAiTurn.content_zh || completedAiTurn.content_en,
        hasAudio: !!completedAiTurn.audio_path,
        audioPath: completedAiTurn.audio_path || undefined,
        timestamp: new Date(),
      }

      setChats(prev => prev.map(c => {
        if (c.id === activeChatId) {
          return {
            ...c,
            messages: c.messages
              .map(m => m.id === userMessage.id ? { ...m, issues: userIssues.length > 0 ? userIssues : undefined } : m)
              .concat([aiMessage]),
            lastMessage: aiMessage.contentEn,
            timestamp: new Date(),
          }
        }
        return c
      }))

      // Auto-play AI response audio if available
      if (aiMessage.audioPath) {
        // Small delay to ensure state is updated
        setTimeout(() => playAudio(aiMessage.id), 100)
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
      setChats(prev => prev.map(c => {
        if (c.id === activeChatId) {
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
      setIsTextProcessing(false)
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
    if (!token || !activeChat) return

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

        setIsVoiceProcessing(true)

        try {
          // Convert to WAV format for BigModel ASR API
          const wavBlob = await convertToWav(audioBlob)
          const audioBase64 = await blobToBase64(wavBlob)

          // Get chat ID from active chat
          const chatId = activeChat?.serverId
          if (!chatId || !token) {
            throw new Error('Chat not synced with server')
          }

          // Send voice chat
          const sendResponse = await voiceChatSend(token, chatId, audioBase64)

          // Add user message with transcribed text
          const userMessage: Message = {
            id: Date.now().toString(),
            role: 'user',
            contentEn: sendResponse.user_turn.content_en || '(Audio message)',
            contentZh: sendResponse.user_turn.content_zh || '(语音消息)',
            hasAudio: !!sendResponse.user_turn.audio_path,
            audioPath: sendResponse.user_turn.audio_path || undefined,
            timestamp: new Date(),
          }

          setChats(prev => prev.map(c => {
            if (c.id === activeChatId) {
              return {
                ...c,
                messages: [...c.messages, userMessage],
                lastMessage: userMessage.contentEn,
                timestamp: new Date(),
              }
            }
            return c
          }))

          // Poll for AI turn completion
          const completedAiTurn = await pollChatTurn(token, chatId, sendResponse.ai_turn.id, 1000, 60)

          // Use issues directly from send response
          const userIssues = sendResponse.issues

          // Add AI response and update user message with issues
          const aiMessage: Message = {
            id: (Date.now() + 1).toString(),
            role: 'assistant',
            contentEn: completedAiTurn.content_en,
            contentZh: completedAiTurn.content_zh || completedAiTurn.content_en,
            hasAudio: !!completedAiTurn.audio_path,
            audioPath: completedAiTurn.audio_path || undefined,
            timestamp: new Date(),
          }

          setChats(prev => prev.map(c => {
            if (c.id === activeChatId) {
              return {
                ...c,
                messages: c.messages
                  .map(m => m.id === userMessage.id ? { ...m, issues: userIssues.length > 0 ? userIssues : undefined } : m)
                  .concat([aiMessage]),
                lastMessage: aiMessage.contentEn,
                timestamp: new Date(),
              }
            }
            return c
          }))

          // Auto-play AI response audio if available
          if (aiMessage.audioPath) {
            playAudio(aiMessage.id)
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
          setChats(prev => prev.map(c => {
            if (c.id === activeChatId) {
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
          setIsVoiceProcessing(false)
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

  // Stop recording (will process audio)
  const stopRecording = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop()
    }
    setIsRecording(false)
  }

  // Cancel recording (discard audio without processing)
  const cancelRecording = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      // Remove the onstop handler to prevent processing
      mediaRecorderRef.current.onstop = () => {
        // Just stop the tracks without processing
        if (mediaRecorderRef.current?.stream) {
          mediaRecorderRef.current.stream.getTracks().forEach(track => track.stop())
        }
      }
      mediaRecorderRef.current.stop()
    }
    // Clear recording timer
    if (recordingTimerRef.current) {
      clearInterval(recordingTimerRef.current)
      recordingTimerRef.current = null
    }
    setRecordingDuration(0)
    setIsRecording(false)
    audioChunksRef.current = []
  }

  const toggleRecording = () => {
    if (!isRecording) {
      startRecording()
    } else {
      stopRecording()
    }
  }

  // Keyboard shortcuts for voice recording
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if processing or if user is typing in an input
      if (isVoiceProcessing || isTextProcessing) return

      const activeElement = document.activeElement
      const isInputFocused = activeElement instanceof HTMLTextAreaElement ||
        activeElement instanceof HTMLInputElement ||
        activeElement?.getAttribute('contenteditable') === 'true'

      // Space key toggles recording when input is not focused
      if (e.code === 'Space' && !isInputFocused) {
        e.preventDefault()
        toggleRecording()
      }

      // ESC key cancels recording
      if (e.code === 'Escape' && isRecording) {
        e.preventDefault()
        cancelRecording()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isRecording, isVoiceProcessing, isTextProcessing])

  // Handle free chat (随便聊)
  const handleNewFreeChat = async () => {
    let serverId: number | undefined

    if (token) {
      try {
        // Create new chat on server
        const chat = await createChat(token, '随便聊')
        serverId = chat.id
      } catch (err) {
        console.error('Failed to create chat:', err)
      }
    }

    const newChat: Chat = {
      id: Date.now().toString(),
      serverId,
      title: '随便聊',
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

    setChats(prev => [newChat, ...prev])
    setActiveChatId(newChat.id)
  }

  // Handle context-based chat (选场景)
  const handleNewContextChat = async (context: Context) => {
    setShowContextDialog(false)

    let serverId: number | undefined

    if (token) {
      try {
        // Create new chat on server with context_id
        const chat = await createChat(token, context.name_zh, context.id)
        serverId = chat.id
      } catch (err) {
        console.error('Failed to create chat:', err)
      }
    }

    const newChat: Chat = {
      id: Date.now().toString(),
      serverId,
      title: context.name_zh,
      contextId: context.id,
      icon: context.icon_emoji || undefined,
      lastMessage: '',
      timestamp: new Date(),
      messages: [
        {
          id: Date.now().toString() + '-1',
          role: 'assistant',
          contentEn: `Hi! Let's practice a conversation about "${context.name_en}". I'll play a role in this scenario. Ready to start?`,
          contentZh: `你好！让我们来练习关于"${context.name_zh}"的对话。我会在这个场景中扮演一个角色。准备好开始了吗？`,
          timestamp: new Date(),
        },
      ],
    }

    setChats(prev => [newChat, ...prev])
    setActiveChatId(newChat.id)
  }

  // Pin chat to top
  const handlePinChat = (convId: string) => {
    setMenuOpenId(null)
    setChats(prev => {
      const conv = prev.find(c => c.id === convId)
      if (!conv) return prev
      const others = prev.filter(c => c.id !== convId)
      return [conv, ...others]
    })
  }

  // Open rename dialog
  const handleOpenRename = (convId: string) => {
    const conv = chats.find(c => c.id === convId)
    if (conv) {
      setRenameValue(conv.title)
      setRenameDialogId(convId)
    }
    setMenuOpenId(null)
  }

  // Confirm rename
  const handleConfirmRename = async () => {
    if (renameDialogId && renameValue.trim()) {
      const newTitle = renameValue.trim()
      const conv = chats.find(c => c.id === renameDialogId)

      // Update local state
      setChats(prev => prev.map(c =>
        c.id === renameDialogId ? { ...c, title: newTitle } : c
      ))

      // Sync to server if chat has a server ID
      if (token && conv?.serverId) {
        try {
          console.log('Updating chat title on server:', conv.serverId, newTitle)
          await updateChatTitle(token, conv.serverId, newTitle)
        } catch (err) {
          console.error('Failed to update chat title on server:', err)
        }
      } else {
        console.log('No serverId for chat, skipping server sync:', conv?.id)
      }
    }
    setRenameDialogId(null)
    setRenameValue('')
  }

  // Delete chat
  const handleDeleteChat = (convId: string) => {
    setMenuOpenId(null)
    setChats(prev => {
      const filtered = prev.filter(c => c.id !== convId)
      // If we deleted the active chat, switch to the first one
      if (activeChatId === convId && filtered.length > 0) {
        setActiveChatId(filtered[0].id)
      }
      return filtered
    })
  }

  // Handle chat selection - fetch turns from server
  const handleSelectChat = useCallback(async (chat: Chat) => {
    setActiveChatId(chat.id)

    // If chat has no server ID or already has messages loaded, skip fetching
    if (!chat.serverId || !token) {
      return
    }

    // Check if messages are already loaded for this chat
    const existingChat = chats.find(c => c.id === chat.id)
    if (existingChat && existingChat.messages.length > 0) {
      return
    }

    try {
      // Fetch turns and issues from server with pagination
      const [turnsResponse, issues] = await Promise.all([
        getChatTurns(token, chat.serverId, { limit: 50 }),
        getChatIssues(token, chat.serverId),
      ])

      // Create a map of turn_id -> issues for quick lookup
      const issuesByTurnId: Record<number, TextIssue[]> = {}
      for (const ann of issues) {
        const turnId = ann.chat_turn_id
        if (!issuesByTurnId[turnId]) {
          issuesByTurnId[turnId] = []
        }
        issuesByTurnId[turnId].push({
          type: ann.issue_type,
          original: ann.original_text || '',
          suggested: ann.suggested_text || '',
          description_en: ann.description_en || '',
          description_zh: ann.description_zh || '',
          severity: ann.severity || 'low',
          start_position: ann.start_position,
          end_position: ann.end_position,
        })
      }

      // Convert ChatTurn to Message format and update chat
      const messages: Message[] = turnsResponse.items
        .filter((turn: ChatTurn) => turn.status === 'completed' && (turn.content_en || turn.content_zh))
        .map((turn: ChatTurn) => ({
          id: turn.id.toString(),
          role: turn.speaker === 'user' ? 'user' : 'assistant' as const,
          contentEn: turn.content_en || '',
          contentZh: turn.content_zh || '',
          hasAudio: !!turn.audio_path,
          audioPath: turn.audio_path || undefined,
          timestamp: new Date(turn.created_at),
          issues: issuesByTurnId[turn.id],
        }))

      setChats(prev => prev.map(c => {
        if (c.id === chat.id) {
          return {
            ...c,
            messages,
            lastMessage: messages.length > 0 ? messages[messages.length - 1].contentEn : '',
            hasMorePrev: turnsResponse.has_prev,
            hasMoreNext: turnsResponse.has_next,
            firstId: turnsResponse.first_id ?? undefined,
            lastId: turnsResponse.last_id ?? undefined,
          }
        }
        return c
      }))
    } catch (err) {
      console.error('Failed to fetch chat turns:', err)
    }
  }, [token, chats])

  // Load more (older) messages for the active chat
  const handleLoadMoreMessages = useCallback(async () => {
    if (!activeChat?.serverId || !token || !activeChat.hasMorePrev || !activeChat.firstId) {
      return
    }

    try {
      // Fetch older turns (before the first message we have)
      const response = await getChatTurns(token, activeChat.serverId, {
        limit: 50,
        beforeId: activeChat.firstId,
      })

      // Convert ChatTurn to Message format
      const olderMessages: Message[] = response.items
        .filter((turn: ChatTurn) => turn.status === 'completed' && (turn.content_en || turn.content_zh))
        .map((turn: ChatTurn) => ({
          id: turn.id.toString(),
          role: turn.speaker === 'user' ? 'user' : 'assistant' as const,
          contentEn: turn.content_en || '',
          contentZh: turn.content_zh || '',
          hasAudio: !!turn.audio_path,
          audioPath: turn.audio_path || undefined,
          timestamp: new Date(turn.created_at),
        }))

      // Prepend older messages to existing messages
      setChats(prev => prev.map(c => {
        if (c.id === activeChat.id) {
          return {
            ...c,
            messages: [...olderMessages, ...c.messages],
            hasMorePrev: response.has_prev,
            firstId: response.first_id ?? c.firstId,
          }
        }
        return c
      }))
    } catch (err) {
      console.error('Failed to load more messages:', err)
    }
  }, [token, activeChat])

  const playAudio = async (messageId: string) => {
    // If already playing this message, stop it
    if (isPlayingAudio === messageId) {
      stopAudio()
      return
    }

    // Find the message and play its audio
    const message = activeChat?.messages.find(m => m.id === messageId)
    if (message?.audioBase64) {
      playAudioFromBase64(message.audioBase64, messageId)
    } else if (message?.audioPath && token) {
      // Fetch audio from server endpoint
      // audioPath format: "learn/audios/{user_id}/{filename}"
      try {
        setIsPlayingAudio(messageId) // Show loading state
        const response = await fetch(`/api/${message.audioPath}`, {
          headers: {
            Authorization: `Bearer ${token}`,
          },
        })
        if (!response.ok) {
          throw new Error(`Failed to fetch audio: ${response.status}`)
        }
        const blob = await response.blob()
        const audioUrl = URL.createObjectURL(blob)

        if (audioElementRef.current) {
          audioElementRef.current.pause()
        }

        const audio = new Audio(audioUrl)
        audioElementRef.current = audio

        audio.onended = () => {
          setIsPlayingAudio(null)
          URL.revokeObjectURL(audioUrl)
        }
        audio.onerror = () => {
          setIsPlayingAudio(null)
          URL.revokeObjectURL(audioUrl)
          console.error('Audio playback failed')
        }
        audio.play().catch(err => {
          console.error('Failed to play audio:', err)
          setIsPlayingAudio(null)
          URL.revokeObjectURL(audioUrl)
        })
      } catch (err) {
        console.error('Failed to fetch audio:', err)
        setIsPlayingAudio(null)
        // Fall back to TTS if audio fetch fails
        textToSpeech(token, message.contentEn)
          .then(response => {
            playAudioFromBase64(response.audio_base64, messageId)
          })
          .catch(ttsErr => {
            console.error('Failed to get TTS:', ttsErr)
          })
      }
    } else if (message && token) {
      // Fall back to TTS for messages without saved audio
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
    if (!activeChat) return

    const messagesHtml = activeChat.messages.map(msg => {
      const isUser = msg.role === 'user'
      let issuesHtml = ''
      if (reportMode && isUser && msg.issues && msg.issues.length > 0) {
        issuesHtml = `
          <div style="margin-top: 9px; padding: 8px 12px; background: #fef3c7; border-radius: 8px; font-size: 13px;">
            <div style="font-weight: 600; color: #92400e; margin-bottom: 4px;">改进建议:</div>
            ${msg.issues.map((issue: TextIssue) => `
              <div style="margin-bottom: 4px;">
                <span style="color: #dc2626; text-decoration: line-through;">${issue.original}</span>
                → <span style="color: #16a34a; font-weight: 500;">${issue.suggested}</span>
                <div style="color: #78716c; font-size: 12px; margin-top: 2px;">${issue.description_zh}</div>
              </div>
            `).join('')}
          </div>
        `
      }
      return `
        <div class="message-item" style="text-align: ${isUser ? 'right' : 'left'};">
          <div style="display: inline-block; max-width: 100%; padding: 12px 16px; border-radius: 16px; background: ${isUser ? '#f97316' : '#f3f4f6'}; color: ${isUser ? 'white' : '#111827'};">
            <p style="margin: 0;">${msg.contentEn}</p>
            <p style="margin: 8px 0 0 0; opacity: 0.8; font-size: 9px;">${msg.contentZh}</p>
          </div>
          ${issuesHtml}
        </div>
      `
    }).join('')

    const htmlContent = `<!DOCTYPE html>
      <html>
      <head>
        <title>${activeChat.title} - 对话记录</title>
        <style>
          @page {
            size: A4;
            margin: 15mm 15mm 15mm 15mm;
            @bottom-left {
              content: "开朗英语";
              font-size: 9px;
              color: #ea580c;
              font-weight: 500;
            }
            @bottom-center {
              content: "—  " counter(page) " / " counter(pages) "  —";
              font-size: 9px;
              color: #9ca3af;
            }
            @bottom-right {
              content: "https://cola.ng";
              font-size: 9px;
              color: #ea580c;
            }
          }
          body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 0; }
          .pdf-header { display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 16px; margin-bottom: 0; padding-bottom: 3px; border-bottom: 2px solid #f97316; }
          .pdf-header .brand { display: flex; flex-direction: column; gap: 4px; }
          .pdf-header .brand-row { display: flex; align-items: center; gap: 10px; }
          .pdf-header .logo { height: 32px; width: 32px; }
          .pdf-header .brand-name { font-size: 18px; font-weight: bold; color: #ea580c; }
          .pdf-header .pdf-url { color: #9ca3af; font-size: 12px; }
          .pdf-header .title-section { text-align: right; }
          .pdf-header .title { font-size: 18px; font-weight: bold; color: #111827; }
          .pdf-header .meta { color: #6b7280; font-size: 12px; margin-top: 4px; }
          .messages-container { column-count: 2; column-gap: 24px; margin-top: 16px; }
          .message-item { break-inside: avoid; margin-bottom: 16px; }
          @media print {
            body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
          }
        </style>
      </head>
      <body>
        <div class="pdf-header">
          <div class="brand">
            <div class="brand-row">
              <img class="logo" src="${window.location.origin}/colang-logo.svg" alt="Logo" id="logo-img" />
              <span class="brand-name">开朗英语</span>
            </div>
            <span class="pdf-url">https://cola.ng</span>
          </div>
          <div class="title-section">
            <div class="title">${activeChat.title}</div>
            <div class="meta">导出时间: ${new Date().toLocaleString()}${reportMode ? ' | 报告模式' : ''}</div>
          </div>
        </div>
        <div class="messages-container">
        ${messagesHtml}
        </div>
        <script>
          var logoImg = document.getElementById('logo-img');
          var printTriggered = false;
          function triggerPrint() {
            if (!printTriggered) {
              printTriggered = true;
              window.print();
            }
          }
          if (logoImg.complete) {
            triggerPrint();
          } else {
            logoImg.onload = triggerPrint;
            logoImg.onerror = triggerPrint;
          }
          // Fallback timeout
          setTimeout(triggerPrint, 1000);
        </script>
      </body>
      </html>`

    // Create Blob URL and open in new window
    const blob = new Blob([htmlContent], { type: 'text/html' })
    const url = URL.createObjectURL(blob)
    const printWindow = window.open(url, '_blank')

    // Clean up Blob URL after window is closed or after timeout
    if (printWindow) {
      const cleanup = () => URL.revokeObjectURL(url)
      printWindow.onafterprint = cleanup
      // Fallback cleanup after 60 seconds
      setTimeout(cleanup, 60000)
    } else {
      URL.revokeObjectURL(url)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.ctrlKey) {
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
            <h1 className="text-2xl font-bold text-gray-900 mb-2">天天唠嗑</h1>
            <p className="text-gray-600 mb-6">
              与 AI 进行真实的英语对话练习，提升口语表达能力
            </p>
            <Button asChild>
              <a href="/login?redirectTo=/chat">登录开始对话</a>
            </Button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50 flex flex-col">
      <Header />

      <main className="mx-auto w-full max-w-6xl px-4 pt-4 pb-5">
        <div className="bg-white rounded-xl shadow-lg overflow-hidden h-[calc(100vh-100px)] flex">
          {/* Left Sidebar - Chat History */}
          <div className="w-72 border-r flex flex-col bg-gray-50">
            <div className="p-3 border-b bg-white">
              <div className="flex gap-2">
                <Button onClick={handleNewFreeChat} variant="outline" size="sm" className="flex-1 gap-1 text-xs">
                  <MessageCircle className="h-3.5 w-3.5" />
                  随便聊
                </Button>
                <Button onClick={() => setShowContextDialog(true)} size="sm" className="flex-1 gap-1 text-xs">
                  <Map className="h-3.5 w-3.5" />
                  选场景
                </Button>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto">
              {chatsLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-orange-500" />
                </div>
              ) : chats.length === 0 ? (
                <div className="text-center py-8 text-gray-500 text-sm">
                  暂无对话，点击上方按钮开始
                </div>
              ) : chats.map((conv) => (
                <div
                  key={conv.id}
                  onClick={() => handleSelectChat(conv)}
                  className={cn(
                    'px-3 py-2.5 border-b cursor-pointer hover:bg-white transition-colors relative group',
                    activeChatId === conv.id && 'bg-white border-l-4 border-l-orange-500'
                  )}
                >
                  <div className="flex items-center gap-2.5">
                    <div className="w-8 h-8 rounded-full bg-orange-100 flex items-center justify-center flex-shrink-0">
                      {conv.icon ? (
                        <span className="text-base">{conv.icon}</span>
                      ) : (
                        <MessageCircle className="h-4 w-4 text-orange-600" />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-2">
                        <h3 className="font-medium text-gray-900 truncate text-sm">{conv.title}</h3>
                        <span className="text-xs text-gray-400 flex-shrink-0">
                          {conv.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </span>
                      </div>
                      <p className="text-xs text-gray-500 truncate">{conv.lastMessage || '开始对话...'}</p>
                    </div>
                    {/* Menu button */}
                    <div className="relative">
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          setMenuOpenId(menuOpenId === conv.id ? null : conv.id)
                        }}
                        className="p-1 rounded hover:bg-gray-200 opacity-0 group-hover:opacity-100 transition-opacity"
                      >
                        <MoreVertical className="h-4 w-4 text-gray-500" />
                      </button>
                      {/* Dropdown menu */}
                      {menuOpenId === conv.id && (
                        <div className="absolute right-0 top-full mt-1 bg-white rounded-lg shadow-lg border py-1 z-20 min-w-[100px]">
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              handlePinChat(conv.id)
                            }}
                            className="w-full px-3 py-1.5 text-left text-sm hover:bg-gray-100 flex items-center gap-2"
                          >
                            <Pin className="h-3.5 w-3.5" />
                            置顶
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              handleOpenRename(conv.id)
                            }}
                            className="w-full px-3 py-1.5 text-left text-sm hover:bg-gray-100 flex items-center gap-2"
                          >
                            <Pencil className="h-3.5 w-3.5" />
                            重命名
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              handleDeleteChat(conv.id)
                            }}
                            className="w-full px-3 py-1.5 text-left text-sm hover:bg-gray-100 flex items-center gap-2 text-red-600"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                            删除
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Right Panel - Chat Window */}
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Scrollable container with sticky header */}
            <div className="flex-1 overflow-y-auto">
            {/* Chat Header with Display Settings - sticky within scroll container */}
            <div className="border-b px-6 py-3 bg-white sticky top-0 z-10">
              <div className="flex items-center justify-between">
                <h2 className="font-semibold text-gray-900">
                  {activeChat?.title || '天天唠嗑'}
                </h2>
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-3 text-sm">
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
                      <div className="flex items-center gap-1.5">
                        <button
                          onClick={toggleBotEn}
                          className={cn(
                            'px-2 py-0.5 text-xs font-medium rounded-full transition-all',
                            showBotEn
                              ? 'bg-orange-500 text-white shadow-sm'
                              : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
                          )}
                        >
                          英
                        </button>
                        <button
                          onClick={toggleBotZh}
                          className={cn(
                            'px-2 py-0.5 text-xs font-medium rounded-full transition-all',
                            showBotZh
                              ? 'bg-orange-500 text-white shadow-sm'
                              : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
                          )}
                        >
                          中
                        </button>
                      </div>
                    </div>
                    <div className="flex items-center gap-2 border-l pl-3">
                      <span className="text-gray-500 text-xs">我:</span>
                      <div className="flex items-center gap-1.5">
                        <button
                          onClick={toggleUserEn}
                          className={cn(
                            'px-2 py-0.5 text-xs font-medium rounded-full transition-all',
                            showUserEn
                              ? 'bg-orange-500 text-white shadow-sm'
                              : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
                          )}
                        >
                          英
                        </button>
                        <button
                          onClick={toggleUserZh}
                          className={cn(
                            'px-2 py-0.5 text-xs font-medium rounded-full transition-all',
                            showUserZh
                              ? 'bg-orange-500 text-white shadow-sm'
                              : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
                          )}
                        >
                          中
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Messages */}
            <div className="p-6 space-y-4">
              {/* Load more button at top */}
              {activeChat?.hasMorePrev && (
                <div className="flex justify-center">
                  <button
                    onClick={handleLoadMoreMessages}
                    className="px-4 py-2 text-sm text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
                  >
                    加载更多历史消息...
                  </button>
                </div>
              )}
              {messages.map((message) => {
                const isUser = message.role === 'user'
                const showEn = isUser ? showUserEn : showBotEn
                const showZh = isUser ? showUserZh : showBotZh
                const lastDeselected = isUser ? lastDeselectedUser : lastDeselectedBot
                const bothOff = !showEn && !showZh

                // Determine what to display and blur when both are off
                const displayEn = showEn || (bothOff && lastDeselected === 'en')
                const displayZh = showZh || (bothOff && lastDeselected === 'zh')
                const blurEn = bothOff && lastDeselected === 'en'
                const blurZh = bothOff && lastDeselected === 'zh'

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
                        'max-w-[70%] rounded-2xl px-4 py-3 transition-all',
                        isUser
                          ? 'bg-orange-500 text-white'
                          : 'bg-gray-100 text-gray-900'
                      )}
                    >
                      {displayEn && (
                        <p className={cn('text-sm', blurEn && 'blur-sm select-none')}>{message.contentEn}</p>
                      )}
                      {displayEn && displayZh && (
                        <div className={cn(
                          'my-2 border-t',
                          isUser ? 'border-orange-400/30' : 'border-gray-200'
                        )} />
                      )}
                      {displayZh && (
                        <p className={cn(
                          'text-sm',
                          isUser ? 'text-orange-100' : 'text-gray-600',
                          blurZh && 'blur-sm select-none'
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
                        {(message.hasAudio || message.audioBase64 || !isUser) && (
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
                              <div className="h-4 w-4 flex items-center justify-center gap-[2px]">
                                <span className="w-[3px] h-2 bg-current rounded-full animate-sound-wave" style={{ animationDelay: '0ms' }} />
                                <span className="w-[3px] h-3 bg-current rounded-full animate-sound-wave" style={{ animationDelay: '150ms' }} />
                                <span className="w-[3px] h-2 bg-current rounded-full animate-sound-wave" style={{ animationDelay: '300ms' }} />
                              </div>
                            ) : (
                              <Volume2 className="h-4 w-4" />
                            )}
                          </button>
                        )}
                      </div>
                    </div>
                    {/* Issues display in report mode */}
                    {reportMode && isUser && message.issues && message.issues.length > 0 && (
                      <div className="max-w-[70%] mt-2 px-3 py-2 bg-amber-50 border border-amber-200 rounded-xl">
                        <div className="text-xs font-medium text-amber-700 mb-1.5 flex items-center gap-1">
                          <ClipboardList className="h-3 w-3" />
                          改进建议
                        </div>
                        {message.issues.map((issue, idx) => (
                          <div key={idx} className="text-sm mb-1.5 last:mb-0">
                            <span className="text-red-500 line-through">{issue.original}</span>
                            <span className="text-gray-400 mx-1">→</span>
                            <span className="text-green-600 font-medium">{issue.suggested}</span>
                            <p className="text-xs text-gray-500 mt-0.5">{issue.description_zh}</p>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )
              })}
              <div ref={messagesEndRef} />
            </div>
            </div>

            {/* Input Area */}
            <div className="border-t px-4 py-2 bg-white">
              {/* Row 1: Mic button */}
              <div className="flex flex-col items-center justify-center mb-2">
                {/* Mic Button */}
                <button
                  onClick={toggleRecording}
                  disabled={isVoiceProcessing}
                  className={cn(
                    'h-11 w-11 rounded-full flex items-center justify-center transition-all flex-shrink-0',
                    isVoiceProcessing
                      ? 'bg-gray-400 text-white cursor-not-allowed'
                      : isRecording
                        ? 'bg-red-500 text-white animate-pulse shadow-md shadow-red-200'
                        : 'bg-orange-500 text-white hover:bg-orange-600 shadow-md shadow-orange-200'
                  )}
                  title={isVoiceProcessing ? '处理中...' : isRecording ? '停止录音 (空格键)' : '开始录音 (空格键)'}
                >
                  {isVoiceProcessing ? (
                    <Loader2 className="h-5 w-5 animate-spin" />
                  ) : isRecording ? (
                    <MicOff className="h-5 w-5" />
                  ) : (
                    <Mic className="h-5 w-5" />
                  )}
                </button>
                {/* Recording status hint */}
                <div className={cn(
                  'mt-1 text-xs text-center',
                  isVoiceProcessing ? 'text-gray-500' : isRecording ? 'text-red-500' : 'text-gray-400'
                )}>
                  {isVoiceProcessing ? (
                    <span>正在处理语音...</span>
                  ) : isRecording ? (
                    <span className="flex items-center justify-center gap-1.5">
                      <span className="w-1.5 h-1.5 bg-red-500 rounded-full animate-pulse" />
                      {Math.floor(recordingDuration / 60)}:{(recordingDuration % 60).toString().padStart(2, '0')}
                      <span className="text-gray-400 ml-1">按 <kbd className="px-1 py-0.5 bg-gray-100 rounded text-gray-500">ESC</kbd> 取消</span>
                    </span>
                  ) : (
                    <span>按 <kbd className="px-1 py-0.5 bg-gray-100 rounded text-gray-500">空格</kbd> 开始录音</span>
                  )}
                </div>
              </div>

              {/* Row 2: Text input and send button */}
              <div className="flex items-center gap-2">
                <textarea
                  ref={textareaRef}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="输入文字消息... (Ctrl+Enter 换行)"
                  rows={1}
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-transparent resize-none overflow-y-auto text-sm"
                  style={{ height: '36px', maxHeight: '120px' }}
                />
                <Button
                  onClick={handleSend}
                  disabled={!input.trim() || isTextProcessing}
                  className="h-9 px-4"
                >
                  {isTextProcessing ? (
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

      {/* Context Selection Dialog */}
      {showContextDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50"
            onClick={() => setShowContextDialog(false)}
          />
          {/* Dialog */}
          <div className="relative bg-white rounded-xl shadow-xl max-w-3xl w-full mx-4 max-h-[80vh] flex flex-col">
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b">
              <h2 className="text-lg font-semibold text-gray-900">选择对话场景</h2>
              <button
                onClick={() => setShowContextDialog(false)}
                className="p-1 rounded-lg hover:bg-gray-100 transition-colors"
              >
                <X className="h-5 w-5 text-gray-500" />
              </button>
            </div>
            {/* Content */}
            <div className="flex-1 overflow-y-auto p-4">
              {contextsLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-8 w-8 animate-spin text-orange-500" />
                </div>
              ) : contexts.length === 0 ? (
                <div className="text-center py-12 text-gray-500">
                  暂无可用场景
                </div>
              ) : (
                <div className="grid grid-cols-3 gap-3">
                  {contexts.map((context) => (
                    <button
                      key={context.id}
                      onClick={() => handleNewContextChat(context)}
                      className="flex flex-col items-center gap-2 p-4 rounded-xl border hover:border-orange-300 hover:bg-orange-50 transition-all text-center"
                    >
                      <span className="text-3xl">{context.icon_emoji || '💬'}</span>
                      <span className="font-medium text-gray-900">{context.name_zh}</span>
                      <span className="text-xs text-gray-500">{context.name_en}</span>
                      {context.description_zh && (
                        <span className="text-xs text-gray-400 line-clamp-2">{context.description_zh}</span>
                      )}
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Rename Dialog */}
      {renameDialogId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="absolute inset-0 bg-black/50"
            onClick={() => setRenameDialogId(null)}
          />
          <div className="relative bg-white rounded-xl shadow-xl max-w-sm w-full mx-4 p-4">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">重命名对话</h3>
            <input
              type="text"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleConfirmRename()
                if (e.key === 'Escape') setRenameDialogId(null)
              }}
              className="w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 mb-4"
              autoFocus
            />
            <div className="flex justify-end gap-2">
              <Button variant="outline" size="sm" onClick={() => setRenameDialogId(null)}>
                取消
              </Button>
              <Button size="sm" onClick={handleConfirmRename}>
                确定
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Click outside to close menu */}
      {menuOpenId && (
        <div
          className="fixed inset-0"
          onClick={() => setMenuOpenId(null)}
        />
      )}

    </div>
  )
}
