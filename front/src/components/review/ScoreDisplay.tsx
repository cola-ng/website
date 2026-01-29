import { CheckCircle, AlertCircle } from 'lucide-react'
import { cn } from '../../lib/utils'
import type { PronunciationFeedback } from '../../lib/api'

/** Get text color class based on score */
export function getScoreColor(score: number): string {
  if (score >= 80) return 'text-green-600'
  if (score >= 60) return 'text-amber-600'
  return 'text-red-600'
}

/** Get background color class based on score */
export function getScoreBg(score: number): string {
  if (score >= 80) return 'bg-green-500'
  if (score >= 60) return 'bg-amber-500'
  return 'bg-red-500'
}

/** Get border color class based on score */
export function getScoreBorder(score: number): string {
  if (score >= 80) return 'border-green-500'
  if (score >= 60) return 'border-amber-500'
  return 'border-red-500'
}

/** Get background subtle color class based on score */
export function getScoreBgSubtle(score: number): string {
  if (score >= 80) return 'bg-green-50'
  if (score >= 60) return 'bg-amber-50'
  return 'bg-red-50'
}

interface ScoreCircleProps {
  score: number
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

/** Circular score badge with color coding */
export function ScoreCircle({ score, size = 'md', className }: ScoreCircleProps) {
  const sizeClasses = {
    sm: 'w-10 h-10 text-sm',
    md: 'w-14 h-14 text-xl',
    lg: 'w-20 h-20 text-2xl',
  }

  return (
    <div
      className={cn(
        'rounded-full border-3 flex items-center justify-center shrink-0',
        getScoreBorder(score),
        getScoreBgSubtle(score),
        sizeClasses[size],
        className
      )}
    >
      <span className={cn('font-bold', getScoreColor(score))}>{score}</span>
    </div>
  )
}

interface ScoreBarProps {
  label: string
  score: number
  className?: string
}

/** Single score bar with label and value */
export function ScoreBar({ label, score, className }: ScoreBarProps) {
  return (
    <div className={cn('flex items-center gap-2 text-xs', className)}>
      <span className="text-gray-500 w-16">{label}</span>
      <div className="flex-1 h-1.5 bg-gray-200 rounded-full">
        <div
          className={cn('h-1.5 rounded-full transition-all', getScoreBg(score))}
          style={{ width: `${score}%` }}
        />
      </div>
      <span className={cn('w-8 text-right font-medium', getScoreColor(score))}>{score}</span>
    </div>
  )
}

interface ScoreBarsProps {
  pronunciation: number
  fluency: number
  intonation: number
  className?: string
}

/** Three score bars for pronunciation metrics */
export function ScoreBars({ pronunciation, fluency, intonation, className }: ScoreBarsProps) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <ScoreBar label="发音" score={pronunciation} />
      <ScoreBar label="流畅" score={fluency} />
      <ScoreBar label="语调" score={intonation} />
    </div>
  )
}

interface FeedbackBadgesProps {
  feedback: PronunciationFeedback[]
  className?: string
}

/** Feedback badges with good/warning indicators */
export function FeedbackBadges({ feedback, className }: FeedbackBadgesProps) {
  if (feedback.length === 0) return null

  return (
    <div className={cn('flex flex-wrap gap-2', className)}>
      {feedback.map((item, index) => (
        <span
          key={index}
          className={cn(
            'inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs',
            item.type === 'good' ? 'bg-green-100 text-green-700' : 'bg-amber-100 text-amber-700'
          )}
        >
          {item.type === 'good' ? (
            <CheckCircle className="h-3 w-3" />
          ) : (
            <AlertCircle className="h-3 w-3" />
          )}
          {item.message}
        </span>
      ))}
    </div>
  )
}

interface TranscriptionDisplayProps {
  text: string
  className?: string
}

/** Display transcribed text from pronunciation evaluation */
export function TranscriptionDisplay({ text, className }: TranscriptionDisplayProps) {
  if (!text) return null

  return (
    <div className={cn('text-gray-600 bg-gray-50 rounded px-2 py-1.5 text-xs', className)}>
      <span className="text-gray-400">识别: </span>
      {text}
    </div>
  )
}

interface PronunciationResultProps {
  overallScore: number
  pronunciationScore: number
  fluencyScore: number
  intonationScore: number
  transcribedText?: string
  feedback?: PronunciationFeedback[]
  className?: string
}

/** Complete pronunciation evaluation result display */
export function PronunciationResult({
  overallScore,
  pronunciationScore,
  fluencyScore,
  intonationScore,
  transcribedText,
  feedback = [],
  className,
}: PronunciationResultProps) {
  return (
    <div className={cn('border-t pt-4', className)}>
      <div className="flex items-center gap-4">
        <ScoreCircle score={overallScore} />
        <ScoreBars
          pronunciation={pronunciationScore}
          fluency={fluencyScore}
          intonation={intonationScore}
          className="flex-1"
        />
      </div>

      <div className="mt-3 text-xs space-y-2">
        {transcribedText && <TranscriptionDisplay text={transcribedText} />}
        <FeedbackBadges feedback={feedback} />
      </div>
    </div>
  )
}
