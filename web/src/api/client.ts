export type User = { id: string; username: string }
export type Profile = { id: string; username: string; bio: string | null }
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
