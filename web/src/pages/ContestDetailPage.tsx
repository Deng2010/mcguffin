import { useState, useEffect } from 'react'
import { useParams, Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownEditor from '../components/MarkdownEditor'
import { useDifficulties, DiffBadge } from '../hooks/useDifficulties'

interface ContestDetail {
  id: string
  name: string
  start_time: string
  end_time: string
  description: string
  created_by: string
  created_at: string
  status: string
  link?: string | null
  visible_to?: string[]
  editable_by?: string[]
}

interface ContestProblem {
  id: string
  title: string
  author_name: string
  difficulty: string
  status: string
}

interface MemberOption {
  id: string
  username: string
  display_name: string
}

export default function ContestDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { user, hasPermission } = useAuth()
  const { difficultyMap } = useDifficulties()
  const isAdmin = hasPermission('approve_problem')
  const canEdit = user && (isAdmin || user.role === 'member' || user.role === 'superadmin')

  const [contest, setContest] = useState<ContestDetail | null>(null)
  const [problems, setProblems] = useState<ContestProblem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  // Edit state
  const [editing, setEditing] = useState(false)
  const [editName, setEditName] = useState('')
  const [editStartTime, setEditStartTime] = useState('')
  const [editEndTime, setEditEndTime] = useState('')
  const [editDescription, setEditDescription] = useState('')
  const [editLink, setEditLink] = useState('')
  const [editProblems, setEditProblems] = useState<ContestProblem[]>([])
  const [editProblemsReady, setEditProblemsReady] = useState(false)
  const [editVisibleTo, setEditVisibleTo] = useState<string[]>([])
  const [editEditableBy, setEditEditableBy] = useState<string[]>([])
  const [allMembers, setAllMembers] = useState<MemberOption[]>([])
  const [saving, setSaving] = useState(false)

  const loadData = async () => {
    if (!id) return
    setLoading(true)
    try {
      const [contestsRes, problemsRes] = await Promise.all([
        apiFetch<ContestDetail[]>('/contests'),
        apiFetch<ContestProblem[]>(`/contests/${id}/problems`),
      ])
      const found = contestsRes.find((c: ContestDetail) => c.id === id)
      if (found) setContest(found)
      setProblems(problemsRes)
    } catch { setError('加载失败') }
    finally { setLoading(false) }
  }

  useEffect(() => { loadData() }, [id])

  const openEdit = () => {
    if (!contest) return
    setEditName(contest.name)
    setEditStartTime(contest.start_time)
    setEditEndTime(contest.end_time)
    setEditDescription(contest.description)
    setEditLink(contest.link || '')
    setEditProblemsReady(false)
    setError('')
    setEditVisibleTo(contest.visible_to || [])
    setEditEditableBy(contest.editable_by || [])
    setEditing(true)
    if (isAdmin && allMembers.length === 0) {
      apiFetch<MemberOption[]>('/admin/users').then(setAllMembers).catch(() => {})
    }
    apiFetch<ContestProblem[]>(`/contests/${id}/problems`)
      .then(setEditProblems)
      .catch(() => setEditProblems([]))
      .finally(() => setEditProblemsReady(true))
  }

  const cancelEdit = () => {
    setEditing(false)
    setEditProblems([])
    setEditProblemsReady(false)
    setError('')
    setEditVisibleTo([])
    setEditEditableBy([])
  }

  const moveProblem = (index: number, direction: -1 | 1) => {
    const i2 = index + direction
    if (i2 < 0 || i2 >= editProblems.length) return
    const next = [...editProblems]
    ;[next[index], next[i2]] = [next[i2], next[index]]
    setEditProblems(next)
  }

  const toggleVisibleMember = (uid: string) =>
    setEditVisibleTo(p => p.includes(uid) ? p.filter(x => x !== uid) : [...p, uid])

  const toggleEditableMember = (uid: string) =>
    setEditEditableBy(p => p.includes(uid) ? p.filter(x => x !== uid) : [...p, uid])

  const handleSave = async () => {
    if (!id || !editName.trim() || !editStartTime.trim() || !editEndTime.trim()) {
      setError('请填写比赛名称、开始时间和结束时间'); return
    }
    setSaving(true); setError('')
    try {
      const r1 = await apiFetch<{ success: boolean }>(`/contests/${id}`, {
        method: 'PUT',
        body: JSON.stringify({
          name: editName, start_time: editStartTime, end_time: editEndTime,
          description: editDescription, link: editLink || undefined,
        }),
      })
      if (!r1.success) { setError('保存比赛信息失败'); setSaving(false); return }
      const r2 = await apiFetch<{ success: boolean }>(`/contests/${id}/problem-order`, {
        method: 'POST', body: JSON.stringify({ problem_ids: editProblems.map(p => p.id) }),
      })
      if (!r2.success) { setError('保存题目顺序失败'); setSaving(false); return }
      if (isAdmin) {
        const r3 = await apiFetch<{ success: boolean }>(`/admin/acl/contest/${id}`, {
          method: 'PUT', body: JSON.stringify({ visible_to: editVisibleTo, editable_by: editEditableBy }),
        })
        if (!r3.success) { setError('保存访问控制失败'); setSaving(false); return }
      }
      setEditing(false)
      loadData()
    } catch { setError('保存失败') }
    finally { setSaving(false) }
  }

  const toggleStatus = async () => {
    if (!contest) return
    const newStatus = contest.status === 'public' ? 'draft' : 'public'
    if (newStatus === 'public') {
      const url = prompt('请输入比赛外部链接：', contest.link || 'https://')
      if (!url) return
      if (!confirm('确定要公开此比赛吗？')) return
      try {
        const r = await apiFetch<{ success: boolean }>(`/contests/${id}/status`, {
          method: 'POST', body: JSON.stringify({ status: newStatus, link: url }),
        })
        if (!r.success) { alert('操作失败'); return }
        loadData()
      } catch { alert('操作失败') }
    } else {
      if (!confirm('确定要取消公开此比赛吗？')) return
      try {
        const r = await apiFetch<{ success: boolean }>(`/contests/${id}/status`, {
          method: 'POST', body: JSON.stringify({ status: newStatus }),
        })
        if (!r.success) { alert('操作失败'); return }
        loadData()
      } catch { alert('操作失败') }
    }
  }

  // —— Loading / Error ——
  if (loading) return <div className="p-6 max-w-4xl mx-auto text-center py-12 text-gray-400">加载中...</div>
  if (!contest) return (
    <div className="p-6 max-w-4xl mx-auto text-center py-12">
      <p className="text-gray-500 dark:text-gray-400 mb-4">比赛不存在</p>
      <Link to="/contests" className="text-sm text-blue-600 dark:text-blue-400 underline">返回比赛列表</Link>
    </div>
  )

  const inputClass = "w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800"

  // ========== Read Mode ==========
  if (!editing) return (
    <div className="p-6 max-w-4xl mx-auto">
      <Link to="/contests" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 mb-4 inline-block">&larr; 返回比赛列表</Link>

      {error && <div className="mb-4 p-3 text-sm bg-red-50 border border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">{error}</div>}

      <div className="mg-box-shadow p-6 mb-6">
        <div className="flex items-start justify-between mb-3">
          <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">{contest.name}
            <span className={`ml-3 px-2 py-0.5 text-xs font-medium align-middle ${contest.status === 'public' ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300' : 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'}`}>
              {contest.status === 'public' ? '已公开' : '未公开'}
            </span>
          </h1>
          <div className="flex gap-2 shrink-0 ml-4">
            {isAdmin && (
              <button onClick={toggleStatus}
                className={`px-3 py-1.5 text-xs border ${contest.status === 'public' ? 'border-yellow-500 text-yellow-700 hover:bg-yellow-50 dark:border-yellow-800 dark:text-yellow-400 dark:hover:bg-yellow-900/20' : 'border-green-500 text-green-700 hover:bg-green-50 dark:border-green-800 dark:text-green-400 dark:hover:bg-green-900/20'}`}>
                {contest.status === 'public' ? '取消公开' : '公开'}
              </button>
            )}
            {canEdit && (
              <button onClick={openEdit}
                className="px-3 py-1.5 text-xs border border-gray-300 text-gray-600 hover:bg-gray-100 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-800">
                编辑
              </button>
            )}
          </div>
        </div>

        <div className="space-y-2 text-sm text-gray-600 dark:text-gray-300">
          <p>时间：{contest.start_time} ~ {contest.end_time}</p>
          {contest.link && (
            <p>链接：<a href={contest.link} target="_blank" rel="noopener noreferrer"
              className="text-blue-600 dark:text-blue-400 underline">打开比赛 ↗</a></p>
          )}
          {contest.description && <div className="mt-4 whitespace-pre-wrap">{contest.description}</div>}
          <p className="text-xs text-gray-400 dark:text-gray-500 mt-2">创建于 {contest.created_at}</p>
        </div>
      </div>

      <div className="mg-box-shadow p-6">
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">题目列表</h2>
        {problems.length === 0 ? (
          <p className="text-sm text-gray-400 dark:text-gray-500">暂无题目</p>
        ) : (
          <div className="space-y-2">
            {problems.map((p, idx) => (
              <div key={p.id} className="flex items-center gap-3 p-3 mg-box">
                <span className="w-6 text-center font-mono text-sm text-gray-500 dark:text-gray-400">{idx + 1}</span>
                <Link to={`/problems/${p.id}`} className="flex-1 text-sm text-gray-800 dark:text-gray-100 hover:text-blue-600 dark:hover:text-blue-400">{p.title}</Link>
                <span className="text-xs text-gray-500 dark:text-gray-400">{p.author_name}</span>
                <DiffBadge difficulty={p.difficulty} map={difficultyMap} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )

  // ========== Edit Mode ==========
  return (
    <div className="p-6 max-w-4xl mx-auto">
      <Link to="/contests" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 mb-4 inline-block">&larr; 返回比赛列表</Link>

      {error && <div className="mb-4 p-3 text-sm bg-red-50 border border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">{error}</div>}

      <div className="mg-box-shadow p-6 mb-6">
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">编辑比赛 — {contest.name}</h2>

        <div className="grid grid-cols-3 gap-4 mb-4">
          <div>
            <label className="block text-xs font-medium mb-1 text-gray-600 dark:text-gray-300">比赛名称</label>
            <input type="text" value={editName} onChange={e => setEditName(e.target.value)} className={inputClass} />
          </div>
          <div>
            <label className="block text-xs font-medium mb-1 text-gray-600 dark:text-gray-300">开始时间</label>
            <input type="text" value={editStartTime} onChange={e => setEditStartTime(e.target.value)} className={inputClass} placeholder="2026-05-01 10:00" />
          </div>
          <div>
            <label className="block text-xs font-medium mb-1 text-gray-600 dark:text-gray-300">结束时间</label>
            <input type="text" value={editEndTime} onChange={e => setEditEndTime(e.target.value)} className={inputClass} placeholder="2026-05-01 12:00" />
          </div>
        </div>
        <div className="mb-4">
          <label className="block text-xs font-medium mb-1 text-gray-600 dark:text-gray-300">链接</label>
          <input type="url" value={editLink} onChange={e => setEditLink(e.target.value)} className={inputClass} placeholder="https://..." />
        </div>
        <div className="mb-6">
          <MarkdownEditor value={editDescription} onChange={setEditDescription} label="简介" rows={10} />
        </div>

        {/* Problem order */}
        <div className="border-t border-gray-200 pt-4 mb-6 dark:border-gray-700">
          <h3 className="text-sm font-semibold text-gray-700 mb-3 dark:text-gray-200">题目顺序</h3>
          {!editProblemsReady ? (
            <div className="text-sm text-gray-400">加载题目中...</div>
          ) : editProblems.length === 0 ? (
            <div className="text-sm text-gray-400">该比赛暂无题目</div>
          ) : (
            <div className="space-y-1.5">
              {editProblems.map((p, idx) => (
                <div key={p.id} className="flex items-center gap-3 p-2 bg-gray-50 border border-gray-200 text-sm dark:bg-gray-800/50 dark:border-gray-700">
                  <span className="w-6 text-center font-mono text-gray-500 dark:text-gray-400">{idx + 1}</span>
                  <span className="flex-1 text-gray-800 dark:text-gray-100">{p.title}</span>
                  <span className="text-xs text-gray-500">{p.author_name}</span>
                  <DiffBadge difficulty={p.difficulty} map={difficultyMap} />
                  <button onClick={() => moveProblem(idx, -1)} disabled={idx === 0}
                    className="px-2 py-1 text-xs border border-gray-300 text-gray-600 hover:bg-gray-200 disabled:opacity-30 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-700">↑</button>
                  <button onClick={() => moveProblem(idx, 1)} disabled={idx === editProblems.length - 1}
                    className="px-2 py-1 text-xs border border-gray-300 text-gray-600 hover:bg-gray-200 disabled:opacity-30 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-700">↓</button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* ACL */}
        {isAdmin && allMembers.length > 0 && (
          <div className="border-t border-gray-200 pt-4 mb-6 dark:border-gray-700">
            <h3 className="text-sm font-semibold text-gray-700 mb-3 dark:text-gray-200">访问控制</h3>
            <div className="mb-3">
              <label className="block text-xs font-medium mb-1.5 text-gray-600 dark:text-gray-300">可见成员</label>
              <div className="flex flex-wrap gap-2">
                {allMembers.map(m => (
                  <label key={m.id} className="flex items-center gap-1.5 text-sm cursor-pointer">
                    <input type="checkbox" checked={editVisibleTo.includes(m.id)}
                      onChange={() => toggleVisibleMember(m.id)} className="accent-gray-800 dark:accent-gray-400" />
                    {m.display_name || m.username}
                  </label>
                ))}
              </div>
            </div>
            <div>
              <label className="block text-xs font-medium mb-1.5 text-gray-600 dark:text-gray-300">可编辑成员</label>
              <div className="flex flex-wrap gap-2">
                {allMembers.map(m => (
                  <label key={m.id} className="flex items-center gap-1.5 text-sm cursor-pointer">
                    <input type="checkbox" checked={editEditableBy.includes(m.id)}
                      onChange={() => toggleEditableMember(m.id)} className="accent-gray-800 dark:accent-gray-400" />
                    {m.display_name || m.username}
                  </label>
                ))}
              </div>
            </div>
          </div>
        )}

        <div className="flex gap-2">
          <button onClick={handleSave} disabled={saving || !editProblemsReady}
            className="px-5 py-2 text-sm bg-gray-800 text-white hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600">
            {saving ? '保存中...' : '保存全部'}
          </button>
          <button onClick={cancelEdit}
            className="px-5 py-2 text-sm border border-gray-300 text-gray-600 hover:bg-gray-100 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-800">
            取消
          </button>
        </div>
      </div>
    </div>
  )
}
