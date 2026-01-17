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

export type DictWord = {
  id: number
  word: string
  phonetic_us: string | null
  phonetic_uk: string | null
  audio_us: string | null
  audio_uk: string | null
  difficulty_level: string | null
  frequency_rank: number | null
  core_level: string | null
  created_at: string
  updated_at: string
}

export type DictWordDefinition = {
  id: number
  word_id: number
  definition_en: string
  definition_zh: string | null
  part_of_speech: string | null
  definition_order: number
  register: string | null
  region: string | null
  context: string | null
  usage_notes: string | null
  is_primary: boolean
  created_at: string
}

export type DictWordExample = {
  id: number
  word_id: number
  example_en: string
  example_zh: string | null
  example_order: number
  created_at: string
}

export type DictSynonym = {
  id: number
  word_id: number
  synonym: string
  created_at: string
}

export type DictAntonym = {
  id: number
  word_id: number
  antonym: string
  created_at: string
}

export type DictWordCollocation = {
  id: number
  word_id: number
  collocation: string
  collocation_type: string | null
  example_en: string | null
  example_zh: string | null
  created_at: string
}

export type DictWordPhrase = {
  id: number
  word_id: number
  phrase: string
  meaning_zh: string | null
  example_en: string | null
  example_zh: string | null
  created_at: string
}

export type DictCommonError = {
  id: number
  word_id: number
  error_type: string
  error_example: string | null
  correct_example: string | null
  explanation: string | null
  created_at: string
}

export type DictWordRoot = {
  id: number
  word_id: number
  root: string
  meaning: string | null
  language: string | null
  created_at: string
}

export type WordQueryResponse = {
  word: DictWord
  definitions: DictWordDefinition[]
  examples: DictWordExample[]
  synonyms: DictSynonym[]
  antonyms: DictAntonym[]
  collocations: DictWordCollocation[]
  phrases: DictWordPhrase[]
  common_errors: DictCommonError[]
  roots: DictWordRoot[]
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
