import { useState } from 'react'

interface AvatarProps {
  src?: string | null
  name: string
  size?: 'xs' | 'sm' | 'md' | 'base' | 'lg'
  className?: string
}

const sizeMap = {
  xs: { dim: 'w-5 h-5', text: 'text-[10px]', font: 'font-bold' },
  sm: { dim: 'w-7 h-7', text: 'text-xs', font: 'font-bold' },
  md: { dim: 'w-8 h-8', text: 'text-sm', font: 'font-bold' },
  base: { dim: 'w-10 h-10', text: 'text-sm', font: 'font-bold' },
  lg: { dim: 'w-24 h-24', text: 'text-3xl', font: 'font-bold' },
} as const

export default function Avatar({ src, name, size = 'base', className = '' }: AvatarProps) {
  const [imgError, setImgError] = useState(false)
  const s = sizeMap[size]
  const initial = name.charAt(0) || '?'
  const showImg = src && !imgError

  if (size === 'xs') {
    return showImg ? (
      <img
        src={src}
        className={`${s.dim} rounded-full object-cover ${className}`}
        alt=""
        onError={() => setImgError(true)}
      />
    ) : (
      <span
        className={`${s.dim} inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 ${s.text} ${s.font} shrink-0 rounded-full ${className}`}
      >
        {initial}
      </span>
    )
  }

  const shrinkClass = size === 'md' || size === 'base' || size === 'lg' ? 'shrink-0' : ''
  const borderClass = size === 'lg' ? 'border border-gray-200 dark:border-gray-700' : ''

  return (
    <div className={`${shrinkClass} ${className}`}>
      {showImg ? (
        <img
          src={src}
          className={`${s.dim} rounded-full object-cover ${borderClass}`}
          alt=""
          onError={() => setImgError(true)}
        />
      ) : (
        <div
          className={`${s.dim} bg-gray-200 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-500 dark:text-gray-400 ${s.text} ${s.font}`}
        >
          {initial}
        </div>
      )}
    </div>
  )
}
