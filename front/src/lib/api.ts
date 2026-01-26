export type User = {
  id: string
  email: string
  name: string | null
  phone: string | null
  avatar: string | null
  created_at: string
  updated_at: string
}

export type AuthResponse = {
  user: User
  access_token: string
}

export type OauthLoginResponse =
  | { status: 'ok'; user: User; access_token: string }
  | {
      status: 'needs_bind'
      oauth_identity_id: string
      provider: string
      email: string | null
    }


export type LearningRecord = {
  id: string
  user_id: string
  record_type: string
  content: unknown
  created_at: string
}

export type Word = {
  id: number
  word: string
  word_lower: string
  word_type: string | null
  language: string | null
  frequency: number | null
  difficulty: number | null
  syllable_count: number | null
  is_lemma: boolean | null
  word_count: number | null
  is_active: boolean | null
  created_by: number | null
  updated_by: number | null
  updated_at: string
  created_at: string
}

export type Definition = {
  id: number
  word_id: number
  language: string
  definition: string
  part_of_speech: string | null
  definition_order: number | null
  register: string | null
  region: string | null
  context: string | null
  usage_notes: string | null
  is_primary: boolean | null
  created_at: string
}

export type Sentence = {
  id: number
  language: string
  sentence: string
  source: string | null
  author: string | null
  priority_order: number | null
  difficulty: number | null
  is_common: boolean | null
  created_at: string
}

export type Pronunciation = {
  id: number
  word_id: number
  definition_id: number | null
  ipa: string
  audio_url: string | null
  audio_path: string | null
  dialect: string | null
  gender: string | null
  is_primary: boolean | null
  created_at: string
}

export type Relation = {
  id: number
  word_id: number
  relation_type: string | null
  related_word_id: number
  semantic_field: string | null
  relation_strength: number | null
  created_at: string
}

export type Etymology = {
  id: number
  origin_language: string | null
  origin_word: string | null
  origin_meaning: string | null
  language: string
  etymology: string
  first_attested_year: number | null
  historical_forms: unknown | null
  cognate_words: unknown | null
  created_at: string
}

export type Form = {
  id: number
  word_id: number
  form_type: string | null
  form: string
  is_irregular: boolean | null
  notes: string | null
  created_at: string
}

export type Category = {
  id: number
  name: string
  parent_id: number | null
  created_at: string
}

export type Image = {
  id: number
  word_id: number
  image_url: string | null
  image_path: string | null
  image_type: string | null
  alt_text_en: string | null
  alt_text_zh: string | null
  is_primary: boolean | null
  created_by: number | null
  created_at: string
}

export type Dictionary = {
  id: number
  name: string
  description_en: string | null
  description_zh: string | null
  version: string | null
  publisher: string | null
  license_type: string | null
  license_url: string | null
  source_url: string | null
  total_entries: number | null
  is_active: boolean | null
  is_official: boolean | null
  priority_order: number | null
  created_by: number | null
  updated_by: number | null
  updated_at: string
  created_at: string
}

export type WordDictionary = {
  id: number
  word_id: number
  dictionary_id: number
  definition_id: number | null
  priority_order: number | null
  name: string
  created_at: string
}

export type WordQueryResponse = {
  word: Word
  definitions: Definition[]
  sentences: Sentence[]
  pronunciations: Pronunciation[]
  relations: Relation[]
  etymologies: Etymology[]
  forms: Form[]
  categories: Category[]
  images: Image[]
  dictionaries: (WordDictionary & { dictionary: Dictionary })[]
}

// Error message mappings for user-friendly messages
const ERROR_MESSAGES: Record<string, string> = {
  'failed to create user': '注册失败，请稍后重试',
  'email already registered': '该邮箱已被注册',
  'phone already registered': '该手机号已被注册',
  'email or phone already registered': '该邮箱或手机号已被注册',
  'invalid credentials': '邮箱或密码错误',
  'user not found': '用户不存在',
  'email already exists': '该邮箱已被注册',
  'invalid email': '请输入有效的邮箱地址',
  'password too short': '密码长度不足',
  'password must be at least 8 characters': '密码至少需要8个字符',
  unauthorized: '请先登录',
  forbidden: '没有权限执行此操作',
}

function parseErrorMessage(text: string, status: number): string {
  // Try to parse as JSON error response
  try {
    const json = JSON.parse(text)
    const brief = json?.error?.brief || json?.error?.message || json?.message
    if (brief && typeof brief === 'string') {
      // Check if we have a friendly message for this error
      const lowerBrief = brief.toLowerCase()
      for (const [key, message] of Object.entries(ERROR_MESSAGES)) {
        if (lowerBrief.includes(key.toLowerCase())) {
          return message
        }
      }
      // Return the brief message if no mapping found
      return brief
    }
  } catch {
    // Not JSON, continue with text
  }

  // Check raw text against error messages
  const lowerText = text.toLowerCase()
  for (const [key, message] of Object.entries(ERROR_MESSAGES)) {
    if (lowerText.includes(key.toLowerCase())) {
      return message
    }
  }

  // Fallback based on status code
  if (status === 400) return '请求无效，请检查输入'
  if (status === 401) return '请先登录'
  if (status === 403) return '没有权限执行此操作'
  if (status === 404) return '请求的资源不存在'
  if (status === 409) return '数据冲突，请刷新后重试'
  if (status === 500) return '服务器错误，请稍后重试'

  return text || `请求失败 (${status})`
}

async function requestJson<T>(
  path: string,
  init: RequestInit & { token?: string } = {}
): Promise<T> {
  const headers = new Headers(init.headers)
  headers.set('accept', 'application/json')
  headers.set('content-type', 'application/json')
  if (init.token) headers.set('authorization', `Bearer ${init.token}`)

  const res = await fetch(path, { ...init, headers })
  if (!res.ok) {
    const text = await res.text().catch(() => '')
    throw new Error(parseErrorMessage(text, res.status))
  }
  return (await res.json()) as T
}

export function register(input: {
  email: string
  password: string
  name?: string
}): Promise<AuthResponse> {
  return requestJson<AuthResponse>('/api/register', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function login(input: {
  email: string
  password: string
}): Promise<AuthResponse> {
  return requestJson<AuthResponse>('/api/login', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function me(token: string): Promise<User> {
  return requestJson<User>('/api/me', { method: 'GET', token })
}

export function updateMe(
  token: string,
  input: { name?: string | null; phone?: string | null }
): Promise<User> {
  return requestJson<User>('/api/me', {
    method: 'PUT',
    token,
    body: JSON.stringify(input),
  })
}

export async function uploadAvatar(token: string, file: File): Promise<User> {
  const formData = new FormData()
  formData.append('image', file)

  const response = await fetch('/api/me/avatar', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
    },
    body: formData,
  })

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}))
    throw new Error(errorData.message || `Upload failed: ${response.status}`)
  }

  return response.json()
}

export function deleteAvatar(token: string): Promise<User> {
  return requestJson<User>('/api/me/avatar', {
    method: 'DELETE',
    token,
  })
}

/**
 * Fetch avatar with authentication and return blob URL
 * @param token Auth token
 * @returns Promise resolving to blob URL or null if no avatar
 */
export async function fetchAvatarUrl(token: string): Promise<string | null> {
  try {
    const response = await fetch('/api/me/avatar', {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    })
    if (!response.ok) {
      return null
    }
    const blob = await response.blob()
    return URL.createObjectURL(blob)
  } catch {
    return null
  }
}

/**
 * @deprecated Use fetchAvatarUrl instead for authenticated avatar access
 */
export function getAvatarUrl(user: User | null, _size: number = 160): string | null {
  if (!user) return null
  if (user.avatar) {
    // This won't work with authenticated endpoints - use fetchAvatarUrl instead
    return `/api/me/avatar`
  }
  return null
}

export function listRecords(token: string, limit = 50): Promise<LearningRecord[]> {
  return requestJson<LearningRecord[]>(`/api/me/records?limit=${limit}`, {
    method: 'GET',
    token,
  })
}

export function createRecord(token: string, input: {
  record_type: string
  content: unknown
}): Promise<LearningRecord> {
  return requestJson<LearningRecord>('/api/me/records', {
    method: 'POST',
    token,
    body: JSON.stringify(input),
  })
}

export function desktopCreateCode(
  token: string,
  input: { redirect_uri: string; state: string }
): Promise<{ code: string; redirect_uri: string; state: string; expires_at: string }> {
  return requestJson('/api/auth/code', {
    method: 'POST',
    token,
    body: JSON.stringify(input),
  })
}

export function desktopConsumeCode(input: {
  code: string
  redirect_uri: string
}): Promise<{ access_token: string }> {
  return requestJson('/api/auth/consume', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function oauthLogin(input: {
  provider: string
  provider_user_id: string
  email?: string
}): Promise<OauthLoginResponse> {
  return requestJson<OauthLoginResponse>('/api/auth/oauth/login', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function oauthBind(input: {
  oauth_identity_id: string
  email: string
  password: string
}): Promise<OauthLoginResponse> {
  return requestJson<OauthLoginResponse>('/api/auth/oauth/bind', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function oauthSkip(input: {
  oauth_identity_id: string
  name?: string
  email?: string
}): Promise<OauthLoginResponse> {
  return requestJson<OauthLoginResponse>('/api/auth/oauth/skip', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function lookup(word: string): Promise<WordQueryResponse> {
  return requestJson<WordQueryResponse>(`/api/dict/lookup?word=${encodeURIComponent(word)}`, {
    method: 'GET',
  })
}

// Search History types
export type SearchHistory = {
  id: number
  user_id: number
  word: string
  searched_at: string
}

export function saveSearchHistory(token: string, word: string): Promise<SearchHistory> {
  return requestJson<SearchHistory>('/api/dict/history', {
    method: 'POST',
    token,
    body: JSON.stringify({ word }),
  })
}

export function getSearchHistory(token: string, limit: number = 10): Promise<SearchHistory[]> {
  return requestJson<SearchHistory[]>(`/api/dict/history?limit=${limit}`, {
    method: 'GET',
    token,
  })
}

export function clearSearchHistory(token: string): Promise<{ cleared: boolean }> {
  return requestJson('/api/dict/history', {
    method: 'DELETE',
    token,
  })
}

// Learning Summary types
export type WeeklyMinutes = {
  day: number
  date: string
  minutes: number
}

export type LearnSummary = {
  has_data: boolean
  weekly_chat_minutes: number
  mastered_vocabulary_count: number
  pending_review_count: number
  weekly_minutes: WeeklyMinutes[]
}

export function getLearnSummary(token: string): Promise<LearnSummary> {
  return requestJson<LearnSummary>('/api/learn/summary', {
    method: 'GET',
    token,
  })
}

// ============================================================================
// Achievement System Types
// ============================================================================

export type RankInfo = {
  code: string
  name_en: string
  name_zh: string
  icon: string | null
  color: string | null
  level: number
  min_xp: number
}

export type AchievementBadge = {
  code: string
  name_en: string
  name_zh: string
  icon: string | null
  rarity: string
  completed_at: string | null
}

export type UserProfileSummary = {
  total_xp: number
  current_streak_days: number
  rank: RankInfo | null
  next_rank: RankInfo | null
  xp_to_next_rank: number
  recent_achievements: AchievementBadge[]
  total_achievements: number
  completed_achievements: number
}

export type AchievementWithProgress = {
  id: number
  code: string
  name_en: string
  name_zh: string
  description_en: string | null
  description_zh: string | null
  icon: string | null
  category: string
  rarity: string
  xp_reward: number
  requirement_value: number
  progress: number
  is_completed: boolean
  completed_at: string | null
}

// Achievement API functions
export function getUserProfileSummary(token: string): Promise<UserProfileSummary> {
  return requestJson<UserProfileSummary>('/api/achievements/profile', {
    method: 'GET',
    token,
  })
}

export function getUserAchievements(token: string): Promise<AchievementWithProgress[]> {
  return requestJson<AchievementWithProgress[]>('/api/achievements/my', {
    method: 'GET',
    token,
  })
}

export function getRankDefinitions(): Promise<RankInfo[]> {
  return requestJson<RankInfo[]>('/api/achievements/ranks', {
    method: 'GET',
  })
}

// ============================================================================
// Voice Chat Types and Functions
// ============================================================================

/** Text issue (grammar, word choice, or suggestion) */
export type TextIssue = {
  /** Type of issue: grammar | word_choice | suggestion */
  type: string
  /** Original problematic text */
  original: string
  /** Suggested correction */
  suggested: string
  /** Explanation in English */
  description_en: string
  /** Explanation in Chinese */
  description_zh: string
  /** Severity: low | medium | high */
  severity: string
  /** Start position in text (optional) */
  start_position: number | null
  /** End position in text (optional) */
  end_position: number | null
}

/** Chat response */
export type ChatResponse = {
  /** Language of user input: "en" | "zh" | "mix" */
  use_lang: string
  /** User text in English (original or transcribed) */
  user_text_en: string
  /** User text in Chinese */
  user_text_zh: string
  /** AI's text response in English */
  ai_text_en: string
  /** AI's text response in Chinese */
  ai_text_zh: string
  /** Base64 encoded audio of user's message (if audio input or TTS generated) */
  user_audio_base64: string | null
  /** Base64 encoded audio of AI response */
  ai_audio_base64: string | null
  /** Grammar/word choice issues found in user's text */
  issues: TextIssue[]
}

export type TtsResponse = {
  audio_base64: string
}

/**
 * Send audio for voice chat
 * Returns two turns: user (completed) and AI (processing)
 * @param token Auth token
 * @param chatId Chat ID
 * @param audioBase64 Base64 encoded audio (WAV format)
 */
export function voiceChatSend(
  token: string,
  chatId: number,
  audioBase64: string
): Promise<ChatSendResponse> {
  return requestJson<ChatSendResponse>(`/api/learn/chats/${chatId}/send`, {
    method: 'POST',
    token,
    body: JSON.stringify({
      type: 'audio',
      audio_base64: audioBase64,
    }),
  })
}

/**
 * Send text for chat
 * Returns two turns: user (completed) and AI (processing)
 * @param token Auth token
 * @param chatId Chat ID
 * @param message Text message
 * @param generateAudio Whether to generate audio response
 */
export function textChatSend(
  token: string,
  chatId: number,
  message: string,
  _generateAudio: boolean = true
): Promise<ChatSendResponse> {
  return requestJson<ChatSendResponse>(`/api/learn/chats/${chatId}/send`, {
    method: 'POST',
    token,
    body: JSON.stringify({
      type: 'text',
      message,
    }),
  })
}

/**
 * Convert text to speech
 * @param token Auth token
 * @param text Text to synthesize
 * @param voice Voice option
 * @param speed Speed (0.5 - 2.0)
 */
export function textToSpeech(
  token: string,
  text: string,
  voice?: string,
  speed?: number
): Promise<TtsResponse> {
  return requestJson<TtsResponse>('/api/chat/tts', {
    method: 'POST',
    token,
    body: JSON.stringify({ text, voice, speed }),
  })
}

/**
 * Delete all chats for current user
 * @param token Auth token
 */
export function clearAllChats(token: string): Promise<{ ok: boolean }> {
  return requestJson<{ ok: boolean }>('/api/learn/chats', {
    method: 'DELETE',
    token,
  })
}

/** Chat turn status */
export type ChatTurnStatus = 'processing' | 'completed' | 'error'

/** Chat turn from history with embedded issues */
export type ChatTurn = {
  id: number
  user_id: number
  chat_id: number
  speaker: string
  use_lang: string
  content_en: string
  content_zh: string
  audio_path: string | null
  duration_ms: number | null
  words_per_minute: number | null
  issues_count: number | null
  hesitation_count: number | null
  status: ChatTurnStatus
  error: string | null
  created_at: string
  /** Embedded issues for this turn (only present if issues_count > 0) */
  issues: ChatIssue[]
}

/** Paginated response with cursor-based pagination */
export type PaginatedResponse<T> = {
  /** Items in this page */
  items: T[]
  /** Total count of items matching the query */
  total: number
  /** Number of items requested */
  limit: number
  /** Whether there are more items before this page */
  has_prev: boolean
  /** Whether there are more items after this page */
  has_next: boolean
  /** ID of the first item (use as before_id for previous page) */
  first_id: number | null
  /** ID of the last item (use as after_id for next page) */
  last_id: number | null
}

/** Response from send_chat - returns two turns with embedded issues */
export type ChatSendResponse = {
  /** User's chat turn with embedded issues (status: completed) */
  user_turn: ChatTurn
  /** AI's chat turn (status: completed) */
  ai_turn: ChatTurn
}

/** Options for fetching chat turns with cursor-based pagination */
export type GetChatTurnsOptions = {
  /** Max number of turns to return (default 50, max 500) */
  limit?: number
  /** Load items after this ID (for loading newer messages) */
  afterId?: number
  /** Load items before this ID (for loading older messages) */
  beforeId?: number
  /** If true, load the latest messages first (default false) */
  fromLatest?: boolean
}

/**
 * Get chat turns for a specific chat with cursor-based pagination
 * @param token Auth token
 * @param chatId Chat ID
 * @param options Pagination options
 */
export function getChatTurns(
  token: string,
  chatId: number,
  options: GetChatTurnsOptions = {}
): Promise<PaginatedResponse<ChatTurn>> {
  const params = new URLSearchParams()
  if (options.limit) params.set('limit', options.limit.toString())
  if (options.afterId) params.set('after_id', options.afterId.toString())
  if (options.beforeId) params.set('before_id', options.beforeId.toString())
  if (options.fromLatest) params.set('from_latest', 'true')

  const queryString = params.toString()
  const url = `/api/learn/chats/${chatId}/turns${queryString ? `?${queryString}` : ''}`

  return requestJson<PaginatedResponse<ChatTurn>>(url, {
    method: 'GET',
    token,
  })
}

/**
 * Get all chat turns for current user with cursor-based pagination
 * @param token Auth token
 * @param options Pagination options
 */
export function getAllChatTurns(
  token: string,
  options: GetChatTurnsOptions = {}
): Promise<PaginatedResponse<ChatTurn>> {
  const params = new URLSearchParams()
  if (options.limit) params.set('limit', options.limit.toString())
  if (options.afterId) params.set('after_id', options.afterId.toString())
  if (options.beforeId) params.set('before_id', options.beforeId.toString())

  const queryString = params.toString()
  const url = `/api/learn/chats/turns${queryString ? `?${queryString}` : ''}`

  return requestJson<PaginatedResponse<ChatTurn>>(url, {
    method: 'GET',
    token,
  })
}

/**
 * Get a single chat turn by ID with long-polling support
 * Server will block for up to 30s if turn is still processing
 * @param token Auth token
 * @param turnId Turn ID
 * @returns The turn (may still be processing if server timeout)
 */
export function getChatTurn(token: string, turnId: number): Promise<ChatTurn> {
  return requestJson<ChatTurn>(`/api/learn/chats/turns/${turnId}`, {
    method: 'GET',
    token,
  })
}

/**
 * Poll for a chat turn to complete using long-polling endpoint
 * @param token Auth token
 * @param chatId Chat ID (unused, kept for backward compatibility)
 * @param turnId Turn ID to poll
 * @param intervalMs Polling interval in ms (default 1000) - used as fallback
 * @param maxAttempts Max polling attempts (default 60)
 * @returns The completed or errored turn
 */
export async function pollChatTurn(
  token: string,
  _chatId: number,
  turnId: number,
  intervalMs = 1000,
  maxAttempts = 60
): Promise<ChatTurn> {
  for (let i = 0; i < maxAttempts; i++) {
    // Use long-polling endpoint - server will block for up to 30s if processing
    const turn = await getChatTurn(token, turnId)

    if (turn.status === 'completed' || turn.status === 'error') {
      return turn
    }

    // If still processing after server timeout, wait a bit and retry
    await new Promise((resolve) => setTimeout(resolve, intervalMs))
  }

  throw new Error('Polling timeout')
}

// ============================================================================
// Learn Chat API
// ============================================================================

export type LearnChat = {
  id: number
  user_id: number
  title: string
  context_id: number | null
  icon_emoji: string | null
  duration_ms: number | null
  issues_count: number | null
  created_at: string
}

/**
 * Create a new chat
 * @param token Auth token
 * @param title Chat title
 * @param contextId Optional context ID for scenario-based chats
 */
export function createChat(
  token: string,
  title: string,
  contextId?: number
): Promise<LearnChat> {
  return requestJson<LearnChat>('/api/learn/chats', {
    method: 'POST',
    token,
    body: JSON.stringify({
      title,
      context_id: contextId,
    }),
  })
}

/**
 * Update a chat's title
 * @param token Auth token
 * @param chatId Chat ID
 * @param title New title
 */
export function updateChatTitle(token: string, chatId: number, title: string): Promise<LearnChat> {
  console.log('Updating chat title:', chatId, title)
  return requestJson<LearnChat>(`/api/learn/chats/${chatId}`, {
    method: 'PUT',
    token,
    body: JSON.stringify({ title }),
  })
}

/**
 * List user's chats
 * @param token Auth token
 * @param limit Max number of chats to return
 */
export function listChats(token: string, limit = 50): Promise<LearnChat[]> {
  return requestJson<LearnChat[]>(`/api/learn/chats?limit=${limit}`, {
    method: 'GET',
    token,
  })
}

/**
 * Reset a chat - delete all turns and issues
 * @param token Auth token
 * @param chatId Chat ID
 */
export function resetChat(token: string, chatId: number): Promise<{ ok: boolean }> {
  return requestJson<{ ok: boolean }>(`/api/learn/chats/${chatId}/reset`, {
    method: 'POST',
    token,
  })
}

// ============================================================================
// Chat Issues API
// ============================================================================

/** Chat issue (grammar/vocabulary feedback) */
export type ChatIssue = {
  id: number
  user_id: number
  chat_id: number
  chat_turn_id: number
  issue_type: string
  start_position: number | null
  end_position: number | null
  original_text: string | null
  suggested_text: string | null
  description_en: string | null
  description_zh: string | null
  severity: string | null
  created_at: string
}

/**
 * Get issues for a specific chat
 * @param token Auth token
 * @param chatId Chat ID
 */
export function getChatIssues(token: string, chatId: number): Promise<ChatIssue[]> {
  return requestJson<ChatIssue[]>(`/api/learn/chats/${chatId}/issues`, {
    method: 'GET',
    token,
  })
}

/**
 * Get issues for a specific chat turn
 * @param token Auth token
 * @param turnId Turn ID
 */
export function getTurnIssues(token: string, turnId: number): Promise<ChatIssue[]> {
  return requestJson<ChatIssue[]>(`/api/learn/chats/turns/${turnId}/issues`, {
    method: 'GET',
    token,
  })
}

/**
 * Delete a chat turn
 * @param token Auth token
 * @param turnId Turn ID
 */
export function deleteChatTurn(token: string, turnId: number): Promise<{ ok: boolean }> {
  return requestJson<{ ok: boolean }>(`/api/learn/chats/turns/${turnId}`, {
    method: 'DELETE',
    token,
  })
}

// ============================================================================
// Reading Practice API
// ============================================================================

/** Reading subject (exercise collection) */
export type ReadSubject = {
  id: number
  code: string
  title_en: string
  title_zh: string
  description_en: string | null
  description_zh: string | null
  difficulty: number | null
  subject_type: string | null
  created_at: string
}

/** Reading sentence */
export type ReadSentence = {
  id: number
  subject_id: number
  sentence_order: number
  content_en: string
  content_zh: string
  phonetic_transcription: string | null
  native_audio_path: string | null
  difficulty: number | null
  focus_sounds: string[] | null
  common_mistakes: string[] | null
}

/**
 * List reading subjects (exercises)
 * @param difficulty Optional difficulty filter
 * @param subjectType Optional subject type filter
 * @param limit Max number of subjects to return
 */
export function listReadSubjects(params?: {
  difficulty?: number
  type?: string
  limit?: number
}): Promise<ReadSubject[]> {
  const searchParams = new URLSearchParams()
  if (params?.difficulty !== undefined) searchParams.set('difficulty', params.difficulty.toString())
  if (params?.type) searchParams.set('type', params.type)
  if (params?.limit !== undefined) searchParams.set('limit', params.limit.toString())

  const query = searchParams.toString()
  return requestJson<ReadSubject[]>(`/api/asset/read/subjects${query ? `?${query}` : ''}`, {
    method: 'GET',
  })
}

/**
 * Get sentences for a reading subject
 * @param subjectId Subject ID
 */
export function getReadSentences(subjectId: number): Promise<ReadSentence[]> {
  return requestJson<ReadSentence[]>(`/api/asset/read/subjects/${subjectId}/sentences`, {
    method: 'GET',
  })
}
