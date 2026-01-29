/** Review types supported by the system */
export type ReviewType =
  | 'pronunciation' // 发音不准 - Audio recording practice
  | 'meaning' // 意思不明确 - Meaning selection/matching
  | 'spelling' // 不会书写 - Spelling from audio
  | 'recognition' // 不认识 - Flashcard recognition
  | 'phrase' // 短语不熟悉 - Phrase context practice

/** A single item to review */
export interface ReviewItem {
  id: number
  word: string
  word_zh: string | null
  phonetic?: string
  example?: string
  example_zh?: string
  review_type: ReviewType
  mastery_level: number
  source: 'vocabulary' | 'issue_word'
  source_id: number
}

/** Result of a single review attempt */
export interface ReviewResult {
  item_id: number
  result: 'correct' | 'partial' | 'incorrect'
  score?: number // 0-100 for pronunciation
  time_spent_ms: number
  attempts: number
}

/** An active review session */
export interface ReviewSession {
  id: string
  review_type: ReviewType
  items: ReviewItem[]
  current_index: number
  results: ReviewResult[]
  started_at: Date
}

/** Statistics for each review type */
export interface ReviewTypeStats {
  pronunciation: number
  meaning: number
  spelling: number
  recognition: number
  phrase: number
}

/** Review type metadata for UI display */
export interface ReviewTypeInfo {
  type: ReviewType
  title: string
  description: string
  icon: string
}

/** All review types with their display info */
export const REVIEW_TYPES: ReviewTypeInfo[] = [
  {
    type: 'pronunciation',
    title: '发音纠正',
    description: '练习准确发音，AI评分反馈',
    icon: 'mic',
  },
  {
    type: 'meaning',
    title: '词义复习',
    description: '巩固单词释义，选择正确答案',
    icon: 'book-open',
  },
  {
    type: 'spelling',
    title: '拼写练习',
    description: '听音拼写，强化记忆',
    icon: 'pencil',
  },
  {
    type: 'recognition',
    title: '单词识记',
    description: '翻卡片复习，自评掌握程度',
    icon: 'layers',
  },
  {
    type: 'phrase',
    title: '短语搭配',
    description: '填空练习，掌握常用搭配',
    icon: 'puzzle',
  },
]
