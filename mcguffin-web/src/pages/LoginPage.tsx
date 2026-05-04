import { useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'

export default function LoginPage() {
  const { login, adminLogin } = useAuth()
  const location = useLocation()
  const navigate = useNavigate()
  const error = new URLSearchParams(location.search).get('error')
  const errorMsg = new URLSearchParams(location.search).get('msg')
  const [password, setPassword] = useState('')
  const [adminError, setAdminError] = useState('')
  const [adminLoading, setAdminLoading] = useState(false)

  const handleAdminLogin = async () => {
    if (!password.trim()) return
    setAdminLoading(true)
    setAdminError('')
    const result = await adminLogin(password)
    setAdminLoading(false)
    if (result.success) {
      navigate('/')
    } else {
      setAdminError(result.message)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100">
      <div className="bg-white p-8 w-full max-w-md border border-gray-300">
        <h1 className="text-2xl font-bold mb-2 text-gray-800">McGuffin</h1>
        <p className="text-gray-500 mb-6">算法竞赛出题团队工具</p>

        {error && (
          <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm">
            登录失败：{error}
            {errorMsg && <div className="mt-1 text-xs text-red-500 break-all">{decodeURIComponent(errorMsg)}</div>}
          </div>
        )}

        <button
          onClick={login}
          className="w-full py-3 px-4 bg-gray-800 text-white font-medium border border-gray-900 hover:bg-gray-700 transition-colors"
        >
          通过 CP OAuth 登录
        </button>

        <div className="mt-6 pt-6 border-t border-gray-200">
          <p className="text-sm text-gray-500 mb-3">管理员密码登录</p>
          <div className="flex gap-2">
            <input
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleAdminLogin()}
              placeholder="输入管理员密码"
              className="flex-1 px-3 py-2 border border-gray-300 text-sm focus:outline-none focus:border-gray-500"
            />
            <button
              onClick={handleAdminLogin}
              disabled={adminLoading}
              className="px-4 py-2 bg-gray-800 text-white text-sm font-medium border border-gray-900 hover:bg-gray-700 disabled:opacity-50 transition-colors"
            >
              {adminLoading ? '登录中...' : '登录'}
            </button>
          </div>
          {adminError && (
            <p className="mt-2 text-sm text-red-600">{adminError}</p>
          )}
        </div>

        <div className="mt-6 text-center">
          <Link to="/" className="text-sm text-gray-500 underline">游客访问（仅查看成果）</Link>
        </div>
      </div>
    </div>
  )
}
