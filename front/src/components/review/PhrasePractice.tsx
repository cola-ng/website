import { useState, useEffect, useRef } from 'react'
import { CheckCircle, XCircle, SkipForward, HelpCircle } from 'lucide-react'
import { Button } from '../ui/button'
import { cn } from '../../lib/utils'
import type { ReviewItem, ReviewResult } from '../../types/review'

interface PhrasePracticeProps {
  item: ReviewItem
  token: string
  onComplete: (result: ReviewResult) => void
  onSkip?: () => void
}

/** Parse phrase to find target word and create fill-in-the-blank pattern */
function parsePhrase(phrase: string, targetWord: string): { before: string; after: string } | null {
  const lowerPhrase = phrase.toLowerCase()
  const lowerTarget = targetWord.toLowerCase()
  const index = lowerPhrase.indexOf(lowerTarget)

  if (index === -1) return null

  return {
    before: phrase.slice(0, index),
    after: phrase.slice(index + targetWord.length),
  }
}

export function PhrasePractice({ item, token, onComplete, onSkip }: PhrasePracticeProps) {
  const [userInput, setUserInput] = useState('')
  const [showResult, setShowResult] = useState(false)
  const [showHint, setShowHint] = useState(false)
  const [startTime] = useState(() => Date.now())
  const [attempts, setAttempts] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  const correctAnswer = item.word.toLowerCase().trim()
  const userAnswer = userInput.toLowerCase().trim()
  const isCorrect = userAnswer === correctAnswer

  // Parse example phrase to create fill-in-the-blank
  const parsedPhrase = item.example ? parsePhrase(item.example, item.word) : null

  // Reset when item changes
  useEffect(() => {
    setUserInput('')
    setShowResult(false)
    setShowHint(false)
    setAttempts(0)
    inputRef.current?.focus()
  }, [item.id])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!userInput.trim() || showResult) return
    setAttempts((a) => a + 1)
    setShowResult(true)
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
    setUserInput('')
    setShowResult(false)
    inputRef.current?.focus()
  }

  const handleShowHint = () => {
    setShowHint(true)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && showResult) {
      e.preventDefault()
      handleNext()
    }
  }

  // If no example phrase, show simple word practice
  if (!parsedPhrase) {
    return (
      <div className="flex flex-col h-full">
        <div className="text-center mb-8">
          <p className="text-gray-400 text-sm mb-2">填写正确的单词</p>
          {item.word_zh && <h2 className="text-2xl font-bold text-gray-900 mb-2">{item.word_zh}</h2>}
        </div>

        <form onSubmit={handleSubmit} className="max-w-md mx-auto w-full mb-6">
          <input
            ref={inputRef}
            type="text"
            value={userInput}
            onChange={(e) => setUserInput(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={showResult}
            placeholder="输入单词..."
            className={cn(
              'w-full px-4 py-3 text-xl text-center border-2 rounded-xl transition-all',
              'focus:outline-none focus:ring-2 focus:ring-orange-500',
              !showResult && 'border-gray-300',
              showResult && isCorrect && 'border-green-500 bg-green-50',
              showResult && !isCorrect && 'border-red-500 bg-red-50'
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

        {showResult && (
          <div className="text-center mb-6">
            {isCorrect ? (
              <div className="flex items-center justify-center gap-2 text-green-600">
                <CheckCircle className="h-6 w-6" />
                <span className="text-lg font-medium">正确！</span>
              </div>
            ) : (
              <div className="text-red-600">
                <div className="flex items-center justify-center gap-2 mb-1">
                  <XCircle className="h-6 w-6" />
                  <span className="text-lg font-medium">不对</span>
                </div>
                <p className="text-sm text-gray-600">
                  正确答案: <span className="font-medium text-green-600">{item.word}</span>
                </p>
              </div>
            )}
          </div>
        )}

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

  return (
    <div className="flex flex-col h-full">
      {/* Instructions */}
      <div className="text-center mb-6">
        <p className="text-gray-400 text-sm">填写正确的单词完成短语</p>
      </div>

      {/* Phrase with blank */}
      <div className="bg-white rounded-xl shadow-lg p-6 mb-6 max-w-lg mx-auto w-full">
        <form onSubmit={handleSubmit}>
          <p className="text-xl text-gray-800 leading-relaxed flex items-center justify-center flex-wrap gap-1">
            <span>{parsedPhrase.before}</span>
            <input
              ref={inputRef}
              type="text"
              value={userInput}
              onChange={(e) => setUserInput(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={showResult}
              placeholder="____"
              className={cn(
                'w-32 px-2 py-1 text-xl text-center border-b-2 transition-all bg-transparent',
                'focus:outline-none',
                !showResult && 'border-gray-300 focus:border-orange-500',
                showResult && isCorrect && 'border-green-500 text-green-600 font-medium',
                showResult && !isCorrect && 'border-red-500 text-red-600'
              )}
              autoComplete="off"
              autoCapitalize="off"
              spellCheck={false}
            />
            <span>{parsedPhrase.after}</span>
          </p>
        </form>

        {/* Chinese translation */}
        {item.example_zh && (
          <p className="text-gray-500 text-sm text-center mt-4">{item.example_zh}</p>
        )}
      </div>

      {/* Hint */}
      {!showResult && !showHint && (
        <div className="text-center mb-4">
          <Button variant="ghost" size="sm" onClick={handleShowHint} className="gap-1 text-gray-500">
            <HelpCircle className="h-4 w-4" />
            显示提示
          </Button>
        </div>
      )}
      {showHint && !showResult && (
        <div className="text-center mb-4">
          <p className="text-gray-600">
            提示: <span className="font-medium">{item.word_zh}</span>
          </p>
        </div>
      )}

      {/* Submit button */}
      {!showResult && (
        <div className="flex justify-center mb-6">
          <Button type="submit" onClick={handleSubmit} disabled={!userInput.trim()} className="px-8">
            确认
          </Button>
        </div>
      )}

      {/* Result feedback */}
      {showResult && (
        <div className="text-center mb-6">
          {isCorrect ? (
            <div className="flex items-center justify-center gap-2 text-green-600">
              <CheckCircle className="h-6 w-6" />
              <span className="text-lg font-medium">正确！</span>
            </div>
          ) : (
            <div className="text-red-600">
              <div className="flex items-center justify-center gap-2 mb-1">
                <XCircle className="h-6 w-6" />
                <span className="text-lg font-medium">不对</span>
              </div>
              <p className="text-sm text-gray-600">
                正确答案: <span className="font-medium text-green-600">{item.word}</span>
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
