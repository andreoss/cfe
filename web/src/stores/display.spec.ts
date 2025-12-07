import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import {
  applyDensity,
  applyTheme,
  toDensity,
  toTheme,
  toTimeStyle,
  useDisplayStore,
} from './display'

describe('display preferences', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-density')
  })

  it('reads back every choice it offers', () => {
    expect(toTheme('system')).toBe('system')
    expect(toTheme('light')).toBe('light')
    expect(toTheme('dark')).toBe('dark')
    expect(toTimeStyle('relative')).toBe('relative')
    expect(toTimeStyle('exact')).toBe('exact')
    expect(toDensity('comfortable')).toBe('comfortable')
    expect(toDensity('compact')).toBe('compact')
  })

  it('falls back to the ordinary look when asked for something unknown', () => {
    expect(toTheme('neon')).toBe('system')
    expect(toTheme(null)).toBe('system')
    expect(toTimeStyle('sometime')).toBe('relative')
    expect(toDensity('roomy')).toBe('comfortable')
  })

  it('leaves the page alone when the reader follows their system', () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    applyTheme('system')
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('states an explicit choice on the page itself', () => {
    applyTheme('dark')
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    applyTheme('light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    applyDensity('compact')
    expect(document.documentElement.getAttribute('data-density')).toBe('compact')
  })

  it('starts from what the browser was told last time', () => {
    window.localStorage.setItem('display.theme', 'dark')
    window.localStorage.setItem('display.time', 'exact')
    window.localStorage.setItem('display.density', 'compact')
    const store = useDisplayStore()
    expect(store.theme).toBe('dark')
    expect(store.timeStyle).toBe('exact')
    expect(store.density).toBe('compact')
  })

  it('ignores a remembered value that is no longer offered', () => {
    window.localStorage.setItem('display.theme', 'neon')
    const store = useDisplayStore()
    expect(store.theme).toBe('system')
  })

  it('keeps a change for next time', async () => {
    const store = useDisplayStore()
    store.theme = 'dark'
    store.density = 'compact'
    await Promise.resolve()
    expect(window.localStorage.getItem('display.theme')).toBe('dark')
    expect(window.localStorage.getItem('display.density')).toBe('compact')
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})
