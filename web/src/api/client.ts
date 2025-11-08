export type User = { id: string; username: string }
export type ApiResult<T> = { ok: true; value: T } | { ok: false; error: string }

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''

async function post<T>(path: string, body: unknown): Promise<ApiResult<T>> {
  const response = await fetch(`${BASE_URL}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  const data = await response.json()
  if (!response.ok) {
    return { ok: false, error: data.error ?? 'request failed' }
  }
  return { ok: true, value: data as T }
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
