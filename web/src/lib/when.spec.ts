import { describe, expect, it } from 'vitest'
import { exactWhen, readableWhen } from './when'

const now = new Date('2024-06-08T12:00:00Z')

function ago(seconds: number) {
  return new Date(now.getTime() - seconds * 1000).toISOString()
}

describe('readableWhen', () => {
  it('reads a moment ago as just now', () => {
    expect(readableWhen(ago(0), now)).toBe('just now')
    expect(readableWhen(ago(59), now)).toBe('just now')
  })

  it('counts minutes, hours and days', () => {
    expect(readableWhen(ago(60), now)).toBe('1 minute ago')
    expect(readableWhen(ago(60 * 5), now)).toBe('5 minutes ago')
    expect(readableWhen(ago(3600), now)).toBe('1 hour ago')
    expect(readableWhen(ago(3600 * 5), now)).toBe('5 hours ago')
    expect(readableWhen(ago(86400), now)).toBe('1 day ago')
    expect(readableWhen(ago(86400 * 5), now)).toBe('5 days ago')
  })

  it('counts months and years once it is that long', () => {
    expect(readableWhen(ago(86400 * 30), now)).toBe('1 month ago')
    expect(readableWhen(ago(86400 * 90), now)).toBe('3 months ago')
    expect(readableWhen(ago(86400 * 365), now)).toBe('1 year ago')
    expect(readableWhen(ago(86400 * 730), now)).toBe('2 years ago')
  })

  it('never says a thing was written in the future', () => {
    const later = new Date(now.getTime() + 60_000).toISOString()
    expect(readableWhen(later, now)).toBe('just now')
  })

  it('hands back anything it cannot read', () => {
    expect(readableWhen('not a time', now)).toBe('not a time')
  })
})

describe('exactWhen', () => {
  it('reads as a plain date and time', () => {
    expect(exactWhen('2024-06-08T12:34:56Z')).toBe('2024-06-08 12:34')
  })

  it('hands back anything it cannot read', () => {
    expect(exactWhen('not a time')).toBe('not a time')
  })
})
