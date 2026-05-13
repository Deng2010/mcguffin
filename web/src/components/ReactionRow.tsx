import { useState } from 'react'
import type { DiscussionEmoji } from '../types'

interface Props {
  reactions: Record<string, string[]>
  emojis: DiscussionEmoji[]
  currentUserId: string | undefined
  onReact: (emoji: string) => void
}

export default function ReactionRow({ reactions, emojis, currentUserId, onReact }: Props) {
  if (emojis.length === 0) return null

  const [open, setOpen] = useState(false)

  const totalReactions = Object.values(reactions).reduce((sum, users) => sum + (users?.length || 0), 0)

  return (
    <div className="mt-2 ml-0">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="inline-flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
      >
        <svg
          className={`w-3 h-3 transition-transform ${open ? 'rotate-90' : ''}`}
          fill="none" stroke="currentColor" viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
        </svg>
        反应 {totalReactions > 0 && `(${totalReactions})`}
      </button>
      {open && (
        <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
          {emojis.map(e => {
            const users = reactions[e.char] || []
            const count = users.length
            const active = currentUserId ? users.includes(currentUserId) : false
            return (
              <button
                key={e.id}
                onClick={() => onReact(e.char)}
                className={`inline-flex items-center gap-1 px-2 py-0.5 text-xs border transition-colors ${
                  active
                    ? 'border-gray-500 dark:border-gray-400 bg-gray-100 dark:bg-gray-800'
                    : 'border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'
                }`}
                title={count > 0 ? `${count} 人` : e.name}
              >
                <span>{e.char}</span>
                {count > 0 && <span className="text-gray-500 dark:text-gray-400">{count}</span>}
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
