import { useState, useEffect } from 'react'
import { apiFetch } from '../api'

interface AdminUser {
  id: string
  username: string
  display_name: string
  email: string | null
  role: string
  team_status: string
  is_team_member: boolean
  group_ids: string[]
  user_permissions: string[]
  created_at: string
}

interface MemberGroup {
  id: string
  name: string
  permissions: string[]
}

export default function AdminUsersPage() {
  const [users, setUsers] = useState<AdminUser[]>([])
  const [groups, setGroups] = useState<MemberGroup[]>([])
  const [loading, setLoading] = useState(true)
  const [msg, setMsg] = useState('')
  const [changingRole, setChangingRole] = useState<string | null>(null)

  const loadData = async () => {
    setLoading(true)
    try {
      const [uRes, gRes] = await Promise.all([
        apiFetch<AdminUser[]>('/admin/users'),
        apiFetch<MemberGroup[]>('/admin/groups'),
      ])
      setUsers(Array.isArray(uRes) ? uRes : [])
      setGroups(Array.isArray(gRes) ? gRes : [])
    } catch (err) { setMsg(`加载失败: ${err}`) }
    finally { setLoading(false) }
  }

  useEffect(() => { loadData() }, [])

  const handleChangeRole = async (userId: string, role: string) => {
    setChangingRole(userId)
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/admin/users/${userId}/role`, {
        method: 'POST',
        body: JSON.stringify({ role }),
      })
      if (res.success) {
        setMsg('角色已更新')
        loadData()
      } else {
        setMsg(res.message)
      }
    } catch (err) { setMsg(`操作失败: ${err}`) }
    finally { setChangingRole(null) }
  }

  const handleRemoveUser = async (userId: string, displayName: string) => {
    if (!confirm(`确定要删除用户「${displayName}」吗？此操作不可撤销。`)) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/admin/users/${userId}/remove`, {
        method: 'POST',
      })
      if (res.success) {
        setMsg('用户已删除')
        loadData()
      } else {
        setMsg(res.message)
      }
    } catch (err) { setMsg(`操作失败: ${err}`) }
  }

  if (loading) {
    return <div className="text-center py-8 text-gray-400 dark:text-gray-500">加载用户列表...</div>
  }

  return (
    <div>
      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') || msg.includes('不能') || msg.includes('不存在') || msg.includes('无效')
            ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300'
            : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
        }`}>
          {msg}
          <button onClick={() => setMsg('')} className="ml-3 text-xs underline">关闭</button>
        </div>
      )}
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
        共 {users.length} 个用户。超级管理员账号不可删除或修改。权限编辑请到「权限」页面。
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-gray-300 dark:border-gray-700">
              <th className="text-left py-2 px-3 font-medium text-gray-600 dark:text-gray-400">用户名</th>
              <th className="text-left py-2 px-3 font-medium text-gray-600 dark:text-gray-400">显示名</th>
              <th className="text-left py-2 px-3 font-medium text-gray-600 dark:text-gray-400">角色</th>
              <th className="text-left py-2 px-3 font-medium text-gray-600 dark:text-gray-400">团队状态</th>
              <th className="text-left py-2 px-3 font-medium text-gray-600 dark:text-gray-400">成员组</th>
              <th className="text-right py-2 px-3 font-medium text-gray-600 dark:text-gray-400">操作</th>
            </tr>
          </thead>
          <tbody>
            {users.map(u => (
              <tr key={u.id} className="border-b border-gray-200 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-900/50">
                <td className="py-3 px-3">
                  <div className="text-gray-800 dark:text-gray-100">{u.username}</div>
                  {u.email && <div className="text-xs text-gray-400">{u.email}</div>}
                </td>
                <td className="py-3 px-3 text-gray-800 dark:text-gray-100">{u.display_name}</td>
                <td className="py-3 px-3">
                  {u.id === 'admin' ? (
                    <span className="text-xs px-2 py-0.5 bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400 border border-yellow-300 dark:border-yellow-800">superadmin</span>
                  ) : (
                    <select
                      value={u.role}
                      onChange={e => handleChangeRole(u.id, e.target.value)}
                      disabled={changingRole === u.id}
                      className="text-xs border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1 text-gray-800 dark:text-gray-100"
                    >
                      <option value="admin">admin</option>
                      <option value="member">member</option>
                      <option value="guest">guest</option>
                    </select>
                  )}
                </td>
                <td className="py-3 px-3">
                  <span className={`text-xs px-2 py-0.5 border ${
                    u.team_status === 'joined'
                      ? 'border-green-300 text-green-600 dark:border-green-800 dark:text-green-400'
                      : u.team_status === 'pending'
                      ? 'border-yellow-300 text-yellow-600 dark:border-yellow-800 dark:text-yellow-400'
                      : 'border-gray-300 text-gray-500 dark:border-gray-700 dark:text-gray-400'
                  }`}>
                    {u.team_status === 'joined' ? '已加入' : u.team_status === 'pending' ? '待审核' : '未加入'}
                  </span>
                </td>
                <td className="py-3 px-3">
                  <div className="flex flex-wrap gap-1">
                    {u.group_ids.map(gid => {
                      const g = groups.find(x => x.id === gid)
                      return g ? (
                        <span key={gid} className="text-xs px-1.5 py-0.5 bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-300">
                          {g.name}
                        </span>
                      ) : null
                    })}
                    {u.group_ids.length === 0 && <span className="text-xs text-gray-400">-</span>}
                  </div>
                </td>
                <td className="py-3 px-3 text-right">
                  {u.id !== 'admin' && (
                    <button
                      onClick={() => handleRemoveUser(u.id, u.display_name)}
                      className="text-xs px-2 py-1 text-red-500 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20"
                    >
                      删除
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
