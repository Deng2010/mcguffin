interface RoleBadgeProps {
  role: string
  variant?: 'default' | 'colored'
  className?: string
}

const ROLE_LABELS: Record<string, string> = {
  superadmin: '超级管理员',
  admin: '管理员',
  member: '成员',
  guest: '游客',
  pending: '待审核',
}

const COLORED_STYLES: Record<string, string> = {
  superadmin: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300',
  admin: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300',
  member: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300',
  guest: 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300',
  pending: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-300',
}

const DEFAULT_STYLE = 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300'

export default function RoleBadge({ role, variant = 'default', className = '' }: RoleBadgeProps) {
  const label = ROLE_LABELS[role] ?? role
  const style = variant === 'colored'
    ? (COLORED_STYLES[role] ?? DEFAULT_STYLE)
    : DEFAULT_STYLE

  return (
    <span className={`px-2 py-0.5 text-xs font-medium ${style} ${className}`}>
      {label}
    </span>
  )
}
