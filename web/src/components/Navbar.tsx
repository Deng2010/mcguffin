import { Link, useLocation } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { useSite } from '../SiteContext'
import { useDarkMode } from '../DarkModeContext'
import { useNotifications } from '../NotificationContext'
import { useState, useRef, useEffect } from 'react'

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
  const showApply = isAuthenticated && user?.team_status !== 'joined'
  const showAdminConfig = user?.role === 'superadmin'

  const formatTime = (dateStr: string) => {
    const d = new Date(dateStr)
    const now = new Date()
    const diff = now.getTime() - d.getTime()
    const mins = Math.floor(diff / 60000)
    if (mins < 60) return `${mins}分钟前`
    const hours = Math.floor(mins / 60)
    if (hours < 24) return `${hours}小时前`
    const days = Math.floor(hours / 24)
    if (days < 7) return `${days}天前`
    return d.toLocaleDateString('zh-CN')
  }

  return (
    <nav className="bg-white dark:bg-gray-900 border-b border-gray-300 dark:border-gray-700 px-6 py-3">
      <div className="flex items-center justify-between max-w-6xl mx-auto">
        <div className="flex items-center gap-8">
          <Link to="/" className="text-lg font-bold text-gray-800 dark:text-gray-100 tracking-tight">{siteInfo?.name || 'McGuffin'}</Link>
          <div className="flex gap-5">
            {showProblems && navLink('/problems', '题目')}
            {showManageContests && navLink('/contests', '比赛')}
            {showTeam && navLink('/team', '成员')}
            {showSuggestions && navLink('/suggestions', '建议')}
            {navLink('/announcements', '公告')}
            {showAdminConfig && navLink('/admin/config', '配置')}
            {showAdminConfig && navLink('/admin/backup', '备份')}
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
              {showNotifications && (
                <div className="absolute right-0 top-full mt-2 w-80 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 shadow-lg z-50 max-h-96 flex flex-col">
                  <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200 dark:border-gray-700">
                    <span className="text-sm font-medium text-gray-800 dark:text-gray-200">通知</span>
                    {unreadCount > 0 && (
                      <button
                        onClick={markAllRead}
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
                            markRead(n.id)
                            setShowNotifications(false)
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
                                onClick={() => setShowNotifications(false)}
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
              )}
            </div>
          )}

          {isAuthenticated && user ? (
            <>
              <div className="flex items-center gap-2">
                <Link to="/profile" className="flex items-center gap-2 hover:opacity-80">
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
