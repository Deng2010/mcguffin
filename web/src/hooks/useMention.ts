import { useState, useMemo, useCallback } from 'react'

// ============== Types ==============

export interface MentionMember {
  user_id: string
  name: string
  avatar_url: string | null
  username: string
  role: string
}

interface MentionState {
  open: boolean
  query: string
  index: number
  /** Character position in the text where '@' was found */
  atPos: number
}

const ROLE_LABELS: Record<string, string> = {
  superadmin: '系统管理员',
  admin: '管理员',
  member: '成员',
}

// ============== Hook ==============

/**
 * Hook for @mention autocomplete.
 *
 * Usage:
 *   const mention = useMention(members)
 *   // In onChange handler: mention.handleTextChange(newText, cursorPos)
 *   // In onKeyDown handler: if (mention.handleKeyDown(e)) return
 *   // On submit: mention.getMentionedUserIds(content)
 *   // Render: {mention.open && <MentionPopup ... />}
 */
export function useMention(members: MentionMember[]) {
  const [state, setState] = useState<MentionState>({
    open: false,
    query: '',
    index: 0,
    atPos: -1,
  })

  // Build name→user_id lookups
  const nameToId = useMemo(() => {
    const map = new Map<string, string>()
    for (const m of members) {
      map.set(m.name, m.user_id)
    }
    return map
  }, [members])

  // Filtered members based on query
  const filtered = useMemo(() => {
    if (!state.open) return []
    if (!state.query) return members
    const q = state.query.toLowerCase()
    return members.filter(m =>
      m.name.toLowerCase().includes(q) || m.username.toLowerCase().includes(q)
    )
  }, [members, state.open, state.query])

  // Detect @ and extract query from text at cursor position
  const handleTextChange = useCallback((text: string, cursorPos: number) => {
    const beforeCursor = text.slice(0, cursorPos)
    const atIdx = beforeCursor.lastIndexOf('@')

    if (atIdx >= 0) {
      // Check that '@' is at word boundary (preceded by space or start of string)
      const prec = beforeCursor[atIdx - 1]
      if (!prec || /\s/.test(prec)) {
        const query = beforeCursor.slice(atIdx + 1)
        // Only keep open if query has no spaces (mention ends at space)
        if (!/\s/.test(query)) {
          setState({ open: true, query, index: 0, atPos: atIdx })
          return
        }
      }
    }

    setState({ open: false, query: '', index: 0, atPos: -1 })
  }, [])

  // Handle keyboard events for the input/textarea
  // Returns true if the event was consumed (don't propagate)
  const handleKeyDown = useCallback((e: React.KeyboardEvent): boolean => {
    if (!state.open || filtered.length === 0) return false

    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setState(s => ({ ...s, index: Math.min(s.index + 1, filtered.length - 1) }))
      return true
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      setState(s => ({ ...s, index: Math.max(s.index - 1, 0) }))
      return true
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      setState(s => ({ ...s, open: false }))
      return true
    }
    return false
  }, [state.open, state.index, filtered.length])

  // Insert a mention into text at the given cursor position
  const insertMention = useCallback((text: string, _cursorPos: number, member: MentionMember): string => {
    const beforeAt = text.slice(0, state.atPos)
    const afterCursor = text.slice(state.atPos)
    // Find the end of the current query
    let endIdx = state.atPos + 1
    while (endIdx < text.length && !/\s/.test(text[endIdx])) {
      endIdx++
    }
    const afterQuery = text.slice(endIdx)
    const newText = beforeAt + '@' + member.name + ' ' + afterQuery
    setState({ open: false, query: '', index: 0, atPos: -1 })
    return newText
  }, [state.atPos])

  // Insert the currently highlighted mention
  const insertSelected = useCallback((text: string, cursorPos: number): [string, boolean] => {
    if (!state.open || filtered.length === 0) return [text, false]
    const member = filtered[state.index]
    if (!member) return [text, false]
    return [insertMention(text, cursorPos, member), true]
  }, [state.open, filtered, state.index, insertMention])

  // Parse content for @display_name patterns, return unique user_ids
  const getMentionedUserIds = useCallback((text: string): string[] => {
    const ids: string[] = []
    const seen = new Set<string>()
    for (const [name, userId] of nameToId) {
      if (seen.has(userId)) continue
      const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const regex = new RegExp(`(?:^|[\\s，。、！？；：])@${escapedName}(?=[\\s，。、！？；：]|$)`)
      if (regex.test(text)) {
        ids.push(userId)
        seen.add(userId)
      }
    }
    return ids
  }, [nameToId])

  return {
    open: state.open,
    query: state.query,
    selectedIndex: state.index,
    filtered,
    handleTextChange,
    handleKeyDown,
    insertMention,
    insertSelected,
    getMentionedUserIds,
    close: () => setState({ open: false, query: '', index: 0, atPos: -1 }),
  }
}
