import { Link } from 'react-router-dom'
import { formatTime } from '../utils/time'
import type { Notification } from '../types'

interface Props {
  notifications: Notification[]
  unreadCount: number
  open: boolean
  onClose: () => void
  onMarkRead: (id: string) => void
  onMarkAllRead: () => void
}

export default function NotificationDropdown({ notifications, unreadCount, open, onClose, onMarkRead, onMarkAllRead }: Props) {
  if (!open) return null

  return (
    <div className="absolute right-0 top-full mt-2 w-80 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 shadow-lg z-50 max-h-96 flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200 dark:border-gray-700">
        <span className="text-sm font-medium text-gray-800 dark:text-gray-200">通知</span>
        {unreadCount > 0 && (
          <button
            onClick={onMarkAllRead}
            className="text-xs text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
          >
            全部已读
          </button>
        )}
      </div>
      <div className="overflow-y-auto flex-1">
        {notifications.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-gray-400 dark:text-gray-500">
            暂无通知
          </div>
        ) : (
          notifications.map(n => (
            <div
              key={n.id}
              className={`px-4 py-3 border-b border-gray-100 dark:border-gray-700/50 cursor-pointer transition-colors ${
                n.read
                  ? 'hover:bg-gray-50 dark:hover:bg-gray-700/30'
                  : 'bg-gray-50 dark:bg-gray-700/20 hover:bg-gray-100 dark:hover:bg-gray-700/40'
              }`}
              onClick={() => {
                onMarkRead(n.id)
                onClose()
              }}
            >
              <div className="flex items-start gap-2">
                {!n.read && (
                  <span className="w-2 h-2 bg-red-500 rounded-full mt-1.5 shrink-0" />
                )}
                <div className={`flex-1 min-w-0 ${n.read ? 'ml-4' : ''}`}>
                  <Link
                    to={n.link || '#'}
                    className="text-sm font-medium text-gray-800 dark:text-gray-200 hover:underline block"
                    onClick={() => onClose()}
                  >
                    {n.title}
                  </Link>
                  <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">{n.body}</p>
                  <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">{formatTime(n.created_at)}</p>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
