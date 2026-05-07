import { useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { useSite } from '../SiteContext'

export default function LoginPage() {
  const { login, accountLogin } = useAuth()
  const { siteInfo } = useSite()
  const location = useLocation()
  const navigate = useNavigate()
  const error = new URLSearchParams(location.search).get('error')
  const errorMsg = new URLSearchParams(location.search).get('msg')
  const [identifier, setIdentifier] = useState('')
  const [password, setPassword] = useState('')
  const [loginError, setLoginError] = useState('')
  const [loading, setLoading] = useState(false)

  const siteName = siteInfo?.name || 'McGuffin'

  const handleLogin = async () => {
    if (!password.trim()) return
    setLoading(true)
    setLoginError('')
    const result = await accountLogin(identifier, password)
    setLoading(false)
    if (result.success) {
      navigate('/')
    } else {
      setLoginError(result.message)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100 dark:bg-gray-950">
      <div className="bg-white dark:bg-gray-900 p-8 w-full max-w-md border border-gray-300 dark:border-gray-700">
        <h1 className="text-2xl font-bold mb-2 text-gray-800 dark:text-gray-100">{siteName}</h1>
        <p className="text-gray-500 dark:text-gray-400 mb-6">算法竞赛出题团队工具</p>

        {error && (
          <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">
            登录失败：{error}
            {errorMsg && <div className="mt-1 text-xs text-red-500 dark:text-red-400 break-all">{decodeURIComponent(errorMsg)}</div>}
          </div>
        )}

        <button
          onClick={login}
          className="w-full py-3 px-4 bg-gray-800 text-white font-medium border border-gray-900 hover:bg-gray-700 transition-colors dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600"
        >
          通过 CP OAuth 登录
        </button>

        <div className="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-3">账密登录</p>
          <div className="space-y-2">
            <input
              type="text"
              value={identifier}
              onChange={e => setIdentifier(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleLogin()}
              placeholder="账户名或显示名称（管理员登录留空）"
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
            />
            <div className="flex gap-2">
              <input
                type="password"
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleLogin()}
                placeholder="输入密码"
                className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
              />
              <button
                onClick={handleLogin}
                disabled={loading}
                className="px-4 py-2 bg-gray-800 text-white text-sm font-medium border border-gray-900 hover:bg-gray-700 disabled:opacity-50 transition-colors dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600"
              >
                {loading ? '登录中...' : '登录'}
              </button>
            </div>
          </div>
          {loginError && (
            <p className="mt-2 text-sm text-red-600 dark:text-red-400">{loginError}</p>
          )}
        </div>

        <div className="mt-6 text-center">
          <Link to="/" className="text-sm text-gray-500 dark:text-gray-400 underline">游客访问（仅查看成果）</Link>
        </div>
      </div>
    </div>
  )
}
