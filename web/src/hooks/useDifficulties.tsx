import { useState, useEffect } from 'react'
import { useDarkMode } from '../DarkModeContext'

export interface DifficultyInfo {
  name: string
  label: string
  color: string
}

/**
 * Hook to fetch and cache difficulty levels from the server.
 * Returns a Map<name, DifficultyInfo> and the levels array.
 */
export function useDifficulties(): {
  difficultyMap: Map<string, DifficultyInfo>
  difficulties: DifficultyInfo[]
  loading: boolean
} {
  const [difficulties, setDifficulties] = useState<DifficultyInfo[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    const fetchData = async () => {
      try {
        const res = await fetch('/api/site/difficulties')
        const data = await res.json()
        if (!cancelled && data?.levels) {
          setDifficulties(data.levels)
        }
      } catch {
        // ignore
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    fetchData()
    return () => { cancelled = true }
  }, [])

  const difficultyMap = new Map(difficulties.map(d => [d.name, d]))

  return { difficultyMap, difficulties, loading }
}

/** Get label text from difficulty name */
export function diffLabel(d: string, map: Map<string, DifficultyInfo>): string {
  return map.get(d)?.label || d
}

/** Get color hex from difficulty name */
export function diffColor(d: string, map: Map<string, DifficultyInfo>): string {
  return map.get(d)?.color || '#888888'
}

/**
 * Lighten a hex color if it's too dark for dark mode.
 * Blends toward white by `amount` (0-1) when perceived luminance < 128.
 */
function lightenIfDark(hex: string, darkMode: boolean, amount: number = 0.45): string {
  if (!darkMode) return hex
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  const luminance = 0.299 * r + 0.587 * g + 0.114 * b
  if (luminance < 128) {
    const nr = Math.round(r + (255 - r) * amount)
    const ng = Math.round(g + (255 - g) * amount)
    const nb = Math.round(b + (255 - b) * amount)
    return `#${nr.toString(16).padStart(2, '0')}${ng.toString(16).padStart(2, '0')}${nb.toString(16).padStart(2, '0')}`
  }
  return hex
}

/**
 * A styled span that shows difficulty label with its configured color.
 * Uses inline style so any hex color works (not just Tailwind classes).
 * Automatically lightens dark colors (like Black) in dark mode.
 */
export function DiffBadge({ difficulty, map, className = '' }: {
  difficulty: string
  map: Map<string, DifficultyInfo>
  className?: string
}) {
  const { isDark } = useDarkMode()
  const info = map.get(difficulty)
  const rawColor = info?.color || '#888888'
  const label = info?.label || difficulty
  const color = lightenIfDark(rawColor, isDark)
  return (
    <span
      className={className}
      style={{ color, backgroundColor: color + '18' }}
    >
      {label}
    </span>
  )
}
