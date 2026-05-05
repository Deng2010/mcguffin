import { useState, useEffect, useContext, createContext, useCallback, type ReactNode } from 'react'

// ============== Context Type ==============

interface DarkModeContextType {
  isDark: boolean
  toggle: () => void
  setDark: (dark: boolean) => void
}

const DarkModeContext = createContext<DarkModeContextType | null>(null)

// ============== Hook ==============

export function useDarkMode(): DarkModeContextType {
  const ctx = useContext(DarkModeContext)
  if (!ctx) throw new Error('useDarkMode must be used within DarkModeProvider')
  return ctx
}

// ============== Provider ==============

export function DarkModeProvider({ children }: { children: ReactNode }) {
  const [isDark, setIsDark] = useState<boolean>(() => {
    // 1. Check localStorage
    const stored = localStorage.getItem('mcguffin-dark-mode')
    if (stored !== null) return stored === 'true'
    // 2. Fall back to system preference
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  })

  // Sync the `dark` class on <html>
  useEffect(() => {
    if (isDark) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
    localStorage.setItem('mcguffin-dark-mode', String(isDark))
  }, [isDark])

  // Listen for system preference changes (only when no explicit user choice)
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const handler = (e: MediaQueryListEvent) => {
      const stored = localStorage.getItem('mcguffin-dark-mode')
      if (stored === null) {
        setIsDark(e.matches)
      }
    }
    mq.addEventListener('change', handler)
    return () => mq.removeEventListener('change', handler)
  }, [])

  const toggle = useCallback(() => setIsDark(prev => !prev), [])
  const setDark = useCallback((dark: boolean) => setIsDark(dark), [])

  return (
    <DarkModeContext.Provider value={{ isDark, toggle, setDark }}>
      {children}
    </DarkModeContext.Provider>
  )
}
