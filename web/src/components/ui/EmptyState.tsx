interface EmptyStateProps {
  message: string
  description?: string
  padding?: 'sm' | 'lg'
  bordered?: boolean
  className?: string
}

export default function EmptyState({
  message,
  description,
  padding = 'lg',
  bordered = false,
  className = '',
}: EmptyStateProps) {
  const py = padding === 'sm' ? 'py-8' : 'py-12'
  const border = bordered
    ? 'border border-dashed border-gray-300 dark:border-gray-700'
    : ''

  return (
    <div className={`text-center ${py} text-gray-400 dark:text-gray-500 ${border} ${className}`}>
      <p>{message}</p>
      {description && (
        <p className="text-sm mt-1">{description}</p>
      )}
    </div>
  )
}
