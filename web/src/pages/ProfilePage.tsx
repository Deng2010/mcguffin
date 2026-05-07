import { useState, useEffect } from 'react'
import { useParams, Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

interface PublicProfile {
  exists: boolean
  username: string
  display_name: string
  avatar_url: string | null
  bio: string
  role: string
  is_team_member: boolean
  team_role: string | null
  created_at: string
  message?: string
}

export default function ProfilePage() {
  const { user, refreshUser } = useAuth()
  const { username: routeUsername } = useParams<{ username: string }>()
  const [publicProfile, setPublicProfile] = useState<PublicProfile | null>(null)
  const [loadingProfile, setLoadingProfile] = useState(false)

  // Edit state
  const [editing, setEditing] = useState(false)
  const [displayName, setDisplayName] = useState('')
  const [avatarUrl, setAvatarUrl] = useState('')
  const [bio, setBio] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')

  // Determine if this is viewing self or someone else
  const isSelf = !routeUsername || (user && user.username === routeUsername)

  // Fetch public profile if viewing someone else
  useEffect(() => {
    if (routeUsername && !isSelf) {
      setLoadingProfile(true)
      apiFetch<PublicProfile>(`/user/profile/${routeUsername}`)
        .then(setPublicProfile)
        .catch(() => setPublicProfile(null))
        .finally(() => setLoadingProfile(false))
    } else {
      setPublicProfile(null)
    }
  }, [routeUsername, isSelf])

  if (!isSelf) {
    // Viewing someone else's profile
    if (loadingProfile) {
      return (
        <div className="p-6 max-w-2xl mx-auto text-center py-12">
          <p className="text-gray-400 dark:text-gray-500">加载中...</p>
        </div>
      )
    }

    if (!publicProfile || !publicProfile.exists) {
      return (
        <div className="p-6 max-w-2xl mx-auto text-center py-12">
          <p className="text-gray-500 dark:text-gray-400 mb-4">该用户不存在</p>
          <Link to="/" className="text-sm text-gray-500 dark:text-gray-400 underline">返回首页</Link>
        </div>
      )
    }

    const roleLabel = (role: string) => {
      switch (role) {
        case 'superadmin': return '超级管理员'
        case 'admin': return '管理员'
        case 'member': return '成员'
        case 'guest': return '游客'
        case 'pending': return '待审核'
        default: return role
      }
    }

    return (
      <div className="p-6 max-w-2xl mx-auto">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-6">个人主页</h1>
        <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-6">
          <div className="flex items-start gap-6 mb-6">
            <div className="shrink-0">
              {publicProfile.avatar_url ? (
                <img src={publicProfile.avatar_url} alt="" className="w-24 h-24 rounded-full object-cover border border-gray-200 dark:border-gray-700" />
              ) : (
                <div className="w-24 h-24 bg-gray-200 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-500 dark:text-gray-400 text-3xl font-bold">
                  {publicProfile.display_name?.charAt(0) || '?'}
                </div>
              )}
            </div>
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-1">
                <h2 className="text-xl font-bold text-gray-800 dark:text-gray-100">{publicProfile.display_name}</h2>
                <span className="px-2 py-0.5 text-xs font-medium bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300">
                  {roleLabel(publicProfile.role)}
                </span>
                {publicProfile.is_team_member && (
                  <span className="px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
                    团队成员
                  </span>
                )}
              </div>
              <p className="text-sm text-gray-500 dark:text-gray-400">@{publicProfile.username}</p>
              <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                加入于 {new Date(publicProfile.created_at).toLocaleDateString('zh-CN')}
              </p>
            </div>
          </div>
          {publicProfile.bio && (
            <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
              <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">简介</h3>
              <p className="text-sm text-gray-600 dark:text-gray-300 whitespace-pre-wrap">{publicProfile.bio}</p>
            </div>
          )}
        </div>
      </div>
    )
  }

  // Self profile
  if (!user) {
    return (
      <div className="p-6 max-w-2xl mx-auto text-center py-12">
        <p className="text-gray-400 dark:text-gray-500">请先登录</p>
      </div>
    )
  }

  const openEdit = () => {
    setDisplayName(user.display_name)
    setAvatarUrl(user.avatar_url || '')
    setBio(user.bio || '')
    setNewPassword('')
    setConfirmPassword('')
    setMsg('')
    setEditing(true)
  }

  const cancelEdit = () => {
    setEditing(false)
    setNewPassword('')
    setConfirmPassword('')
    setMsg('')
  }

  const handleSave = async () => {
    if (!displayName.trim()) {
      setMsg('显示名称不能为空')
      return
    }
    // Validate password if changing (only for non-superadmin)
    const isSuperadmin = user.role === 'superadmin'
    if (!isSuperadmin && (newPassword || confirmPassword)) {
      if (newPassword.length < 3) {
        setMsg('密码至少需要3个字符')
        return
      }
      if (newPassword !== confirmPassword) {
        setMsg('两次输入的密码不一致')
        return
      }
    }
    setSaving(true)
    setMsg('')
    try {
      const body: Record<string, any> = {}
      // Superadmin cannot change display_name or password from profile page
      if (!isSuperadmin && displayName !== user.display_name) body.display_name = displayName
      if (avatarUrl !== (user.avatar_url || '')) body.avatar_url = avatarUrl
      if (bio !== (user.bio || '')) body.bio = bio
      if (!isSuperadmin && newPassword) body.password = newPassword

      if (Object.keys(body).length === 0) {
        setMsg('没有修改')
        setSaving(false)
        return
      }

      const res = await apiFetch<{ success: boolean; message: string; user?: any }>('/user/profile', {
        method: 'PUT',
        body: JSON.stringify(body),
      })
      if (!res.success) {
        setMsg(res.message)
        setSaving(false)
        return
      }
      await refreshUser()
      setEditing(false)
      setNewPassword('')
      setConfirmPassword('')
      setMsg('已保存')
      setTimeout(() => setMsg(''), 2000)
    } catch (err) {
      setMsg(`保存失败: ${err}`)
    } finally {
      setSaving(false)
    }
  }

  const isSuperadmin = user.role === 'superadmin'

  const roleLabel = (role: string) => {
    switch (role) {
      case 'superadmin': return '超级管理员'
      case 'admin': return '管理员'
      case 'member': return '成员'
      case 'guest': return '游客'
      case 'pending': return '待审核'
      default: return role
    }
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-6">个人主页</h1>

      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg === '已保存' ? 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300' : 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300'
        }`}>
          {msg}
        </div>
      )}

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-6">
        {/* Avatar & Basic Info */}
        <div className="flex items-start gap-6 mb-6">
          <div className="shrink-0">
            {user.avatar_url ? (
              <img src={user.avatar_url} alt="" className="w-24 h-24 rounded-full object-cover border border-gray-200 dark:border-gray-700" />
            ) : (
              <div className="w-24 h-24 bg-gray-200 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-500 dark:text-gray-400 text-3xl font-bold">
                {user.display_name?.charAt(0) || '?'}
              </div>
            )}
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-3 mb-1">
              <h2 className="text-xl font-bold text-gray-800 dark:text-gray-100">{user.display_name}</h2>
              <span className="px-2 py-0.5 text-xs font-medium bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300">
                {roleLabel(user.role)}
              </span>
            </div>
            <p className="text-sm text-gray-500 dark:text-gray-400">@{user.username}</p>
            {user.email && (
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{user.email}</p>
            )}
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              加入于 {new Date(user.created_at).toLocaleDateString('zh-CN')}
            </p>
          </div>
          {!editing && (
            <button
              onClick={openEdit}
              className="px-4 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
            >
              编辑资料
            </button>
          )}
        </div>

        {/* Bio */}
        {!editing && user.bio && (
          <div className="border-t border-gray-200 dark:border-gray-700 pt-4 mb-4">
            <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">简介</h3>
            <p className="text-sm text-gray-600 dark:text-gray-300 whitespace-pre-wrap">{user.bio}</p>
          </div>
        )}

        {/* Edit Form — with password fields inside */}
        {editing && (
          <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
            <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-200 mb-4">编辑资料</h3>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">显示名称</label>
              <input
                type="text"
                value={displayName}
                onChange={e => setDisplayName(e.target.value)}
                disabled={isSuperadmin}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400 text-sm disabled:opacity-60 disabled:cursor-not-allowed"
                placeholder="您的显示名称"
              />
              {isSuperadmin && (
                <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                  超级管理员请前往 <Link to="/admin/config" className="text-blue-500 dark:text-blue-400 underline">配置页面</Link> 修改显示名称与密码
                </p>
              )}
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">头像 URL</label>
              <input
                type="url"
                value={avatarUrl}
                onChange={e => setAvatarUrl(e.target.value)}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400 text-sm"
                placeholder="https://example.com/avatar.png（留空清除）"
              />
              {avatarUrl && (
                <div className="mt-2 flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                  <span>预览：</span>
                  <img src={avatarUrl} alt="" className="w-8 h-8 rounded-full object-cover border border-gray-200 dark:border-gray-700"
                    onError={e => { (e.target as HTMLImageElement).style.display = 'none' }}
                  />
                </div>
              )}
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">简介</label>
              <textarea
                value={bio}
                onChange={e => setBio(e.target.value)}
                rows={5}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400 text-sm"
                placeholder="简单介绍一下自己..."
              />
            </div>

            {/* Password fields — hidden for superadmin (managed in config page) */}
            {isSuperadmin ? (
              <div className="mb-4 pt-4 border-t border-gray-200 dark:border-gray-700">
                <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">设置密码</h4>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  超级管理员请前往 <Link to="/admin/config" className="text-blue-500 dark:text-blue-400 underline">配置页面</Link> 修改密码
                </p>
              </div>
            ) : (
            <div className="mb-4 pt-4 border-t border-gray-200 dark:border-gray-700">
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">设置密码</h4>
              <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
                设置密码后可使用账户名或显示名称进行账密登录。留空则不修改密码。
              </p>
              <div className="space-y-3 max-w-sm">
                <input
                  type="password"
                  value={newPassword}
                  onChange={e => setNewPassword(e.target.value)}
                  placeholder="新密码（留空不修改）"
                  className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
                />
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={e => setConfirmPassword(e.target.value)}
                  placeholder="确认新密码"
                  className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
                />
              </div>
            </div>
            )}

            <div className="flex gap-3">
              <button
                onClick={handleSave}
                disabled={saving}
                className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
              >
                {saving ? '保存中...' : '保存'}
              </button>
              <button
                onClick={cancelEdit}
                className="px-6 py-2 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                取消
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
