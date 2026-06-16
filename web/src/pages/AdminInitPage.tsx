import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { apiFetch } from '../api'

export default function AdminInitPage() {
  const navigate = useNavigate()
  const [loading, setLoading] = useState(true)
  const [initialized, setInitialized] = useState(false)

  const [displayName, setDisplayName] = useState('')
  const [avatarUrl, setAvatarUrl] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [msg, setMsg] = useState('')
  const [success, setSuccess] = useState(false)

  useEffect(() => {
    const checkInit = async () => {
      try {
        const res = await apiFetch<{ initialized: boolean }>('/admin/init-status')
        if (res.initialized) {
          setInitialized(true)
          navigate('/login', { replace: true })
        }
      } catch {
        // If the API fails, show the form anyway
      } finally {
        setLoading(false)
      }
    }
    checkInit()
  }, [navigate])

  const handleSubmit = async () => {
    setMsg('')

    if (!displayName.trim()) {
      setMsg('显示名称不能为空')
      return
    }
    if (displayName.trim().length > 30) {
      setMsg('显示名称不能超过30个字符')
      return
    }
    if (!password) {
      setMsg('密码不能为空')
      return
    }
    if (password.length < 3) {
      setMsg('密码至少需要3个字符')
      return
    }
    if (password !== confirmPassword) {
      setMsg('两次输入的密码不一致')
      return
    }

    setSubmitting(true)
    try {
      const body: Record<string, any> = {
        display_name: displayName.trim(),
        password,
      }
      if (avatarUrl.trim()) {
        body.avatar_url = avatarUrl.trim()
      }
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/init', {
        method: 'POST',
        body: JSON.stringify(body),
      })
      if (!res.success) {
        setMsg(res.message)
        return
      }
      setSuccess(true)
    } catch (err) {
      setMsg(`初始化失败: ${err}`)
    } finally {
      setSubmitting(false)
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-100 dark:bg-gray-950">
        <div className="bg-white dark:bg-gray-900 p-8 w-full max-w-md border border-gray-300 dark:border-gray-700">
          <p className="text-center text-gray-400 dark:text-gray-500">检查初始化状态...</p>
        </div>
      </div>
    )
  }

  if (initialized) {
    return null
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100 dark:bg-gray-950">
      <div className="bg-white dark:bg-gray-900 p-8 w-full max-w-md border border-gray-300 dark:border-gray-700">
        <h1 className="text-2xl font-bold mb-2 text-gray-800 dark:text-gray-100">初始化管理员</h1>
        <p className="text-gray-500 dark:text-gray-400 mb-6">设置超级管理员账户</p>

        {msg && (
          <div className={`mb-4 p-3 text-sm border ${
            success
              ? 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
              : 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300'
          }`}>
            {msg}
          </div>
        )}

        {success ? (
          <div className="text-center">
            <p className="text-sm text-green-700 dark:text-green-300 mb-4">管理员初始化成功！</p>
            <button
              onClick={() => navigate('/login')}
              className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
            >
              前往登录
            </button>
          </div>
        ) : (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">显示名称</label>
              <input
                type="text"
                value={displayName}
                onChange={e => setDisplayName(e.target.value)}
                maxLength={30}
                onKeyDown={e => e.key === 'Enter' && handleSubmit()}
                placeholder="您的显示名称"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">头像 URL（可选）</label>
              <input
                type="url"
                value={avatarUrl}
                onChange={e => setAvatarUrl(e.target.value)}
                placeholder="https://example.com/avatar.png"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
              />
              {avatarUrl && (
                <div className="mt-2 flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                  <span>预览：</span>
                  <img
                    src={avatarUrl}
                    alt=""
                    className="w-8 h-8 rounded-full object-cover border border-gray-200 dark:border-gray-700"
                    onError={e => { (e.target as HTMLImageElement).style.display = 'none' }}
                  />
                </div>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">密码</label>
              <input
                type="password"
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleSubmit()}
                placeholder="至少3个字符"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">确认密码</label>
              <input
                type="password"
                value={confirmPassword}
                onChange={e => setConfirmPassword(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleSubmit()}
                placeholder="再次输入密码"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
              />
            </div>

            <button
              onClick={handleSubmit}
              disabled={submitting}
              className="w-full py-3 px-4 bg-gray-800 text-white font-medium border border-gray-900 hover:bg-gray-700 disabled:opacity-50 transition-colors dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600"
            >
              {submitting ? '初始化中...' : '初始化管理员'}
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
