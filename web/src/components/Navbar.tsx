import { Link, useLocation } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { useSite } from '../SiteContext'

export default function Navbar() {
  const { user, isAuthenticated, logout, hasPermission } = useAuth()
  const { siteInfo } = useSite()
  const location = useLocation()

  const navLink = (to: string, label: string) => {
    const active = location.pathname === to
    return (
      <Link
        to={to}
        className={`text-sm ${active ? 'text-gray-900 font-medium' : 'text-gray-500 hover:text-gray-800'}`}
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

  return (
    <nav className="bg-white border-b border-gray-300 px-6 py-3">
      <div className="flex items-center justify-between max-w-6xl mx-auto">
        <div className="flex items-center gap-8">
          <Link to="/" className="text-lg font-bold text-gray-800 tracking-tight">{siteInfo?.name || 'McGuffin'}</Link>
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
          {isAuthenticated && user ? (
            <>
              <div className="flex items-center gap-2">
                <Link to="/profile" className="flex items-center gap-2 hover:opacity-80">
                  {user.avatar_url ? (
                    <img src={user.avatar_url} alt="" className="w-7 h-7 rounded-full object-cover" />
                  ) : (
                    <div className="w-7 h-7 bg-gray-300 rounded-full flex items-center justify-center text-gray-600 text-xs font-bold">
                      {user.display_name?.charAt(0) || '?'}
                    </div>
                  )}
                  <span className="text-sm text-gray-700">{user.display_name}</span>
                </Link>
                <span className="text-xs px-2 py-0.5 bg-gray-200 text-gray-600">
                  {user.role === 'superadmin' || user.role === 'admin' ? '管理员' : user.role === 'member' ? '成员' : user.role === 'pending' ? '待审核' : '游客'}
                </span>
              </div>
              <button onClick={logout} className="px-3 py-1 text-sm text-gray-600 hover:text-gray-900 border border-gray-300 hover:bg-gray-100">退出</button>
            </>
          ) : (
            <Link to="/login" className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700">登录</Link>
          )}
        </div>
      </div>
    </nav>
  )
}
