import { describe, it, expect, vi, beforeEach } from 'vitest'
import { register, signIn } from './client'

describe('register', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the created user on success', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      json: async () => ({ id: '1', username: 'alice_01' }),
    } as Response)
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns the server error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: false,
      json: async () => ({ error: 'username taken' }),
    } as Response)
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: false, error: 'username taken' })
  })
})

describe('signIn', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the user on success', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      json: async () => ({ id: '1', username: 'alice_01' }),
    } as Response)
    const result = await signIn('alice_01', 'correcthorse')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns invalid-credentials error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: false,
      json: async () => ({ error: 'invalid credentials' }),
    } as Response)
    const result = await signIn('alice_01', 'wrong')
    expect(result).toEqual({ ok: false, error: 'invalid credentials' })
  })
})
