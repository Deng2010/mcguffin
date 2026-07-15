import { create } from 'zustand'

// ============== Types ==============

type FontSize = 'small' | 'default' | 'large'

interface ThemeState {
  isDark: boolean
  fontSize: FontSize
  toggle: () => void
  setDark: (dark: boolean) => void
  setFontSize: (size: FontSize) => void
}

// ============== Helpers ==============

const DARK_KEY = 'mcguffin-dark-mode'
const FONT_KEY = 'mcguffin-font-size'

function getInitialDark(): boolean {
  const stored = localStorage.getItem(DARK_KEY)
  if (stored !== null) return stored === 'true'
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

function getInitialFontSize(): FontSize {
  const stored = localStorage.getItem(FONT_KEY) as FontSize | null
  if (stored && ['small', 'default', 'large'].includes(stored)) return stored
  return 'default'
}

function applyDark(isDark: boolean) {
  document.documentElement.classList.toggle('dark', isDark)
  localStorage.setItem(DARK_KEY, String(isDark))
}

function applyFontSize(size: FontSize) {
  document.documentElement.setAttribute('data-font-size', size)
  localStorage.setItem(FONT_KEY, size)
}

// ============== Store ==============

export const useThemeStore = create<ThemeState>()((set) => {
  const initialDark = getInitialDark()
  applyDark(initialDark)
  applyFontSize(getInitialFontSize())

  // Listen for system preference changes (only when no explicit user choice)
  const mq = window.matchMedia('(prefers-color-scheme: dark)')
  const handler = (e: MediaQueryListEvent) => {
    const stored = localStorage.getItem(DARK_KEY)
    if (stored === null) {
      applyDark(e.matches)
      set({ isDark: e.matches })
    }
  }
  mq.addEventListener('change', handler)

  return {
    isDark: initialDark,
    fontSize: getInitialFontSize(),
    toggle: () => set(state => {
      const newDark = !state.isDark
      applyDark(newDark)
      return { isDark: newDark }
    }),
    setDark: (dark: boolean) => {
      applyDark(dark)
      set({ isDark: dark })
    },
    setFontSize: (size: FontSize) => {
      applyFontSize(size)
      set({ fontSize: size })
    },
  }
})
