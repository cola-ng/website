import { useState, useEffect, useRef, useCallback } from 'react'
import { Volume2, CheckCircle, XCircle, SkipForward, Loader2 } from 'lucide-react'
import { Button } from '../ui/button'
import { cn } from '../../lib/utils'
import { textToSpeech } from '../../lib/api'
import { ensureAudioContextRunning, queueAudioFromBase64 } from '../../lib/audio'
import type { ReviewItem, ReviewResult } from '../../types/review'

interface SpellingPracticeProps {
  item: ReviewItem
  token: string
  onComplete: (result: ReviewResult) => void
  onSkip?: () => void
}

/** Calculate Levenshtein distance for partial credit */
function levenshteinDistance(a: string, b: string): number {
  const matrix: number[][] = []

  for (let i = 0; i <= b.length; i++) {
    matrix[i] = [i]
  }
  for (let j = 0; j <= a.length; j++) {
    matrix[0][j] = j
  }

  for (let i = 1; i <= b.length; i++) {
    for (let j = 1; j <= a.length; j++) {
      if (b.charAt(i - 1) === a.charAt(j - 1)) {
        matrix[i][j] = matrix[i - 1][j - 1]
      } else {
        matrix[i][j] = Math.min(
          matrix[i - 1][j - 1] + 1,
          matrix[i][j - 1] + 1,
          matrix[i - 1][j] + 1
        )
      }
    }
  }

  return matrix[b.length][a.length]
}

/** Calculate similarity score (0-100) based on Levenshtein distance */
function calculateSimilarity(input: string, target: string): number {
  const distance = levenshteinDistance(input.toLowerCase(), target.toLowerCase())
  const maxLength = Math.max(input.length, target.length)
  if (maxLength === 0) return 100
  return Math.round((1 - distance / maxLength) * 100)
}

export function SpellingPractice({ item, token, onComplete, onSkip }: SpellingPracticeProps) {
  const [userInput, setUserInput] = useState('')
  const [showResult, setShowResult] = useState(false)
  const [audioLoading, setAudioLoading] = useState(false)
  const [startTime] = useState(() => Date.now())
  const [attempts, setAttempts] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  const correctAnswer = item.word.toLowerCase().trim()
  const userAnswer = userInput.toLowerCase().trim()
  const isExactMatch = userAnswer === correctAnswer
  const similarity = calculateSimilarity(userAnswer, correctAnswer)
  const isPartialMatch = similarity >= 80 && !isExactMatch

  // Reset and auto-play when item changes
  useEffect(() => {
    setUserInput('')
    setShowResult(false)
    setAttempts(0)
    inputRef.current?.focus()
    // Auto-play audio on mount
    handlePlayAudio()
  }, [item.id])

  const handlePlayAudio = useCallback(async () => {
    if (audioLoading) return
    await ensureAudioContextRunning()
    setAudioLoading(true)
    try {
      const response = await textToSpeech(token, item.word)
      queueAudioFromBase64(response.audio_base64, 'spelling-audio')
    } catch (err) {
      console.error('TTS error:', err)
    } finally {
      setAudioLoading(false)
    }
  }, [audioLoading, token, item.word])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!userInput.trim() || showResult) return
    setAttempts((a) => a + 1)
    setShowResult(true)
  }

  const handleNext = () => {
    const result: ReviewResult = {
      item_id: item.id,
      result: isExactMatch ? 'correct' : isPartialMatch ? 'partial' : 'incorrect',
      score: similarity,
      time_spent_ms: Date.now() - startTime,
      attempts,
    }
    onComplete(result)
  }

  const handleRetry = () => {
    setUserInput('')
    setShowResult(false)
    inputRef.current?.focus()
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && showResult) {
      e.preventDefault()
      handleNext()
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Audio prompt */}
      <div className="text-center mb-8">
        <Button
          variant="outline"
          size="lg"
          onClick={handlePlayAudio}
          disabled={audioLoading}
          className="gap-3 px-8 py-6 text-lg"
        >
          {audioLoading ? (
            <Loader2 className="h-6 w-6 animate-spin" />
          ) : (
            <Volume2 className="h-6 w-6" />
          )}
          播放发音
        </Button>
        <p className="text-gray-400 text-sm mt-4">听发音，拼写单词</p>
      </div>

      {/* Hint */}
      {item.word_zh && (
        <div className="text-center mb-6">
          <p className="text-gray-600 text-lg">
            提示: <span className="font-medium">{item.word_zh}</span>
          </p>
        </div>
      )}

      {/* Input form */}
      <form onSubmit={handleSubmit} className="max-w-md mx-auto w-full mb-6">
        <input
          ref={inputRef}
          type="text"
          value={userInput}
          onChange={(e) => setUserInput(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={showResult}
          placeholder="输入单词拼写..."
          className={cn(
            'w-full px-4 py-3 text-xl text-center border-2 rounded-xl transition-all',
            'focus:outline-none focus:ring-2 focus:ring-orange-500',
            !showResult && 'border-gray-300',
            showResult && isExactMatch && 'border-green-500 bg-green-50',
            showResult && isPartialMatch && 'border-amber-500 bg-amber-50',
            showResult && !isExactMatch && !isPartialMatch && 'border-red-500 bg-red-50'
          )}
          autoComplete="off"
          autoCapitalize="off"
          spellCheck={false}
        />
        {!showResult && (
          <Button type="submit" className="w-full mt-4" disabled={!userInput.trim()}>
            确认
          </Button>
        )}
      </form>

      {/* Result feedback */}
      {showResult && (
        <div className="text-center mb-6">
          {isExactMatch ? (
            <div className="flex items-center justify-center gap-2 text-green-600">
              <CheckCircle className="h-6 w-6" />
              <span className="text-lg font-medium">完全正确！</span>
            </div>
          ) : isPartialMatch ? (
            <div className="text-amber-600">
              <div className="flex items-center justify-center gap-2 mb-1">
                <CheckCircle className="h-6 w-6" />
                <span className="text-lg font-medium">接近正确 ({similarity}%)</span>
              </div>
              <p className="text-sm text-gray-600">
                正确拼写: <span className="font-medium text-green-600">{item.word}</span>
              </p>
            </div>
          ) : (
            <div className="text-red-600">
              <div className="flex items-center justify-center gap-2 mb-1">
                <XCircle className="h-6 w-6" />
                <span className="text-lg font-medium">拼写错误</span>
              </div>
              <p className="text-sm text-gray-600">
                正确拼写: <span className="font-medium text-green-600">{item.word}</span>
              </p>
            </div>
          )}
        </div>
      )}

      {/* Action buttons */}
      <div className="flex justify-center gap-4 mt-auto">
        {onSkip && !showResult && (
          <Button variant="ghost" onClick={onSkip}>
            跳过
          </Button>
        )}
        {showResult && !isExactMatch && (
          <Button variant="outline" onClick={handleRetry}>
            再试一次
          </Button>
        )}
        {showResult && (
          <Button onClick={handleNext} className="gap-2">
            下一个
            <SkipForward className="h-4 w-4" />
          </Button>
        )}
      </div>
    </div>
  )
}
