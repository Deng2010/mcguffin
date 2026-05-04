import { useState, useEffect } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import { useDifficulties, DiffBadge } from '../hooks/useDifficulties'

interface ContestItem {
  id: string
  name: string
  start_time: string
  end_time: string
  description: string
  created_by: string
  created_at: string
  status: string
  link?: string | null
  problem_order: string[]
}

interface ContestProblem {
  id: string
  title: string
  author_name: string
  difficulty: string
  status: string
}

type TabId = 'all' | 'public' | 'draft'

export default function ContestManagePage() {
  const { hasPermission } = useAuth()
  const { difficultyMap } = useDifficulties()
  const isAdmin = hasPermission('approve_problem')
  const [contests, setContests] = useState<ContestItem[]>([])
  const [loading, setLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)
  const [name, setName] = useState('')
  const [startTime, setStartTime] = useState('')
  const [endTime, setEndTime] = useState('')
  const [description, setDescription] = useState('')
  const [link, setLink] = useState('')
  const [error, setError] = useState('')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<TabId>('all')

  // Edit state — unified: contest info + problem order
  const [editName, setEditName] = useState('')
  const [editStartTime, setEditStartTime] = useState('')
  const [editEndTime, setEditEndTime] = useState('')
  const [editDescription, setEditDescription] = useState('')
  const [editLink, setEditLink] = useState('')
  const [editProblems, setEditProblems] = useState<ContestProblem[]>([])
  const [editProblemsReady, setEditProblemsReady] = useState(false)
  const [editSaving, setEditSaving] = useState(false)

  const loadContests = () => {
    apiFetch<ContestItem[]>('/contests')
      .then(setContests)
      .catch(() => setContests([]))
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadContests() }, [])

  const filteredContests = contests.filter(c => {
    if (activeTab === 'public') return c.status === 'public'
    if (activeTab === 'draft') return c.status === 'draft'
    return true
  })

  const counts = {
    public: contests.filter(c => c.status === 'public').length,
    draft: contests.filter(c => c.status === 'draft').length,
  }

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim() || !startTime.trim() || !endTime.trim()) {
      setError('请填写比赛名称、开始时间和结束时间')
      return
    }
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/contests', {
        method: 'POST',
        body: JSON.stringify({ name, start_time: startTime, end_time: endTime, description, link: link || undefined }),
      })
      if (!res.success) { setError(res.message); return }
      setName(''); setStartTime(''); setEndTime(''); setDescription(''); setLink('')
      setShowForm(false); setError('')
      loadContests()
    } catch (err) { setError(`创建失败: ${err}`) }
  }

  const handleDelete = async (contestId: string, contestName: string) => {
    if (!window.confirm(`确定要永久删除比赛「${contestName}」吗？关联此比赛的题目将保留但不再关联。`)) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/contests/${contestId}`, { method: 'DELETE' })
      if (!res.success) { alert(res.message); return }
      loadContests()
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  // ====== Unified Edit ======

  const startEdit = async (c: ContestItem) => {
    setEditingId(c.id)
    setEditName(c.name)
    setEditStartTime(c.start_time)
    setEditEndTime(c.end_time)
    setEditDescription(c.description)
    setEditLink(c.link || '')
    setEditProblemsReady(false)
    setError('')
    // Load problems for ordering
    try {
      const problems = await apiFetch<ContestProblem[]>(`/contests/${c.id}/problems`)
      setEditProblems(problems)
    } catch {
      setEditProblems([])
    } finally {
      setEditProblemsReady(true)
    }
  }

  const cancelEdit = () => {
    setEditingId(null)
    setEditProblems([])
    setEditProblemsReady(false)
    setError('')
  }

  const moveProblem = (index: number, direction: -1 | 1) => {
    const newIndex = index + direction
    if (newIndex < 0 || newIndex >= editProblems.length) return
    const newProblems = [...editProblems]
    const temp = newProblems[index]
    newProblems[index] = newProblems[newIndex]
    newProblems[newIndex] = temp
    setEditProblems(newProblems)
  }

  const handleSaveEdit = async () => {
    if (!editingId) return
    if (!editName.trim() || !editStartTime.trim() || !editEndTime.trim()) {
      setError('请填写比赛名称、开始时间和结束时间')
      return
    }
    setEditSaving(true)
    setError('')
    try {
      // 1. Save contest info
      const infoRes = await apiFetch<{ success: boolean; message: string }>(`/contests/${editingId}`, {
        method: 'PUT',
        body: JSON.stringify({
          name: editName,
          start_time: editStartTime,
          end_time: editEndTime,
          description: editDescription,
          link: editLink || undefined,
        }),
      })
      if (!infoRes.success) { setError(infoRes.message); setEditSaving(false); return }

      // 2. Save problem order
      const problemIds = editProblems.map(p => p.id)
      const orderRes = await apiFetch<{ success: boolean; message: string }>(
        `/contests/${editingId}/problem-order`,
        { method: 'POST', body: JSON.stringify({ problem_ids: problemIds }) },
      )
      if (!orderRes.success) { setError(orderRes.message); setEditSaving(false); return }

      cancelEdit()
      loadContests()
    } catch (err) {
      setError(`保存失败: ${err}`)
    } finally {
      setEditSaving(false)
    }
  }

  const handleToggleStatus = async (contest: ContestItem) => {
    const currentStatus = contest.status
    const newStatus = currentStatus === 'public' ? 'draft' : 'public'
    const label = newStatus === 'public' ? '公开' : '取消公开'
    if (newStatus === 'public' && !contest.link) {
      const url = prompt('请输入比赛外部链接（如 https://codeforces.com/...）：', 'https://www.youtube.com/watch?v=dQw4w9WgXcQ')
      if (!url) return
      if (!window.confirm(`确定要公开此比赛吗？`)) return
      try {
        const res = await apiFetch<{ success: boolean; message: string }>(`/contests/${contest.id}/status`, {
          method: 'POST',
          body: JSON.stringify({ status: newStatus, link: url }),
        })
        if (!res.success) { alert(res.message); return }
        loadContests()
      } catch (err) { alert(`操作失败: ${err}`) }
      return
    }
    if (!window.confirm(`确定要${label}此比赛吗？`)) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/contests/${contest.id}/status`, {
        method: 'POST',
        body: JSON.stringify({ status: newStatus }),
      })
      if (!res.success) { alert(res.message); return }
      loadContests()
    } catch (err) { alert(`操作失败: ${err}`) }
  }

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800">比赛</h1>
        <button
          onClick={() => { setShowForm(!showForm); setEditingId(null); setError('') }}
          className={`px-4 py-2 text-sm ${isAdmin ? 'bg-gray-800 text-white hover:bg-gray-700' : 'bg-gray-200 text-gray-400 cursor-not-allowed'}`}
          disabled={!isAdmin}
        >
          {showForm ? '收起' : '创建比赛'}
        </button>
      </div>

      {error && <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm">{error}</div>}

      {showForm && (
        <form onSubmit={handleCreate} className="bg-white border border-gray-300 p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4 text-gray-700">创建新比赛</h2>
          <div className="grid grid-cols-3 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium mb-1 text-gray-700">比赛名称 *</label>
              <input type="text" value={name} onChange={e => setName(e.target.value)} required
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"
                placeholder="如：2026春季周赛" />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1 text-gray-700">开始时间 *</label>
              <input type="text" value={startTime} onChange={e => setStartTime(e.target.value)} required
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"
                placeholder="如：2026-05-01 10:00" />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1 text-gray-700">结束时间 *</label>
              <input type="text" value={endTime} onChange={e => setEndTime(e.target.value)} required
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"
                placeholder="如：2026-05-01 12:00" />
            </div>
          </div>
          <div className="mb-4">
            <label className="block text-sm font-medium mb-1 text-gray-700">简介</label>
            <textarea value={description} onChange={e => setDescription(e.target.value)}
              rows={3}
              className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"
              placeholder="比赛简介（可选）" />
          </div>
          <div className="mb-4">
            <label className="block text-sm font-medium mb-1 text-gray-700">链接（设为公开时必填）</label>
            <input type="url" value={link} onChange={e => setLink(e.target.value)}
              placeholder="https://..."
              className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
          </div>
          <button type="submit" className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700">创建</button>
        </form>
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 mb-6">
        {[
          { id: 'all' as TabId, label: '全部', count: contests.length },
          { id: 'public' as TabId, label: '已公开', count: counts.public },
          { id: 'draft' as TabId, label: '未公开', count: counts.draft },
        ].map(tab => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-gray-800 text-gray-900'
                : 'border-transparent text-gray-500 hover:text-gray-800'
            }`}
          >
            {tab.label}
            <span className={`ml-1.5 px-1.5 py-0.5 text-xs rounded ${
              activeTab === tab.id ? 'bg-gray-800 text-white' : 'bg-gray-200 text-gray-600'
            }`}>
              {tab.count}
            </span>
          </button>
        ))}
      </div>

      {loading ? (
        <div className="text-center py-12 text-gray-400">加载中...</div>
      ) : filteredContests.length === 0 ? (
        <div className="text-center py-12 text-gray-400">暂无比赛</div>
      ) : (
        <div className="space-y-3">
          {filteredContests.map(c => (
            <div key={c.id} className="bg-white border border-gray-300 p-4">
              {editingId === c.id ? (
                <div>
                  <h3 className="font-semibold text-gray-800 mb-4">编辑比赛 — {c.name}</h3>

                  {/* Contest info fields */}
                  <div className="grid grid-cols-3 gap-4 mb-4">
                    <div>
                      <label className="block text-xs font-medium mb-1 text-gray-600">比赛名称</label>
                      <input type="text" value={editName} onChange={e => setEditName(e.target.value)} required
                        className="w-full px-3 py-1.5 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
                    </div>
                    <div>
                      <label className="block text-xs font-medium mb-1 text-gray-600">开始时间</label>
                      <input type="text" value={editStartTime} onChange={e => setEditStartTime(e.target.value)} required
                        className="w-full px-3 py-1.5 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
                    </div>
                    <div>
                      <label className="block text-xs font-medium mb-1 text-gray-600">结束时间</label>
                      <input type="text" value={editEndTime} onChange={e => setEditEndTime(e.target.value)} required
                        className="w-full px-3 py-1.5 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
                    </div>
                  </div>
                  <div className="mb-4">
                    <label className="block text-xs font-medium mb-1 text-gray-600">链接</label>
                    <input type="url" value={editLink} onChange={e => setEditLink(e.target.value)}
                      placeholder="https://..."
                      className="w-full px-3 py-1.5 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
                  </div>
                  <div className="mb-6">
                    <label className="block text-xs font-medium mb-1 text-gray-600">简介</label>
                    <textarea value={editDescription} onChange={e => setEditDescription(e.target.value)}
                      rows={2}
                      className="w-full px-3 py-1.5 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
                  </div>

                  {/* Problem order section */}
                  <div className="border-t border-gray-200 pt-4 mb-6">
                    <h4 className="text-sm font-semibold text-gray-700 mb-3">题目顺序</h4>
                    {!editProblemsReady ? (
                      <div className="text-sm text-gray-400">加载题目中...</div>
                    ) : editProblems.length === 0 ? (
                      <div className="text-sm text-gray-400">该比赛暂无题目</div>
                    ) : (
                      <div className="space-y-1.5">
                        {editProblems.map((p, idx) => (
                          <div
                            key={p.id}
                            className="flex items-center gap-3 p-2 bg-gray-50 border border-gray-200 text-sm"
                          >
                            <span className="w-6 text-center font-mono text-gray-500">{idx + 1}</span>
                            <span className="flex-1 text-gray-800">{p.title}</span>
                            <span className="text-xs text-gray-500">{p.author_name}</span>
                            <span className="text-xs text-gray-500"><DiffBadge difficulty={p.difficulty} map={difficultyMap} /></span>
                            <button
                              onClick={() => moveProblem(idx, -1)}
                              disabled={idx === 0}
                              className="px-2 py-1 text-xs border border-gray-300 text-gray-600 hover:bg-gray-200 disabled:opacity-30 disabled:cursor-not-allowed"
                            >↑</button>
                            <button
                              onClick={() => moveProblem(idx, 1)}
                              disabled={idx === editProblems.length - 1}
                              className="px-2 py-1 text-xs border border-gray-300 text-gray-600 hover:bg-gray-200 disabled:opacity-30 disabled:cursor-not-allowed"
                            >↓</button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Actions */}
                  <div className="flex gap-2">
                    <button
                      onClick={handleSaveEdit}
                      disabled={editSaving || !editProblemsReady}
                      className="px-5 py-2 text-sm bg-gray-800 text-white hover:bg-gray-700 disabled:opacity-50"
                    >
                      {editSaving ? '保存中...' : '保存全部'}
                    </button>
                    <button
                      type="button"
                      onClick={cancelEdit}
                      className="px-5 py-2 text-sm border border-gray-300 text-gray-600 hover:bg-gray-100"
                    >
                      取消
                    </button>
                  </div>
                </div>
              ) : (
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <h3 className="font-semibold text-gray-800">{c.name}
                      <span className={`ml-2 px-2 py-0.5 text-xs font-medium ${c.status === 'public' ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'}`}>
                        {c.status === 'public' ? '已公开' : '未公开'}
                      </span>
                    </h3>
                    <div className="text-sm text-gray-500 mt-1">
                      {c.start_time} ~ {c.end_time}
                    </div>
                    {c.link && (
                      <div className="text-sm mt-1">
                        <a href={c.link} target="_blank" rel="noopener noreferrer" className="text-blue-600 underline hover:text-blue-800">外部链接 ↗</a>
                      </div>
                    )}
                    {c.description && (
                      <div className="text-sm text-gray-600 mt-1">{c.description}</div>
                    )}
                    <div className="text-xs text-gray-400 mt-1">创建于 {c.created_at}</div>
                  </div>
                  <div className="flex gap-2 shrink-0">
                    {isAdmin && (<>
                    <button
                      onClick={() => handleToggleStatus(c)}
                      className={`px-3 py-1.5 text-xs border ${c.status === 'public' ? 'border-yellow-500 text-yellow-700 hover:bg-yellow-50' : 'border-green-500 text-green-700 hover:bg-green-50'}`}
                    >
                      {c.status === 'public' ? '取消公开' : '公开'}
                    </button>
                    <button
                      onClick={() => startEdit(c)}
                      className="px-3 py-1.5 text-xs border border-gray-300 text-gray-600 hover:bg-gray-100"
                    >
                      编辑
                    </button>
                    <button
                      onClick={() => handleDelete(c.id, c.name)}
                      className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50"
                    >
                      删除
                    </button>
                    </>)}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
