import type { MentionMember } from '../hooks/useMention'

interface Props {
  /** 是否显示下拉 */
  open: boolean
  /** 筛选后的成员列表 */
  filtered: MentionMember[]
  /** 当前高亮索引 */
  selectedIndex: number
  /** 选中某个成员时的回调 */
  onSelect: (member: MentionMember) => void
  /** 额外 className，用于定位 */
  className?: string
}

export default function MentionDropdown({ open, filtered, selectedIndex, onSelect, className = '' }: Props) {
  if (!open || filtered.length === 0) return null

  return (
    <div
      className={`absolute z-50 w-60 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 shadow-lg max-h-48 overflow-y-auto ${className}`}
    >
      {filtered.map((m, i) => (
        <div
          key={m.user_id}
          className={`px-3 py-2 text-sm cursor-pointer flex items-center gap-2 ${
            i === selectedIndex
              ? 'bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100'
              : 'text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700/50'
          }`}
          onMouseDown={e => e.preventDefault()}
          onClick={() => onSelect(m)}
        >
          {m.avatar_url ? (
            <img src={m.avatar_url} className="w-5 h-5 rounded-full object-cover" alt="" />
          ) : (
            <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-xs font-bold shrink-0">
              {m.name.charAt(0)}
            </span>
          )}
          <span className="font-medium">{m.name}</span>
        </div>
      ))}
    </div>
  )
}
