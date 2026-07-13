import { create } from 'zustand'

// ============== State & Actions ==============

interface ThemeState {
  isDark: boolean
  toggle: () => void
  setDark: (dark: boolean) => void
}

function getInitialDark(): boolean {
  const stored = localStorage.getItem('mcguffin-dark-mode')
  if (stored !== null) return stored === 'true'
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

function applyDark(isDark: boolean) {
  if (isDark) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
  localStorage.setItem('mcguffin-dark-mode', String(isDark))
}

export const useThemeStore = create<ThemeState>()((set) => {
  const initialDark = getInitialDark()
  applyDark(initialDark)

  // Listen for system preference changes (only when no explicit user choice)
  const mq = window.matchMedia('(prefers-color-scheme: dark)')
  const handler = (e: MediaQueryListEvent) => {
    const stored = localStorage.getItem('mcguffin-dark-mode')
    if (stored === null) {
      applyDark(e.matches)
      set({ isDark: e.matches })
    }
  }
  mq.addEventListener('change', handler)

  return {
    isDark: initialDark,
    toggle: () => set(state => {
      const newDark = !state.isDark
      applyDark(newDark)
      return { isDark: newDark }
    }),
    setDark: (dark: boolean) => {
      applyDark(dark)
      set({ isDark: dark })
    },
  }
})
