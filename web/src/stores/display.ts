import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export const THEMES = ['system', 'light', 'dark'] as const
export const TIME_STYLES = ['relative', 'exact'] as const
export const DENSITIES = ['comfortable', 'compact'] as const

export type Theme = (typeof THEMES)[number]
export type TimeStyle = (typeof TIME_STYLES)[number]
export type Density = (typeof DENSITIES)[number]

const THEME_KEY = 'display.theme'
const TIME_KEY = 'display.time'
const DENSITY_KEY = 'display.density'

export function toTheme(raw: unknown): Theme {
  return THEMES.includes(raw as Theme) ? (raw as Theme) : 'system'
}

export function toTimeStyle(raw: unknown): TimeStyle {
  return TIME_STYLES.includes(raw as TimeStyle) ? (raw as TimeStyle) : 'relative'
}

export function toDensity(raw: unknown): Density {
  return DENSITIES.includes(raw as Density) ? (raw as Density) : 'comfortable'
}

function remembered(key: string): string | null {
  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}

function remember(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value)
  } catch {
    return
  }
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement
  if (theme === 'system') root.removeAttribute('data-theme')
  else root.setAttribute('data-theme', theme)
}

export function applyDensity(density: Density) {
  document.documentElement.setAttribute('data-density', density)
}

export const useDisplayStore = defineStore('display', () => {
  const theme = ref<Theme>(toTheme(remembered(THEME_KEY)))
  const timeStyle = ref<TimeStyle>(toTimeStyle(remembered(TIME_KEY)))
  const density = ref<Density>(toDensity(remembered(DENSITY_KEY)))

  function apply() {
    applyTheme(theme.value)
    applyDensity(density.value)
  }

  watch(theme, (value) => {
    remember(THEME_KEY, value)
    applyTheme(value)
  })

  watch(timeStyle, (value) => {
    remember(TIME_KEY, value)
  })

  watch(density, (value) => {
    remember(DENSITY_KEY, value)
    applyDensity(value)
  })

  return { theme, timeStyle, density, apply }
})
