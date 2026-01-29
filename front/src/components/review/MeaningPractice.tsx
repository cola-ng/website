import { useState, useEffect, useMemo } from 'react'
import { CheckCircle, XCircle, SkipForward, Volume2 } from 'lucide-react'
import { Button } from '../ui/button'
import { cn } from '../../lib/utils'
import { textToSpeech } from '../../lib/api'
import { ensureAudioContextRunning, queueAudioFromBase64 } from '../../lib/audio'
import type { ReviewItem, ReviewResult } from '../../types/review'

interface MeaningPracticeProps {
  item: ReviewItem
  token: string
  allItems: ReviewItem[] // For generating distractors
  onComplete: (result: ReviewResult) => void
  onSkip?: () => void
}

function shuffleArray<T>(array: T[]): T[] {
  const shuffled = [...array]
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]]
  }
  return shuffled
}

export function MeaningPractice({ item, token, allItems, onComplete, onSkip }: MeaningPracticeProps) {
  const [selected, setSelected] = useState<string | null>(null)
  const [showResult, setShowResult] = useState(false)
  const [startTime] = useState(() => Date.now())
  const [attempts, setAttempts] = useState(0)

  // Generate options: 1 correct + 3 distractors
  const options = useMemo(() => {
    const correctAnswer = item.word_zh || ''
    const distractors = allItems
      .filter((i) => i.id !== item.id && i.word_zh)
      .map((i) => i.word_zh!)
      .slice(0, 10) // Get up to 10 candidates

    // Shuffle and pick 3 distractors
    const shuffledDistractors = shuffleArray(distractors).slice(0, 3)

    // If not enough distractors, add some generic ones
    const fallbackOptions = ['其他含义', '未知词汇', '不确定']
    while (shuffledDistractors.length < 3) {
      const fallback = fallbackOptions[shuffledDistractors.length]
      if (fallback && !shuffledDistractors.includes(fallback)) {
        shuffledDistractors.push(fallback)
      }
    }

    // Combine and shuffle all options
    return shuffleArray([correctAnswer, ...shuffledDistractors])
  }, [item, allItems])

  const correctAnswer = item.word_zh || ''
  const isCorrect = selected === correctAnswer

  // Reset state when item changes
  useEffect(() => {
    setSelected(null)
    setShowResult(false)
    setAttempts(0)
  }, [item.id])

  const handleSelect = (option: string) => {
    if (showResult) return
    setSelected(option)
    setAttempts((a) => a + 1)
    setShowResult(true)
  }

  const handlePlayAudio = async () => {
    await ensureAudioContextRunning()
    try {
      const response = await textToSpeech(token, item.word)
      queueAudioFromBase64(response.audio_base64, 'word-audio')
    } catch (err) {
      console.error('TTS error:', err)
    }
  }

  const handleNext = () => {
    const result: ReviewResult = {
      item_id: item.id,
      result: isCorrect ? 'correct' : 'incorrect',
      time_spent_ms: Date.now() - startTime,
      attempts,
    }
    onComplete(result)
  }

  const handleRetry = () => {
    setSelected(null)
    setShowResult(false)
  }

  return (
    <div className="flex flex-col h-full">
      {/* Word display */}
      <div className="text-center mb-8">
        <div className="flex items-center justify-center gap-2 mb-2">
          <h2 className="text-3xl font-bold text-gray-900">{item.word}</h2>
          <Button variant="ghost" size="sm" onClick={handlePlayAudio} className="p-1">
            <Volume2 className="h-5 w-5 text-gray-500" />
          </Button>
        </div>
        {item.phonetic && <p className="text-gray-500 text-lg">{item.phonetic}</p>}
        <p className="text-gray-400 text-sm mt-2">选择正确的中文释义</p>
      </div>

      {/* Options */}
      <div className="grid grid-cols-1 gap-3 max-w-md mx-auto w-full mb-8">
        {options.map((option, index) => {
          const isSelected = selected === option
          const isThisCorrect = option === correctAnswer
          const showCorrectHighlight = showResult && isThisCorrect
          const showWrongHighlight = showResult && isSelected && !isThisCorrect

          return (
            <button
              key={index}
              onClick={() => handleSelect(option)}
              disabled={showResult}
              className={cn(
                'p-4 rounded-xl text-left transition-all border-2',
                !showResult && 'hover:border-orange-300 hover:bg-orange-50',
                !showResult && !isSelected && 'border-gray-200 bg-white',
                showCorrectHighlight && 'border-green-500 bg-green-50',
                showWrongHighlight && 'border-red-500 bg-red-50',
                showResult && !showCorrectHighlight && !showWrongHighlight && 'border-gray-200 bg-gray-50 opacity-50'
              )}
            >
              <div className="flex items-center justify-between">
                <span
                  className={cn(
                    'text-lg',
                    showCorrectHighlight && 'text-green-700 font-medium',
                    showWrongHighlight && 'text-red-700'
                  )}
                >
                  {option}
                </span>
                {showCorrectHighlight && <CheckCircle className="h-5 w-5 text-green-500" />}
                {showWrongHighlight && <XCircle className="h-5 w-5 text-red-500" />}
              </div>
            </button>
          )
        })}
      </div>

      {/* Result feedback */}
      {showResult && (
        <div className={cn('text-center mb-6', isCorrect ? 'text-green-600' : 'text-red-600')}>
          {isCorrect ? (
            <div className="flex items-center justify-center gap-2">
              <CheckCircle className="h-6 w-6" />
              <span className="text-lg font-medium">正确！</span>
            </div>
          ) : (
            <div>
              <div className="flex items-center justify-center gap-2 mb-1">
                <XCircle className="h-6 w-6" />
                <span className="text-lg font-medium">不对</span>
              </div>
              <p className="text-sm text-gray-600">
                正确答案: <span className="font-medium text-green-600">{correctAnswer}</span>
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
        {showResult && !isCorrect && (
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
