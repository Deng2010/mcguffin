interface LoadingSpinnerProps {
  text?: string
  size?: 'sm' | 'md'
  fullPage?: boolean
  className?: string
}

export default function LoadingSpinner({
  text = '加载中...',
  size = 'md',
  fullPage = false,
  className = '',
}: LoadingSpinnerProps) {
  const py = size === 'sm' ? 'py-8' : 'py-12'
  const padding = fullPage ? 'p-6' : ''

  return (
    <div className={`${padding} text-center text-gray-400 dark:text-gray-500 ${py} ${className}`}>
      {text}
    </div>
  )
}
