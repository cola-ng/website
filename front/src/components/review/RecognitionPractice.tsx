import { useState, useEffect, useCallback } from 'react'
import { Volume2, SkipForward, Loader2 } from 'lucide-react'
import { Button } from '../ui/button'
import { cn } from '../../lib/utils'
import { textToSpeech } from '../../lib/api'
import { ensureAudioContextRunning, queueAudioFromBase64 } from '../../lib/audio'
import type { ReviewItem, ReviewResult } from '../../types/review'

interface RecognitionPracticeProps {
  item: ReviewItem
  token: string
  onComplete: (result: ReviewResult) => void
  onSkip?: () => void
}

type RatingType = 'forgot' | 'hard' | 'good' | 'easy'

const RATINGS: { type: RatingType; label: string; shortcut: string; color: string }[] = [
  { type: 'forgot', label: '忘记了', shortcut: '1', color: 'bg-red-500 hover:bg-red-600' },
  { type: 'hard', label: '有点难', shortcut: '2', color: 'bg-amber-500 hover:bg-amber-600' },
  { type: 'good', label: '记住了', shortcut: '3', color: 'bg-green-500 hover:bg-green-600' },
  { type: 'easy', label: '太简单', shortcut: '4', color: 'bg-blue-500 hover:bg-blue-600' },
]

export function RecognitionPractice({ item, token, onComplete, onSkip }: RecognitionPracticeProps) {
  const [flipped, setFlipped] = useState(false)
  const [audioLoading, setAudioLoading] = useState(false)
  const [startTime] = useState(() => Date.now())

  // Reset when item changes
  useEffect(() => {
    setFlipped(false)
  }, [item.id])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if user is typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return

      if (e.code === 'Space' || e.code === 'Enter') {
        e.preventDefault()
        if (!flipped) {
          setFlipped(true)
        }
      } else if (flipped) {
        const rating = RATINGS.find((r) => r.shortcut === e.key)
        if (rating) {
          e.preventDefault()
          handleRate(rating.type)
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [flipped])

  const handlePlayAudio = useCallback(async () => {
    if (audioLoading) return
    await ensureAudioContextRunning()
    setAudioLoading(true)
    try {
      const response = await textToSpeech(token, item.word)
      queueAudioFromBase64(response.audio_base64, 'recognition-audio')
    } catch (err) {
      console.error('TTS error:', err)
    } finally {
      setAudioLoading(false)
    }
  }, [audioLoading, token, item.word])

  const handleFlip = () => {
    if (!flipped) {
      setFlipped(true)
    }
  }

  const handleRate = (rating: RatingType) => {
    const resultMap: Record<RatingType, ReviewResult['result']> = {
      forgot: 'incorrect',
      hard: 'partial',
      good: 'correct',
      easy: 'correct',
    }

    const scoreMap: Record<RatingType, number> = {
      forgot: 0,
      hard: 50,
      good: 80,
      easy: 100,
    }

    const result: ReviewResult = {
      item_id: item.id,
      result: resultMap[rating],
      score: scoreMap[rating],
      time_spent_ms: Date.now() - startTime,
      attempts: 1,
    }
    onComplete(result)
  }

  return (
    <div className="flex flex-col h-full">
      {/* Flashcard */}
      <div
        onClick={handleFlip}
        className={cn(
          'flex-1 max-h-[400px] mx-auto w-full max-w-md cursor-pointer',
          'perspective-1000'
        )}
      >
        <div
          className={cn(
            'relative w-full h-full transition-transform duration-500 transform-style-preserve-3d',
            flipped && 'rotate-y-180'
          )}
        >
          {/* Front - Word */}
          <div
            className={cn(
              'absolute inset-0 backface-hidden',
              'bg-white rounded-2xl shadow-lg p-8',
              'flex flex-col items-center justify-center'
            )}
          >
            <h2 className="text-4xl font-bold text-gray-900 mb-4">{item.word}</h2>
            {item.phonetic && <p className="text-gray-500 text-xl mb-4">{item.phonetic}</p>}
            <Button
              variant="ghost"
              size="sm"
              onClick={(e) => {
                e.stopPropagation()
                handlePlayAudio()
              }}
              className="gap-2"
            >
              {audioLoading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Volume2 className="h-4 w-4" />}
              播放发音
            </Button>
            <p className="text-gray-400 text-sm mt-6">点击卡片或按空格键翻转</p>
          </div>

          {/* Back - Meaning */}
          <div
            className={cn(
              'absolute inset-0 backface-hidden rotate-y-180',
              'bg-gradient-to-br from-orange-50 to-amber-50 rounded-2xl shadow-lg p-8',
              'flex flex-col items-center justify-center'
            )}
          >
            <h3 className="text-2xl font-bold text-gray-900 mb-4">{item.word}</h3>
            {item.word_zh && <p className="text-xl text-gray-700 mb-4">{item.word_zh}</p>}
            {item.example && (
              <div className="text-center mt-4 max-w-sm">
                <p className="text-gray-600 text-sm italic">"{item.example}"</p>
                {item.example_zh && <p className="text-gray-500 text-xs mt-1">{item.example_zh}</p>}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Rating buttons - only show when flipped */}
      {flipped && (
        <div className="mt-6">
          <p className="text-center text-gray-500 text-sm mb-4">你记住了吗？</p>
          <div className="flex justify-center gap-3">
            {RATINGS.map((rating) => (
              <Button
                key={rating.type}
                onClick={() => handleRate(rating.type)}
                className={cn('flex-1 max-w-[100px] text-white', rating.color)}
              >
                <span className="text-xs opacity-60 mr-1">{rating.shortcut}</span>
                {rating.label}
              </Button>
            ))}
          </div>
          <p className="text-center text-gray-400 text-xs mt-3">按数字键 1-4 快速选择</p>
        </div>
      )}

      {/* Skip button - only show when not flipped */}
      {!flipped && onSkip && (
        <div className="flex justify-center mt-6">
          <Button variant="ghost" onClick={onSkip} className="gap-2">
            跳过
            <SkipForward className="h-4 w-4" />
          </Button>
        </div>
      )}

      {/* CSS for 3D transforms */}
      <style>{`
        .perspective-1000 {
          perspective: 1000px;
        }
        .transform-style-preserve-3d {
          transform-style: preserve-3d;
        }
        .backface-hidden {
          backface-visibility: hidden;
        }
        .rotate-y-180 {
          transform: rotateY(180deg);
        }
      `}</style>
    </div>
  )
}
