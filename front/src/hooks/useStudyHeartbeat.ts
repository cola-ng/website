import { useEffect, useRef, useCallback } from 'react'
import { useAuth } from '../lib/auth'

// Page types that can be tracked
export type PageType = 'chat' | 'review' | 'stage' | 'read' | 'dictionary' | 'translate'

// Heartbeat interval in milliseconds
const HEARTBEAT_INTERVAL = 20000 // 20 seconds

// Send heartbeat request
async function sendHeartbeat(token: string, pageType: PageType): Promise<void> {
  try {
    const response = await fetch('/api/learn/heartbeat', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({ page_type: pageType }),
    })

    if (!response.ok) {
      console.warn('Heartbeat failed:', response.status)
    }
  } catch (error) {
    // Silent failure - don't affect user experience
    console.warn('Heartbeat error:', error)
  }
}

/**
 * Study time heartbeat hook
 *
 * Sends heartbeat every 20 seconds while the page is visible and focused.
 * Used to track study time on different pages.
 *
 * @param pageType - The type of page (chat, review, stage, read, dictionary, translate)
 *
 * @example
 * ```tsx
 * function ChatPage() {
 *   useStudyHeartbeat('chat')
 *   // ...
 * }
 * ```
 */
export function useStudyHeartbeat(pageType: PageType) {
  const { token } = useAuth()
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const isActiveRef = useRef(true)
  const lastHeartbeatRef = useRef<number>(0)

  // Send heartbeat with throttling
  const doHeartbeat = useCallback(() => {
    if (!token || !isActiveRef.current) return

    const now = Date.now()
    // Prevent duplicate sends in short time
    if (now - lastHeartbeatRef.current < HEARTBEAT_INTERVAL * 0.8) {
      return
    }

    lastHeartbeatRef.current = now
    sendHeartbeat(token, pageType)
  }, [token, pageType])

  // Handle visibility change
  const handleVisibilityChange = useCallback(() => {
    if (document.hidden) {
      isActiveRef.current = false
    } else {
      isActiveRef.current = true
      // Send heartbeat immediately when page becomes visible
      doHeartbeat()
    }
  }, [doHeartbeat])

  // Handle window focus
  const handleFocus = useCallback(() => {
    isActiveRef.current = true
    doHeartbeat()
  }, [doHeartbeat])

  // Handle window blur
  const handleBlur = useCallback(() => {
    // Mark as inactive but don't completely stop
    // This way if user switches tabs but comes back quickly,
    // we can still maintain continuity
    isActiveRef.current = false
  }, [])

  useEffect(() => {
    if (!token) return

    // Send first heartbeat immediately
    doHeartbeat()

    // Set up interval
    intervalRef.current = setInterval(() => {
      if (isActiveRef.current) {
        doHeartbeat()
      }
    }, HEARTBEAT_INTERVAL)

    // Add event listeners
    document.addEventListener('visibilitychange', handleVisibilityChange)
    window.addEventListener('focus', handleFocus)
    window.addEventListener('blur', handleBlur)

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
      }
      document.removeEventListener('visibilitychange', handleVisibilityChange)
      window.removeEventListener('focus', handleFocus)
      window.removeEventListener('blur', handleBlur)
    }
  }, [token, doHeartbeat, handleVisibilityChange, handleFocus, handleBlur])
}

/**
 * Study time statistics response
 */
export interface StudyTimeStats {
  daily_stats: Array<{
    id: number
    user_id: number
    stat_date: string
    chat_seconds: number
    review_seconds: number
    stage_seconds: number
    read_seconds: number
    dictionary_seconds: number
    translate_seconds: number
    total_seconds: number
    heartbeat_count: number
  }>
  total_seconds: number
  today_seconds: number
}

/**
 * Fetch study time statistics
 *
 * @param token - Auth token
 * @param days - Number of days to fetch (default: 7, max: 365)
 */
export async function getStudyTimeStats(
  token: string,
  days: number = 7
): Promise<StudyTimeStats> {
  const response = await fetch(`/api/learn/study-time?days=${days}`, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  })

  if (!response.ok) {
    throw new Error('Failed to fetch study time stats')
  }

  return response.json()
}

/**
 * Format seconds to human-readable duration
 *
 * @param seconds - Total seconds
 * @returns Formatted string like "1小时30分钟" or "45分钟"
 */
export function formatStudyDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)

  if (hours > 0) {
    return `${hours}小时${minutes}分钟`
  }
  return `${minutes}分钟`
}
