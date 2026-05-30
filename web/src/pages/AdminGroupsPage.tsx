import { useState, useEffect } from 'react'
import { apiFetch } from '../api'

interface MemberGroup {
  id: string
  name: string
  permissions: string[]
}

const PERM_LABELS: Record<string, string> = {
  view_showcase: '浏览展示',
  apply_join: '申请加入',
  view_team: '查看团队',
  manage_team: '审批入队',
  manage_members: '管理成员',
  submit_problem: '投稿题目',
  view_problems: '浏览题目',
  approve_problem: '审核题目',
  manage_contests: '管理赛事',
  manage_site: '管理站点',
  edit_showcase: '编辑展示',
  view_discussions: '浏览讨论',
  manage_discussions: '管理讨论',
  manage_tags: '管理标签',
  manage_notifications: '发送通知',
  manage_backups: '备份恢复',
  view_stats: '查看统计',
  manage_posts: '管理帖子',
}

export default function AdminGroupsPage() {
  const [groups, setGroups] = useState<MemberGroup[]>([])
  const [loading, setLoading] = useState(true)
  const [msg, setMsg] = useState('')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editName, setEditName] = useState('')
  const [newName, setNewName] = useState('')

  const loadGroups = async () => {
    setLoading(true)
    try {
      const res = await apiFetch<MemberGroup[]>('/admin/groups')
      setGroups(Array.isArray(res) ? res : [])
    } catch (err) { setMsg(`加载失败: ${err}`) }
    finally { setLoading(false) }
  }

  useEffect(() => { loadGroups() }, [])

  const handleCreate = async () => {
    const name = newName.trim()
    if (!name) return
    try {
      const res = await apiFetch<{ success: boolean; message: string; id?: string }>('/admin/groups', {
        method: 'POST',
        body: JSON.stringify({ name, permissions: [] }),
      })
      if (res.success) {
        setNewName('')
        loadGroups()
      } else {
        setMsg(res.message)
      }
    } catch (err) { setMsg(`创建失败: ${err}`) }
  }

  const startEdit = (g: MemberGroup) => {
    setEditingId(g.id)
    setEditName(g.name)
  }

  const handleSave = async () => {
    if (!editingId) return
    const name = editName.trim()
    if (!name) return
    try {
      const currentGroup = groups.find(g => g.id === editingId)
      const res = await apiFetch<{ success: boolean; message: string }>(`/admin/groups/${editingId}`, {
        method: 'PUT',
        body: JSON.stringify({ name, permissions: currentGroup?.permissions ?? [] }),
      })
      if (res.success) {
        setEditingId(null)
        loadGroups()
      } else {
        setMsg(res.message)
      }
    } catch (err) { setMsg(`保存失败: ${err}`) }
  }

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`确定要删除成员组「${name}」吗？`)) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/admin/groups/${id}`, { method: 'DELETE' })
      if (res.success) {
        if (editingId === id) setEditingId(null)
        loadGroups()
      } else {
        setMsg(res.message)
      }
    } catch (err) { setMsg(`删除失败: ${err}`) }
  }

  if (loading) return <div className="text-center py-8 text-gray-400 dark:text-gray-500">加载成员组...</div>

  return (
    <div>
      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
        }`}>
          {msg}
          <button onClick={() => setMsg('')} className="ml-3 text-xs underline">关闭</button>
        </div>
      )}

      {/* Create */}
      <div className="flex items-center gap-2 mb-6 p-3 bg-blue-50 dark:bg-blue-900/30 border border-dashed border-blue-300 dark:border-blue-800">
        <input type="text" value={newName} onChange={e => setNewName(e.target.value)}
          className="flex-1 px-3 py-1.5 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm focus:outline-none"
          placeholder="新成员组名称" onKeyDown={e => e.key === 'Enter' && handleCreate()} />
        <button onClick={handleCreate} className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600">创建</button>
      </div>

      {/* Group list */}
      {groups.length === 0 && <p className="text-center py-8 text-gray-400 dark:text-gray-500">暂无成员组</p>}
      <div className="space-y-4">
        {groups.map(g => (
          <div key={g.id} className="border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 shadow">
            {editingId === g.id ? (
              <div className="p-4">
                <div className="flex items-center gap-2 mb-3">
                  <label className="text-sm text-gray-600 dark:text-gray-400 shrink-0">组名</label>
                  <input type="text" value={editName} onChange={e => setEditName(e.target.value)}
                    className="flex-1 px-3 py-1.5 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm" />
                  <button onClick={handleSave} className="px-3 py-1.5 bg-green-600 text-white text-sm hover:bg-green-700">保存</button>
                  <button onClick={() => setEditingId(null)} className="px-3 py-1.5 border border-gray-300 dark:border-gray-700 text-sm hover:bg-gray-100 dark:hover:bg-gray-800">取消</button>
                </div>
              </div>
            ) : (
              <div>
                <div className="flex items-center justify-between px-4 py-3 border-b border-gray-100 dark:border-gray-800">
                  <h3 className="text-sm font-semibold text-gray-800 dark:text-gray-100">{g.name}</h3>
                  <div className="flex items-center gap-2">
                    <button onClick={() => startEdit(g)} className="text-xs px-2 py-1 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800">编辑</button>
                    <button onClick={() => handleDelete(g.id, g.name)} className="text-xs px-2 py-1 border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20">删除</button>
                  </div>
                </div>
                <div className="px-4 py-2">
                  {g.permissions.length === 0 ? (
                    <span className="text-xs text-gray-400">无额外权限</span>
                  ) : (
                    <div className="flex flex-wrap gap-1">
                      {g.permissions.map(p => (
                        <span key={p} className="px-2 py-0.5 text-xs bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                          {PERM_LABELS[p] ?? p}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
