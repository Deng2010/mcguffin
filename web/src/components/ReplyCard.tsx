import type { ReactNode } from 'react'
import type { DiscussionReply, DiscussionEmoji } from '../types'
import { formatTime } from '../utils/time'
import MarkdownRenderer from './MarkdownRenderer'
import ReactionRow from './ReactionRow'

interface Props {
  reply: DiscussionReply
  emojis: DiscussionEmoji[]
  currentUserId: string | undefined
  isAdmin: boolean
  children?: ReactNode
  onDelete: (id: string) => void
  onReact: (id: string, emoji: string) => void
  onReply: (reply: DiscussionReply) => void
  hideReplyButton?: boolean
}

export default function ReplyCard({
  reply,
  emojis,
  currentUserId,
  isAdmin,
  children,
  onDelete,
  onReact,
  onReply,
  hideReplyButton,
}: Props) {
  return (
    <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-4">
      <div className="flex items-start gap-3">
        {/* Avatar */}
        {reply.author_avatar_url ? (
          <img src={reply.author_avatar_url} className="w-8 h-8 rounded-full object-cover shrink-0" alt="" />
        ) : (
          <div className="w-8 h-8 bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-gray-600 dark:text-gray-300 text-sm font-bold shrink-0">
            {reply.author_name?.charAt(0) || '?'}
          </div>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2 mb-1">
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-200">{reply.author_name}</span>
              {reply.reply_to && (
                <span className="text-xs text-gray-400 dark:text-gray-500">
                  回复 <span className="text-gray-500 dark:text-gray-400">@{reply.reply_to}</span>
                </span>
              )}
              <span className="text-xs text-gray-400 dark:text-gray-500">{formatTime(reply.created_at)}</span>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              {!hideReplyButton && (
                <button
                  onClick={() => onReply(reply)}
                  className="text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                >
                  回复
                </button>
              )}
              {(isAdmin || reply.author_id === currentUserId) && (
                <button
                  onClick={() => onDelete(reply.id)}
                  className="text-xs text-red-400 hover:text-red-500"
                >
                  删除
                </button>
              )}
            </div>
          </div>
          <div className="text-sm text-gray-700 dark:text-gray-300 prose prose-sm max-w-none">
            <MarkdownRenderer content={reply.content} />
          </div>

          {/* Reactions on reply */}
          <ReactionRow
            reactions={reply.reactions}
            emojis={emojis}
            currentUserId={currentUserId}
            onReact={(emoji) => onReact(reply.id, emoji)}
          />

          {/* Children (nested replies) */ }
          {children}
        </div>
      </div>
    </div>
  )
}
