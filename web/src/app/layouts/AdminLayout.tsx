import { Link, Outlet, useLocation } from 'react-router-dom'
import { useAuthStore } from '../../stores/authStore'
import { useSiteStore } from '../../stores/siteStore'
import DarkModeToggle from '../../components/ui/DarkModeToggle'

const navItems = [
  { path: '/admin/config', label: '配置' },
  { path: '/admin/users', label: '用户管理' },
  { path: '/admin/roles', label: '角色权限' },
  { path: '/admin/plugins', label: '插件管理' },
  { path: '/admin/backups', label: '备份管理' },
]

export default function AdminLayout() {
  const user = useAuthStore(s => s.user)
  const siteInfo = useSiteStore(s => s.siteInfo)
  const location = useLocation()

  return (
    <div className="h-screen bg-gray-100 dark:bg-gray-950 flex overflow-hidden">
      {/* Sidebar */}
      <aside className="w-56 bg-white dark:bg-gray-900 border-r border-gray-300 dark:border-gray-700 flex flex-col flex-shrink-0 h-screen sticky top-0">
        {/* Logo */}
        <div className="px-5 py-4 border-b border-gray-300 dark:border-gray-700">
          <Link to="/" className="text-lg font-bold text-gray-800 dark:text-gray-100 tracking-tight block">
            {siteInfo?.name || 'McGuffin'}
          </Link>
          <span className="text-xs text-gray-400 dark:text-gray-500">管理后台</span>
        </div>

        {/* Navigation */}
        <nav className="flex-1 px-3 py-4 space-y-1">
          {navItems.map(item => {
            const active = location.pathname === item.path
            return (
              <Link
                key={item.path}
                to={item.path}
                className={`block px-3 py-2 text-sm rounded-none transition-colors ${
                  active
                    ? 'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 font-medium'
                    : 'text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-800/50'
                }`}
              >
                {item.label}
              </Link>
            )
          })}
        </nav>

        {/* User area */}
        <div className="border-t border-gray-300 dark:border-gray-700 px-3 py-3 space-y-2">
          {/* User info */}
          {user ? (
            <Link to={`/profile/${user.username}`} className="flex items-center gap-2.5 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
              {user.avatar_url ? (
                <img src={user.avatar_url} alt="" className="w-8 h-8 rounded-full object-cover flex-shrink-0" />
              ) : (
                <div className="w-8 h-8 bg-gray-300 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-600 dark:text-gray-300 text-sm font-bold flex-shrink-0">
                  {user.display_name?.charAt(0) || '?'}
                </div>
              )}
              <div className="min-w-0 flex-1">
                <div className="text-sm text-gray-700 dark:text-gray-300 truncate">{user.display_name}</div>
                <div className="text-xs text-gray-400 dark:text-gray-500 truncate">
                  {user.role === 'superadmin' || user.role === 'admin' ? '管理员' : user.role === 'member' ? '成员' : '游客'}
                </div>
              </div>
            </Link>
          ) : null}

          {/* Return to main site + Dark mode */}
          <div className="flex items-center justify-between px-3 py-1.5">
            <Link
              to="/"
              className="text-xs text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            >
              返回主站
            </Link>
            <DarkModeToggle />
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 p-6 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  )
}
