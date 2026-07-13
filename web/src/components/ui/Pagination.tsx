interface PaginationProps {
  currentPage: number
  totalPages: number
  onPageChange: (page: number) => void
  mode?: 'full' | 'simple'
  size?: 'sm' | 'xs'
  className?: string
}

export default function Pagination({
  currentPage,
  totalPages,
  onPageChange,
  mode = 'full',
  size = 'sm',
  className = '',
}: PaginationProps) {
  if (totalPages <= 1) return null

  const px = size === 'xs' ? 'px-3 py-1 text-xs' : 'px-3 py-1.5 text-sm'
  const btnBase = `${px} border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400`
  const hoverClass = size === 'xs'
    ? 'hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed'
    : 'hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-40 disabled:cursor-not-allowed'

  if (mode === 'simple') {
    return (
      <div className={`flex items-center justify-center gap-2 mt-4 ${className}`}>
        <button
          disabled={currentPage <= 1}
          onClick={() => onPageChange(Math.max(1, currentPage - 1))}
          className={`${btnBase} ${hoverClass}`}
        >
          上一页
        </button>
        <span className="text-xs text-gray-400 dark:text-gray-500 px-2">
          {currentPage} / {totalPages}
        </span>
        <button
          disabled={currentPage >= totalPages}
          onClick={() => onPageChange(Math.min(totalPages, currentPage + 1))}
          className={`${btnBase} ${hoverClass}`}
        >
          下一页
        </button>
      </div>
    )
  }

  // Full mode: numbered pages with ellipsis window ±2
  const pages = Array.from({ length: totalPages }, (_, i) => i + 1)
    .filter((p) => p === 1 || p === totalPages || Math.abs(p - currentPage) <= 2)

  return (
    <div className={`flex items-center justify-center gap-2 mt-8 ${className}`}>
      <button
        onClick={() => onPageChange(currentPage - 1)}
        disabled={currentPage <= 1}
        className={`${btnBase} ${hoverClass}`}
      >
        上一页
      </button>
      {pages.map((p, idx, arr) => (
        <span key={p} className="flex items-center">
          {idx > 0 && arr[idx - 1] !== p - 1 && (
            <span className="px-1 text-gray-400 dark:text-gray-600">…</span>
          )}
          <button
            onClick={() => onPageChange(p)}
            className={`px-3 py-1.5 text-sm border ${
              p === currentPage
                ? 'border-gray-800 dark:border-gray-100 bg-gray-800 dark:bg-gray-700 text-white'
                : 'border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
            }`}
          >
            {p}
          </button>
        </span>
      ))}
      <button
        onClick={() => onPageChange(currentPage + 1)}
        disabled={currentPage >= totalPages}
        className={`${btnBase} ${hoverClass}`}
      >
        下一页
      </button>
    </div>
  )
}
