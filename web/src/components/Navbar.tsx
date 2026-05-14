import { Link, useLocation } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { useSite } from '../SiteContext'
import { useDarkMode } from '../DarkModeContext'
import { useState, useRef, useEffect } from 'react'
import { useNotifications } from '../NotificationContext'
import NotificationDropdown from './NotificationDropdown'

export default function Navbar() {
  const { user, isAuthenticated, logout, hasPermission } = useAuth()
  const { siteInfo } = useSite()
  const { isDark, toggle } = useDarkMode()
  const { notifications, unreadCount, markRead, markAllRead } = useNotifications()
  const location = useLocation()
  const [showNotifications, setShowNotifications] = useState(false)
  const notifRef = useRef<HTMLDivElement>(null)

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (notifRef.current && !notifRef.current.contains(e.target as Node)) {
        setShowNotifications(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  // Close dropdown when navigating
  useEffect(() => {
    setShowNotifications(false)
  }, [location.pathname])

  const navLink = (to: string, label: string) => {
    const active = location.pathname === to
    return (
      <Link
        to={to}
        className={`text-sm ${active ? 'text-gray-900 dark:text-gray-100 font-medium' : 'text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200'}`}
      >
        {label}
      </Link>
    )
  }

  const showProblems = hasPermission('view_problems')
  const showTeam = hasPermission('view_team')
  const showManageContests = hasPermission('view_problems')
  const showSuggestions = hasPermission('view_suggestions')
  const canUseCommunity = showSuggestions || true // discussions is public
  const showApply = isAuthenticated && user?.team_status !== 'joined'
  const showAdminConfig = user?.role === 'superadmin'

  return (
    <nav className="bg-white dark:bg-gray-900 border-b border-gray-300 dark:border-gray-700 px-6 py-3">
      <div className="flex items-center justify-between max-w-6xl mx-auto">
        <div className="flex items-center gap-8">
          <Link to="/" className="text-lg font-bold text-gray-800 dark:text-gray-100 tracking-tight">{siteInfo?.name || 'McGuffin'}</Link>
          <div className="flex gap-5">
            {showProblems && navLink('/problems', '题目')}
            {showManageContests && navLink('/contests', '比赛')}
            {showTeam && navLink('/team', '成员')}
            {canUseCommunity && navLink('/community', '社区')}
            {showAdminConfig && navLink('/admin/config', '配置')}
            {showApply && navLink('/apply', '申请加入')}
          </div>
        </div>
        <div className="flex items-center gap-3">
          {/* Dark mode toggle */}
          <button
            onClick={toggle}
            className="p-1.5 text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
            title={isDark ? '切换亮色模式' : '切换深色模式'}
          >
            {isDark ? (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
              </svg>
            ) : (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
              </svg>
            )}
          </button>

          {/* Notification bell */}
          {isAuthenticated && (
            <div className="relative" ref={notifRef}>
              <button
                onClick={() => setShowNotifications(!showNotifications)}
                className="relative p-1.5 text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
                title="通知"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
                </svg>
                {unreadCount > 0 && (
                  <span className="absolute -top-1 -right-1 inline-flex items-center justify-center w-4 h-4 text-[10px] font-bold text-white bg-red-500 rounded-full">
                    {unreadCount > 9 ? '9+' : unreadCount}
                  </span>
                )}
              </button>

              {/* Notification dropdown */}
              <NotificationDropdown
                notifications={notifications}
                unreadCount={unreadCount}
                open={showNotifications}
                onClose={() => setShowNotifications(false)}
                onMarkRead={markRead}
                onMarkAllRead={markAllRead}
              />
            </div>
          )}

          {isAuthenticated && user ? (
            <>
              <div className="flex items-center gap-2">
                <Link to={`/profile/${user.username}`} className="flex items-center gap-2 hover:opacity-80">
                  {user.avatar_url ? (
                    <img src={user.avatar_url} alt="" className="w-7 h-7 rounded-full object-cover" />
                  ) : (
                    <div className="w-7 h-7 bg-gray-300 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-600 dark:text-gray-300 text-xs font-bold">
                      {user.display_name?.charAt(0) || '?'}
                    </div>
                  )}
                  <span className="text-sm text-gray-700 dark:text-gray-300">{user.display_name}</span>
                </Link>
                <span className="text-xs px-2 py-0.5 bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-300">
                  {user.role === 'superadmin' || user.role === 'admin' ? '管理员' : user.role === 'member' ? '成员' : user.role === 'pending' ? '待审核' : '游客'}
                </span>
              </div>
              <button onClick={logout} className="px-3 py-1 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800">退出</button>
            </>
          ) : (
            <Link to="/login" className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700">登录</Link>
          )}
        </div>
      </div>
    </nav>
  )
}
