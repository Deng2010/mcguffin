import { useState, useEffect } from 'react'

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
 * A styled span that shows difficulty label with its configured color.
 * Uses inline style so any hex color works (not just Tailwind classes).
 */
export function DiffBadge({ difficulty, map, className = '' }: {
  difficulty: string
  map: Map<string, DifficultyInfo>
  className?: string
}) {
  const info = map.get(difficulty)
  const color = info?.color || '#888888'
  const label = info?.label || difficulty
  return (
    <span
      className={className}
      style={{ color, backgroundColor: color + '18' }}
    >
      {label}
    </span>
  )
}
