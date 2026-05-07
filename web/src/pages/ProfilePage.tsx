import { useState } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

export default function ProfilePage() {
  const { user, refreshUser } = useAuth()
  const [editing, setEditing] = useState(false)
  const [displayName, setDisplayName] = useState('')
  const [avatarUrl, setAvatarUrl] = useState('')
  const [bio, setBio] = useState('')
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')

  // Password change state
  const [changingPassword, setChangingPassword] = useState(false)
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [savingPassword, setSavingPassword] = useState(false)

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
    setMsg('')
    setEditing(true)
  }

  const cancelEdit = () => {
    setEditing(false)
    setMsg('')
  }

  const handleSave = async () => {
    if (!displayName.trim()) {
      setMsg('显示名称不能为空')
      return
    }
    setSaving(true)
    setMsg('')
    try {
      const body: Record<string, any> = {}
      if (displayName !== user.display_name) body.display_name = displayName
      if (avatarUrl !== (user.avatar_url || '')) body.avatar_url = avatarUrl
      if (bio !== (user.bio || '')) body.bio = bio

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
      setMsg('已保存')
      setTimeout(() => setMsg(''), 2000)
    } catch (err) {
      setMsg(`保存失败: ${err}`)
    } finally {
      setSaving(false)
    }
  }

  const handleChangePassword = async () => {
    if (!newPassword.trim()) {
      setMsg('请输入新密码')
      return
    }
    if (newPassword.length < 3) {
      setMsg('密码至少需要3个字符')
      return
    }
    if (newPassword !== confirmPassword) {
      setMsg('两次输入的密码不一致')
      return
    }
    setSavingPassword(true)
    setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/user/profile', {
        method: 'PUT',
        body: JSON.stringify({ password: newPassword }),
      })
      if (!res.success) {
        setMsg(res.message)
      } else {
        setMsg('密码已更新')
        setNewPassword('')
        setConfirmPassword('')
        setChangingPassword(false)
        setTimeout(() => setMsg(''), 2000)
      }
    } catch (err) {
      setMsg(`设置密码失败: ${err}`)
    } finally {
      setSavingPassword(false)
    }
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

      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg === '已保存' || msg === '密码已更新' ? 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300' : 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300'
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

        {/* Edit Form */}
        {editing && (
          <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
            <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-200 mb-4">编辑资料</h3>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">显示名称</label>
              <input
                type="text"
                value={displayName}
                onChange={e => setDisplayName(e.target.value)}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400 text-sm"
                placeholder="您的显示名称"
              />
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

            <div className="mb-6">
              <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">简介</label>
              <textarea
                value={bio}
                onChange={e => setBio(e.target.value)}
                rows={5}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400 text-sm"
                placeholder="简单介绍一下自己..."
              />
            </div>

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

      {/* Password Settings */}
      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-6 mt-6">
        <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-200 mb-2">设置密码</h3>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
          设置密码后可使用账户名或显示名称进行账密登录
        </p>

        {changingPassword ? (
          <div className="space-y-3 max-w-sm">
            <input
              type="password"
              value={newPassword}
              onChange={e => setNewPassword(e.target.value)}
              placeholder="新密码"
              className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
            />
            <input
              type="password"
              value={confirmPassword}
              onChange={e => setConfirmPassword(e.target.value)}
              placeholder="确认新密码"
              className="w-full px-4 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:border-gray-500 dark:focus:border-gray-400"
            />
            <div className="flex gap-3">
              <button
                onClick={handleChangePassword}
                disabled={savingPassword}
                className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
              >
                {savingPassword ? '保存中...' : '保存密码'}
              </button>
              <button
                onClick={() => { setChangingPassword(false); setNewPassword(''); setConfirmPassword('') }}
                className="px-6 py-2 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                取消
              </button>
            </div>
          </div>
        ) : (
          <button
            onClick={() => setChangingPassword(true)}
            className="px-4 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            设置密码
          </button>
        )}
      </div>
    </div>
  )
}
