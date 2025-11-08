import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

vi.mock('@/api/client', () => ({
  register: vi.fn(),
  signIn: vi.fn(),
  signOut: vi.fn(),
  me: vi.fn(),
}))

import { register, signIn, me, signOut } from '@/api/client'
import { useAuthStore } from './auth'

describe('useAuthStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.mocked(register).mockReset()
    vi.mocked(signIn).mockReset()
    vi.mocked(me).mockReset()
    vi.mocked(signOut).mockReset()
  })

  it('doRegister sets the current user on success', async () => {
    vi.mocked(register).mockResolvedValue({ ok: true, value: { id: '1', username: 'alice_01' } })
    const auth = useAuthStore()
    const result = await auth.doRegister('alice_01', 'alice@example.com', 'correcthorse')
    expect(result.ok).toBe(true)
    expect(auth.currentUser).toEqual({ id: '1', username: 'alice_01' })
  })

  it('doRegister leaves the current user unset on failure', async () => {
    vi.mocked(register).mockResolvedValue({ ok: false, error: 'username taken' })
    const auth = useAuthStore()
    const result = await auth.doRegister('alice_01', 'alice@example.com', 'correcthorse')
    expect(result.ok).toBe(false)
    expect(auth.currentUser).toBeNull()
  })

  it('doSignIn sets the current user on success', async () => {
    vi.mocked(signIn).mockResolvedValue({ ok: true, value: { id: '1', username: 'alice_01' } })
    const auth = useAuthStore()
    const result = await auth.doSignIn('alice_01', 'correcthorse')
    expect(result.ok).toBe(true)
    expect(auth.currentUser).toEqual({ id: '1', username: 'alice_01' })
  })

  it('doSignIn leaves the current user unset on failure', async () => {
    vi.mocked(signIn).mockResolvedValue({ ok: false, error: 'invalid credentials' })
    const auth = useAuthStore()
    const result = await auth.doSignIn('alice_01', 'wrong')
    expect(result.ok).toBe(false)
    expect(auth.currentUser).toBeNull()
  })

  it('checkSession sets the current user when a session exists', async () => {
    vi.mocked(me).mockResolvedValue({ ok: true, value: { id: '1', username: 'alice_01' } })
    const auth = useAuthStore()
    await auth.checkSession()
    expect(auth.currentUser).toEqual({ id: '1', username: 'alice_01' })
    expect(auth.checked).toBe(true)
  })

  it('checkSession clears the current user when there is no session', async () => {
    vi.mocked(me).mockResolvedValue({ ok: false, error: 'missing session' })
    const auth = useAuthStore()
    await auth.checkSession()
    expect(auth.currentUser).toBeNull()
    expect(auth.checked).toBe(true)
  })

  it('doSignOut clears the current user', async () => {
    vi.mocked(me).mockResolvedValue({ ok: true, value: { id: '1', username: 'alice_01' } })
    vi.mocked(signOut).mockResolvedValue({ ok: true, value: undefined })
    const auth = useAuthStore()
    await auth.checkSession()
    await auth.doSignOut()
    expect(auth.currentUser).toBeNull()
    expect(signOut).toHaveBeenCalled()
  })
})
