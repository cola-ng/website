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
