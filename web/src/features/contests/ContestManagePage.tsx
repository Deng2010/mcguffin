import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useAuthStore } from '../../stores/authStore'
import { apiFetch } from '../../services/api'
import MarkdownEditor from '../../components/MarkdownEditor'

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
}

type TabId = 'all' | 'public' | 'draft'

export default function ContestManagePage() {
  const { hasPermission } = useAuthStore()
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
  const [activeTab, setActiveTab] = useState<TabId>('all')
  const [searchText, setSearchText] = useState('')

  const loadContests = () => {
    apiFetch<ContestItem[]>('/contests')
      .then(setContests)
      .catch(() => setContests([]))
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadContests() }, [])

  const filteredContests = contests
    .filter(c => {
      if (activeTab === 'public') return c.status === 'public'
      if (activeTab === 'draft') return c.status === 'draft'
      return true
    })
    .filter(c => {
      const q = searchText.toLowerCase().trim()
      if (!q) return true
      return c.name.toLowerCase().includes(q)
    })
    .sort((a, b) => b.start_time.localeCompare(a.start_time))

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

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">比赛</h1>
        <button
          onClick={() => { setShowForm(!showForm); setError('') }}
          className={`px-4 py-2 text-sm ${isAdmin ? 'bg-gray-800 text-white hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600' : 'bg-gray-200 text-gray-400 cursor-not-allowed dark:bg-gray-700'}`}
          disabled={!isAdmin}
        >
          {showForm ? '收起' : '创建比赛'}
        </button>
      </div>

      {error && <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">{error}</div>}

      {showForm && (
        <form onSubmit={handleCreate} className="mg-box-shadow p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">创建新比赛</h2>
          <div className="grid grid-cols-3 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-200">比赛名称 *</label>
              <input type="text" value={name} onChange={e => setName(e.target.value)} required
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800"
                placeholder="如：2026春季周赛" />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-200">开始时间 *</label>
              <input type="text" value={startTime} onChange={e => setStartTime(e.target.value)} required
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800"
                placeholder="如：2026-05-01 10:00" />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-200">结束时间 *</label>
              <input type="text" value={endTime} onChange={e => setEndTime(e.target.value)} required
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800"
                placeholder="如：2026-05-01 12:00" />
            </div>
          </div>
          <div className="mb-4">
            <MarkdownEditor
              value={description}
              onChange={setDescription}
              label="简介"
              placeholder="比赛简介（可选）"
              rows={10}
            />
          </div>
          <div className="mb-4">
            <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-200">链接（设为公开时必填）</label>
            <input type="url" value={link} onChange={e => setLink(e.target.value)}
              placeholder="https://..."
              className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800" />
          </div>
          <button type="submit" className="mg-btn mg-btn-primary mg-btn-md">创建</button>
        </form>
      )}

      {/* Search */}
      <div className="mb-4">
        <input
          type="text"
          value={searchText}
          onChange={e => setSearchText(e.target.value)}
          placeholder="搜索比赛名称..."
          className="w-full px-4 py-2 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800"
        />
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 mb-6 dark:border-gray-700">
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
                ? 'border-gray-800 text-gray-900 dark:border-gray-100 dark:text-gray-100'
                : 'border-transparent text-gray-500 hover:text-gray-800 dark:hover:text-gray-100'
            }`}
          >
            {tab.label}
            <span className={`ml-1.5 px-1.5 py-0.5 text-xs rounded ${
              activeTab === tab.id ? 'bg-gray-800 text-white dark:bg-gray-700' : 'bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-400'
            }`}>
              {tab.count}
            </span>
          </button>
        ))}
      </div>

      {loading ? (
        <div className="text-center py-12 text-gray-400 dark:text-gray-500">加载中...</div>
      ) : filteredContests.length === 0 ? (
        <div className="text-center py-12 text-gray-400 dark:text-gray-500">暂无比赛</div>
      ) : (
        <div className="space-y-3">
          {filteredContests.map(c => (
            <div key={c.id} className="mg-box-shadow p-4">
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <Link to={`/contests/${c.id}`} className="hover:text-blue-600 dark:hover:text-blue-400">
                    <h3 className="font-semibold text-gray-800 dark:text-gray-100">{c.name}
                      <span className={`ml-2 px-2 py-0.5 text-xs font-medium ${
                        c.status === 'public'
                          ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300'
                          : 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'
                      }`}>
                        {c.status === 'public' ? '已公开' : '未公开'}
                      </span>
                    </h3>
                  </Link>
                  <div className="text-sm text-gray-500 mt-1 dark:text-gray-400">{c.start_time} ~ {c.end_time}</div>
                  {c.link && (
                    <div className="text-sm mt-1">
                      <a href={c.link} target="_blank" rel="noopener noreferrer" className="text-blue-600 underline hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300">外部链接 ↗</a>
                    </div>
                  )}
                  {c.description && <div className="text-sm text-gray-600 mt-1 dark:text-gray-300 line-clamp-2">{c.description}</div>}
                  <div className="text-xs text-gray-400 mt-1 dark:text-gray-500">创建于 {c.created_at}</div>
                </div>
                <div className="flex gap-2 shrink-0 ml-4">
                  <Link to={`/contests/${c.id}`}
                    className="px-3 py-1.5 text-xs border border-gray-300 text-gray-600 hover:bg-gray-100 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-800">
                    查看
                  </Link>
                  {isAdmin && (
                    <button onClick={() => handleDelete(c.id, c.name)}
                      className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20">
                      删除
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
