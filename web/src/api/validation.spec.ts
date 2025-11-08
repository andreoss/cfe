import { describe, it, expect } from 'vitest'
import { validateUsername, validateEmail, validatePassword } from './validation'

describe('validateUsername', () => {
  it('accepts a valid username', () => {
    expect(validateUsername('alice_01')).toEqual({ ok: true })
  })

  it('rejects too short', () => {
    expect(validateUsername('ab').ok).toBe(false)
  })

  it('rejects too long', () => {
    expect(validateUsername('a'.repeat(33)).ok).toBe(false)
  })

  it('rejects a leading digit', () => {
    expect(validateUsername('1abc').ok).toBe(false)
  })

  it('rejects invalid characters', () => {
    expect(validateUsername('ali ce').ok).toBe(false)
  })
})

describe('validateEmail', () => {
  it('accepts a valid address', () => {
    expect(validateEmail('alice@example.com')).toEqual({ ok: true })
  })

  it('rejects empty', () => {
    expect(validateEmail('').ok).toBe(false)
  })

  it('rejects missing @', () => {
    expect(validateEmail('alice.example.com').ok).toBe(false)
  })

  it('rejects empty domain', () => {
    expect(validateEmail('alice@').ok).toBe(false)
  })

  it('rejects domain without dot', () => {
    expect(validateEmail('alice@localhost').ok).toBe(false)
  })
})

describe('validatePassword', () => {
  it('accepts 8 or more characters', () => {
    expect(validatePassword('correcthorse')).toEqual({ ok: true })
  })

  it('rejects fewer than 8 characters', () => {
    expect(validatePassword('short').ok).toBe(false)
  })
})
