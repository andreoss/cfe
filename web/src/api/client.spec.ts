import { describe, it, expect, vi, beforeEach } from 'vitest'
import { register, signIn, me, signOut, getProfile, updateBio } from './client'

function jsonResponse(ok: boolean, body: unknown) {
  return { ok, text: async () => JSON.stringify(body) } as Response
}

function emptyResponse(ok: boolean) {
  return { ok, text: async () => '' } as Response
}

describe('register', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the created user on success', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns the server error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'username taken' }))
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: false, error: 'username taken' })
  })

  it('sends credentials so the session cookie is stored', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })
})

describe('signIn', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the user on success', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    const result = await signIn('alice_01', 'correcthorse')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns invalid-credentials error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid credentials' }))
    const result = await signIn('alice_01', 'wrong')
    expect(result).toEqual({ ok: false, error: 'invalid credentials' })
  })
})

describe('me', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the current user when a session is active', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    const result = await me()
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns an error when there is no session', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await me()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('signOut', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('succeeds on an empty response body', async () => {
    vi.mocked(fetch).mockResolvedValue(emptyResponse(true))
    const result = await signOut()
    expect(result).toEqual({ ok: true, value: undefined })
  })
})

describe('getProfile', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the profile on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: 'hello' }),
    )
    const result = await getProfile('alice_01')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01', bio: 'hello' } })
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'a b', bio: null }))
    await getProfile('a b')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/a%20b'),
      expect.anything(),
    )
  })

  it('returns an error for an unknown user', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'user not found' }))
    const result = await getProfile('ghost')
    expect(result).toEqual({ ok: false, error: 'user not found' })
  })
})

describe('updateBio', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the updated profile on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: 'new bio' }),
    )
    const result = await updateBio('new bio')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01', bio: 'new bio' } })
  })

  it('sends null for an empty bio', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01', bio: null }))
    await updateBio('')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ body: JSON.stringify({ bio: null }) }),
    )
  })
})
