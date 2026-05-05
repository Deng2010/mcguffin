import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

interface TeamMemberAPI {
  id: string
  user_id: string
  name: string
  avatar: string
  avatar_url?: string | null
  role: string
  joined_at: string
}

interface JoinRequestAPI {
  id: string
  user_id: string
  user_name: string
  user_email: string
  reason: string
  status: string
  created_at: string
}

type TabId = 'all' | 'admin' | 'member'

export default function TeamPage() {
  const { user, hasPermission } = useAuth()
  const [members, setMembers] = useState<TeamMemberAPI[]>([])
  const [requests, setRequests] = useState<JoinRequestAPI[]>([])
  const [loading, setLoading] = useState(true)
  const canManage = hasPermission('manage_team')
  const [activeTab, setActiveTab] = useState<TabId>('all')

  const loadData = () => {
    Promise.all([
      apiFetch<TeamMemberAPI[]>('/team/members'),
      canManage ? apiFetch<JoinRequestAPI[]>('/team/requests') : Promise.resolve([]),
    ]).then(([m, r]) => {
      setMembers(m)
      setRequests(r)
    }).catch(() => {}).finally(() => setLoading(false))
  }

  useEffect(() => { loadData() }, [canManage])

  const filteredMembers = members.filter(m => {
    if (activeTab === 'admin') return m.role === 'admin' || m.role === 'superadmin'
    if (activeTab === 'member') return m.role === 'member'
    return true
  })

  const counts = {
    admin: members.filter(m => m.role === 'admin' || m.role === 'superadmin').length,
    member: members.filter(m => m.role === 'member').length,
  }

  const handleReviewRequest = async (requestId: string, action: 'approve' | 'reject') => {
    try {
      await apiFetch(`/team/review/${requestId}/${action}`, { method: 'POST' })
      loadData()
    } catch (err) { alert(`操作失败: ${err}`) }
  }

  const handleChangeRole = async (member: TeamMemberAPI, newRole: string) => {
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/team/members/role/${member.user_id}`,
        { method: 'POST', body: JSON.stringify({ role: newRole }) },
      )
      if (!res.success) { alert(res.message); return }
      loadData()
    } catch (err) { alert(`角色修改失败: ${err}`) }
  }

  const handleRemoveMember = async (member: TeamMemberAPI) => {
    if (!window.confirm(`确定要将 ${member.name} 移出团队吗？`)) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/team/members/remove/${member.user_id}`,
        { method: 'POST' },
      )
      if (!res.success) { alert(res.message); return }
      loadData()
    } catch (err) { alert(`移除失败: ${err}`) }
  }

  const roleLabel = (r: string) => (r === 'admin' || r === 'superadmin') ? '管理员' : '成员'
  const isCurrentUser = (member: TeamMemberAPI) => user?.id === member.user_id
  const isSuperAdmin = user?.role === 'superadmin'
  const canManageUser = (member: TeamMemberAPI) => {
    if (!canManage || isCurrentUser(member)) return false
    if (member.role === 'superadmin') return false
    if (member.role === 'admin' && !isSuperAdmin) return false
    return true
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <h1 className="text-2xl font-bold mb-6 text-gray-800 dark:text-gray-100">成员</h1>

      {user?.team_status !== 'joined' && (
        <div className="mb-6 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-800">
          <p className="text-blue-700 dark:text-blue-300">您还不是团队成员，<Link to="/apply" className="underline font-medium">申请加入团队</Link> 以参与协作</p>
        </div>
      )}

      {canManage && requests.length > 0 && (
        <div className="mb-8">
          <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">待处理入队申请</h2>
          <div className="space-y-2">
            {requests.map(req => (
              <div key={req.id} className="flex items-center justify-between p-4 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700">
                <div>
                  <span className="font-medium">{req.user_name}</span>
                  <span className="text-sm text-gray-500 dark:text-gray-400 ml-2">{req.user_email}</span>
                  <p className="text-sm text-gray-600 dark:text-gray-300 mt-1">{req.reason}</p>
                </div>
                <div className="flex gap-2">
                  <button onClick={() => handleReviewRequest(req.id, 'approve')} className="px-4 py-2 bg-gray-800 dark:bg-gray-700 text-white border border-gray-900 hover:bg-gray-700 dark:hover:bg-gray-600">接受</button>
                  <button onClick={() => handleReviewRequest(req.id, 'reject')} className="px-4 py-2 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700">拒绝</button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6">
        {[
          { id: 'all' as TabId, label: '全部', count: members.length },
          { id: 'admin' as TabId, label: '管理员', count: counts.admin },
          { id: 'member' as TabId, label: '成员', count: counts.member },
        ].map(tab => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-gray-800 dark:border-gray-100 text-gray-900 dark:text-gray-100'
                : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200'
            }`}
          >
            {tab.label}
            <span className={`ml-1.5 px-1.5 py-0.5 text-xs rounded ${
              activeTab === tab.id ? 'bg-gray-800 dark:bg-gray-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400'
            }`}>
              {tab.count}
            </span>
          </button>
        ))}
      </div>

      <div className="space-y-2">
        {filteredMembers.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">暂无{activeTab === 'admin' ? '管理员' : activeTab === 'member' ? '成员' : ''}</div>
        ) : (
          filteredMembers.map(m => (
            <div key={m.id} className="flex items-center justify-between p-4 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700">
              <div className="flex items-center gap-3">
                {m.avatar_url ? (
                  <img
                    src={m.avatar_url}
                    alt=""
                    className="w-10 h-10 rounded-full object-cover"
                    onError={(e) => {
                      const img = e.target as HTMLImageElement;
                      img.style.display = 'none';
                      const fallback = img.nextElementSibling as HTMLElement;
                      if (fallback) fallback.classList.remove('hidden');
                    }}
                  />
                ) : null}
                <div className={`w-10 h-10 bg-gray-300 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-600 dark:text-gray-400 font-bold text-sm ${m.avatar_url ? 'hidden' : ''}`}>
                  {m.name.charAt(0)}
                </div>
                <div>
                  <div className="font-medium flex items-center gap-2">
                    {m.name}
                    {isCurrentUser(m) && <span className="text-xs text-gray-400 dark:text-gray-500">(你)</span>}
                  </div>
                  <div className="text-sm text-gray-500 dark:text-gray-400">加入于 {m.joined_at}</div>
                </div>
              </div>

              <div className="flex items-center gap-3">
                {canManageUser(m) ? (
                  <select
                    value={m.role}
                    onChange={e => handleChangeRole(m, e.target.value)}
                    className="text-sm border border-gray-300 dark:border-gray-700 px-2 py-1 bg-white dark:bg-gray-800 focus:outline-none"
                  >
                    <option value="member">成员</option>
                    <option value="admin">管理员</option>
                  </select>
                ) : (
                  <span className="text-sm px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700">
                    {roleLabel(m.role)}
                  </span>
                )}

                {canManageUser(m) && (
                  <button
                    onClick={() => handleRemoveMember(m)}
                    className="px-3 py-1 text-sm text-red-600 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    移除
                  </button>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
