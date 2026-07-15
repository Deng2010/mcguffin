import { useState, useRef, useEffect } from 'react'
import { useThemeStore } from '../../stores/themeStore'

const options: { value: 'small' | 'default' | 'large'; label: string }[] = [
  { value: 'small', label: '小' },
  { value: 'default', label: '中' },
  { value: 'large', label: '大' },
]

interface FontSizeToggleProps {
  className?: string
}

export default function FontSizeToggle({ className = '' }: FontSizeToggleProps) {
  const { fontSize, setFontSize } = useThemeStore()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const currentLabel = options.find(o => o.value === fontSize)?.label || '中'

  return (
    <div ref={ref} className={`relative ${className}`}>
      <button
        onClick={() => setOpen(!open)}
        className="p-1.5 text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        title="字体大小"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7V4h16v3M9 20h6M12 4v16" />
        </svg>
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-1 z-50 min-w-[100px] bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow-lg dark:shadow-xl dark:shadow-black/50">
          <div className="px-3 py-2 text-xs text-gray-400 dark:text-gray-500 border-b border-gray-200 dark:border-gray-700">
            字体大小
          </div>
          {options.map(opt => (
            <button
              key={opt.value}
              onClick={() => {
                setFontSize(opt.value)
                setOpen(false)
              }}
              className={`w-full text-left px-3 py-2 text-sm transition-colors ${
                fontSize === opt.value
                  ? 'text-gray-900 dark:text-gray-100 bg-gray-100 dark:bg-gray-800 font-medium'
                  : 'text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800/50'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
