export type User = {
  id: string
  email: string
  name: string | null
  phone: string | null
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

export type ChatSendResponse = {
  reply: string
  corrections: string[]
  suggestions: string[]
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
    throw new Error(text || `HTTP ${res.status}`)
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

export function chatSend(token: string, message: string): Promise<ChatSendResponse> {
  return requestJson<ChatSendResponse>('/api/chat/send', {
    method: 'POST',
    token,
    body: JSON.stringify({ message }),
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
  weekly_conversation_minutes: number
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

export type HistoryMessage = {
  role: 'user' | 'assistant'
  content: string
}

export type Correction = {
  original: string
  corrected: string
  explanation: string
}

export type VoiceChatResponse = {
  user_text: string | null
  ai_text: string
  ai_text_zh: string | null
  ai_audio_base64: string | null
  corrections: Correction[]
}

export type TtsResponse = {
  audio_base64: string
}

/**
 * Send audio for voice chat
 * @param token Auth token
 * @param audioBase64 Base64 encoded audio (WAV format)
 * @param history Conversation history
 * @param systemPrompt Optional custom system prompt
 */
export function voiceChatSend(
  token: string,
  audioBase64: string,
  history: HistoryMessage[] = [],
  systemPrompt?: string
): Promise<VoiceChatResponse> {
  return requestJson<VoiceChatResponse>('/api/voice-chat/send', {
    method: 'POST',
    token,
    body: JSON.stringify({
      audio_base64: audioBase64,
      history,
      system_prompt: systemPrompt,
    }),
  })
}

/**
 * Send text for chat with optional TTS response
 * @param token Auth token
 * @param message Text message
 * @param history Conversation history
 * @param generateAudio Whether to generate audio response
 * @param systemPrompt Optional custom system prompt
 */
export function textChatSend(
  token: string,
  message: string,
  history: HistoryMessage[] = [],
  generateAudio: boolean = true,
  systemPrompt?: string
): Promise<VoiceChatResponse> {
  return requestJson<VoiceChatResponse>('/api/voice-chat/text-send', {
    method: 'POST',
    token,
    body: JSON.stringify({
      message,
      history,
      generate_audio: generateAudio,
      system_prompt: systemPrompt,
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
  return requestJson<TtsResponse>('/api/voice-chat/tts', {
    method: 'POST',
    token,
    body: JSON.stringify({ text, voice, speed }),
  })
}
