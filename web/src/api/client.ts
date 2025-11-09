export type User = { id: string; username: string }
export type Profile = { id: string; username: string; bio: string | null }
export type Section = { slug: string; title: string }
export type Topic = {
  id: string
  sectionSlug: string
  title: string
  body: string
  tags: string[]
  authorUsername: string
  createdAt: string
}
export type ApiResult<T> = { ok: true; value: T } | { ok: false; error: string }

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''

async function request<T>(path: string, init: RequestInit): Promise<ApiResult<T>> {
  const response = await fetch(`${BASE_URL}${path}`, {
    credentials: 'include',
    ...init,
  })
  const text = await response.text()
  const data = text.length > 0 ? JSON.parse(text) : undefined
  if (!response.ok) {
    return { ok: false, error: data?.error ?? 'request failed' }
  }
  return { ok: true, value: data as T }
}

function post<T>(path: string, body: unknown): Promise<ApiResult<T>> {
  return request<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
}

export function register(
  username: string,
  email: string,
  password: string,
): Promise<ApiResult<User>> {
  return post<User>('/api/register', { username, email, password })
}

export function signIn(username: string, password: string): Promise<ApiResult<User>> {
  return post<User>('/api/sign-in', { username, password })
}

export function me(): Promise<ApiResult<User>> {
  return request<User>('/api/me', { method: 'GET' })
}

export function signOut(): Promise<ApiResult<void>> {
  return request<void>('/api/sign-out', { method: 'POST' })
}

export function getProfile(username: string): Promise<ApiResult<Profile>> {
  return request<Profile>(`/api/users/${encodeURIComponent(username)}`, { method: 'GET' })
}

export function updateBio(bio: string): Promise<ApiResult<Profile>> {
  return request<Profile>('/api/me/bio', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ bio: bio.length > 0 ? bio : null }),
  })
}

type RawTopic = {
  id: string
  section_slug: string
  title: string
  body: string
  tags: string[]
  author_username: string
  created_at: string
}

function toTopic(raw: RawTopic): Topic {
  return {
    id: raw.id,
    sectionSlug: raw.section_slug,
    title: raw.title,
    body: raw.body,
    tags: raw.tags,
    authorUsername: raw.author_username,
    createdAt: raw.created_at,
  }
}

export function getSections(): Promise<ApiResult<Section[]>> {
  return request<Section[]>('/api/sections', { method: 'GET' })
}

export async function getTopics(slug: string): Promise<ApiResult<Topic[]>> {
  const result = await request<RawTopic[]>(`/api/sections/${encodeURIComponent(slug)}/topics`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: result.value.map(toTopic) } : result
}

export async function createTopic(
  slug: string,
  title: string,
  body: string,
  tags: string[],
): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/sections/${encodeURIComponent(slug)}/topics`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, body, tags }),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function getTopic(id: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function getTopicsByTag(tag: string): Promise<ApiResult<Topic[]>> {
  const result = await request<RawTopic[]>(`/api/tags/${encodeURIComponent(tag)}/topics`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: result.value.map(toTopic) } : result
}
