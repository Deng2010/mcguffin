import { useState, useEffect, useMemo } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import { useDifficulties, DiffBadge } from '../hooks/useDifficulties'
import type { ProblemListItem, AdminPendingProblem } from '../types'

interface TeamMemberOption {
  user_id: string
  name: string
}

interface ContestOption {
  id: string
  name: string
}

type TabId = 'list' | 'pending' | 'approved' | 'published'

export default function ProblemsPage() {
  const { user, hasPermission } = useAuth()
  const { difficultyMap, difficulties } = useDifficulties()
  const navigate = useNavigate()
  const canApprove = hasPermission('approve_problem')
  const canSubmit = hasPermission('submit_problem')

  // All problems tab
  const [problems, setProblems] = useState<ProblemListItem[]>([])
  const [loading, setLoading] = useState(true)

  // Review tabs (admin only)
  const [pendingProblems, setPendingProblems] = useState<AdminPendingProblem[]>([])
  const [approvedProblems, setApprovedProblems] = useState<AdminPendingProblem[]>([])
  const [publishedProblems, setPublishedProblems] = useState<AdminPendingProblem[]>([])
  const [members, setMembers] = useState<TeamMemberOption[]>([])
  const [contests, setContests] = useState<ContestOption[]>([])
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [visibilityMap, setVisibilityMap] = useState<Record<string, string[]>>({})

  // Submit form state
  const [showSubmit, setShowSubmit] = useState(false)
  const [formTitle, setFormTitle] = useState('')
  const [contestMode, setContestMode] = useState<'none' | 'select' | 'custom'>('none')
  const [selectedContestId, setSelectedContestId] = useState('')
  const [customContest, setCustomContest] = useState('')
  const [formDifficulty, setFormDifficulty] = useState<string>('Medium')
  const [formContent, setFormContent] = useState('')
  const [formSolution, setFormSolution] = useState('')
  const [submitted, setSubmitted] = useState(false)
  const [formError, setFormError] = useState('')

  // Search & filter state
  const [searchText, setSearchText] = useState('')
  const [filterDifficulty, setFilterDifficulty] = useState('')
  const [filterStatus, setFilterStatus] = useState('')
  const [filterAuthor, setFilterAuthor] = useState('')

  const [activeTab, setActiveTab] = useState<TabId>('list')

  const tabs: { id: TabId; label: string; count?: number }[] = [
    { id: 'list', label: '全部题目' },
  ]
  if (canApprove) {
    tabs.push(
      { id: 'pending', label: '待审核', count: pendingProblems.length },
      { id: 'approved', label: '已通过', count: approvedProblems.length },
      { id: 'published', label: '已发布', count: publishedProblems.length },
    )
  }

  const loadProblems = () => {
    const url = canApprove ? '/problems?all=true' : '/problems'
    apiFetch<ProblemListItem[]>(url)
      .then(setProblems)
      .catch(() => setProblems([]))
      .finally(() => setLoading(false))
  }

  const loadReviewData = () => {
    Promise.all([
      apiFetch<AdminPendingProblem[]>('/problems/admin/pending'),
      apiFetch<AdminPendingProblem[]>('/problems?all=true'),
      apiFetch<TeamMemberOption[]>('/problems/admin/members'),
      apiFetch<ContestOption[]>('/contests'),
    ]).then(([pendingList, allList, memberList, contestList]) => {
      setPendingProblems(pendingList)
      setApprovedProblems(allList.filter((p: AdminPendingProblem) => p.status === 'approved'))
      setPublishedProblems(allList.filter((p: AdminPendingProblem) => p.status === 'published'))
      setMembers(memberList)
      setContests(contestList)
      const vm: Record<string, string[]> = {}
      pendingList.forEach((p: AdminPendingProblem) => { vm[p.id] = p.visible_to || [] })
      setVisibilityMap(vm)
    }).catch(() => {})
  }

  useEffect(() => {
    loadProblems()
    if (canApprove) loadReviewData()
  }, [canApprove])

  // Load contests when submit form opens
  useEffect(() => {
    if (showSubmit) {
      apiFetch<ContestOption[]>('/contests').then(setContests).catch(() => {})
    }
  }, [showSubmit])

  // Client-side filtering
  const filteredProblems = useMemo(() => {
    const q = searchText.toLowerCase().trim()
    const a = filterAuthor.toLowerCase().trim()
    return problems.filter(p => {
      if (q && !p.title.toLowerCase().includes(q)) return false
      if (filterDifficulty && p.difficulty !== filterDifficulty) return false
      if (filterStatus && p.status !== filterStatus) return false
      if (a && !p.author_name.toLowerCase().includes(a)) return false
      return true
    })
  }, [problems, searchText, filterDifficulty, filterStatus, filterAuthor])

  // ====== Actions ======

  const handleClaim = async (problemId: string) => {
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/problems/claim/${problemId}`, { method: 'POST' })
      if (!res.success) { alert(res.message); return }
      loadProblems()
    } catch (err) { alert(`认领失败: ${err}`) }
  }

  const handleUnclaim = async (problemId: string) => {
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/problems/unclaim/${problemId}`, { method: 'POST' })
      if (!res.success) { alert(res.message); return }
      loadProblems()
    } catch (err) { alert(`取消认领失败: ${err}`) }
  }

  const handleReview = async (problemId: string, action: string) => {
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/problems/review/${problemId}/${action}`, { method: 'POST' })
      if (!res.success) { alert(res.message); return }
      loadReviewData()
      loadProblems()
    } catch (err) { alert(`操作失败: ${err}`) }
  }

  const handleSetVisibility = async (problemId: string) => {
    const ids = visibilityMap[problemId] || []
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/problems/visibility/${problemId}`,
        { method: 'POST', body: JSON.stringify({ user_ids: ids }) },
      )
      if (!res.success) { alert(res.message); return }
      alert('可见性已更新')
    } catch (err) { alert(`设置失败: ${err}`) }
  }

  const handleSetContest = async (problemId: string, contestId: string) => {
    const payload = contestId ? { contest_id: contestId } : { contest_id: null }
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/problems/contest/${problemId}`,
        { method: 'POST', body: JSON.stringify(payload) },
      )
      if (!res.success) { alert(res.message); return }
      loadReviewData()
      loadProblems()
    } catch (err) { alert(`设置失败: ${err}`) }
  }

  const handleDelete = async (problemId: string, title: string) => {
    if (!window.confirm(`确定要永久删除题目「${title}」吗？此操作不可撤销。`)) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/problems/${problemId}`,
        { method: 'DELETE' },
      )
      if (!res.success) { alert(res.message); return }
      loadReviewData()
      loadProblems()
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  const toggleMember = (problemId: string, userId: string) => {
    setVisibilityMap(prev => {
      const current = prev[problemId] || []
      const updated = current.includes(userId)
        ? current.filter(id => id !== userId)
        : [...current, userId]
      return { ...prev, [problemId]: updated }
    })
  }

  const resetFilters = () => {
    setSearchText('')
    setFilterDifficulty('')
    setFilterStatus('')
    setFilterAuthor('')
  }

  // ====== Submit form ======

  const getContestName = (): string => {
    if (contestMode === 'select' && selectedContestId) {
      const found = contests.find(c => c.id === selectedContestId)
      return found?.name || ''
    }
    if (contestMode === 'custom') return customContest
    return ''
  }

  const getContestId = (): string | undefined => {
    if (contestMode === 'select' && selectedContestId) return selectedContestId
    return undefined
  }

  const handleSubmitProblem = async (e: React.FormEvent) => {
    e.preventDefault()
    const contest = getContestName()
    const contest_id = getContestId()
    try {
      await apiFetch('/problems', {
        method: 'POST',
        body: JSON.stringify({
          title: formTitle,
          contest,
          contest_id,
          difficulty: formDifficulty,
          content: formContent,
          solution: formSolution.trim() ? formSolution : undefined,
        }),
      })
      setSubmitted(true)
      setFormError('')
      setTimeout(() => {
        setSubmitted(false)
        setShowSubmit(false)
        setFormTitle('')
        setFormContent('')
        setFormSolution('')
        setContestMode('none')
        setSelectedContestId('')
        setCustomContest('')
        setFormDifficulty('Medium')
        loadProblems()
      }, 2000)
    } catch (err) {
      setFormError(`${err}`)
    }
  }

  // ====== Shared helpers ======

  const statusBadge = (s: string) => {
    switch (s) {
      case 'pending': return <span className="px-2 py-0.5 text-xs bg-yellow-100 text-yellow-700">待审核</span>
      case 'approved': return <span className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700">已通过</span>
      case 'published': return <span className="px-2 py-0.5 text-xs bg-green-100 text-green-700">已发布</span>
      case 'rejected': return <span className="px-2 py-0.5 text-xs bg-red-100 text-red-700">已拒绝</span>
      default: return s
    }
  }

  // Shared meta info row for all card types
  const renderMeta = (p: ProblemListItem | AdminPendingProblem) => (
    <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-gray-500 mt-1">
      <span>作者：{p.author_name}</span>
      <span>赛事：{p.contest || '无'}</span>
      <span>难度：<DiffBadge difficulty={p.difficulty} map={difficultyMap} /></span>
      {p.status && <span>状态：{statusBadge(p.status)}</span>}
      {'has_verifier_solution' in p && (p as any).has_verifier_solution && (
        <span className="text-purple-600 font-medium">已有验题人题解</span>
      )}
    </div>
  )

  // Card wrapper — clickable to navigate to problem detail
  const cardClass = "p-4 bg-white border border-gray-300 cursor-pointer hover:bg-gray-50 transition-colors"
  const goDetail = (problemId: string) => (e: React.MouseEvent) => {
    navigate(`/problems/${problemId}`)
  }

  // ====== Search & Filter Bar ======

  const renderFilterBar = () => {
    const hasActiveFilters = searchText || filterDifficulty || filterStatus || filterAuthor
    return (
      <div className="bg-white border border-gray-300 p-4 mb-4 space-y-3">
        {/* Search row */}
        <div className="flex items-center gap-2">
          <svg className="w-4 h-4 text-gray-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="text"
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
            placeholder="搜索题目名称..."
            className="flex-1 px-3 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
          />
        </div>

        {/* Filter row */}
        <div className="flex flex-wrap items-center gap-3">
          {/* Difficulty */}
          <div className="flex items-center gap-1.5">
            <label className="text-xs text-gray-500 font-medium">难度</label>
            <select
              value={filterDifficulty}
              onChange={e => setFilterDifficulty(e.target.value)}
              className="px-2 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
            >
              <option value="">全部</option>
              {difficulties.map(d => (
                <option key={d.name} value={d.name}>{d.label}</option>
              ))}
            </select>
          </div>

          {/* Status (admin only) */}
          {canApprove && (
            <div className="flex items-center gap-1.5">
              <label className="text-xs text-gray-500 font-medium">状态</label>
              <select
                value={filterStatus}
                onChange={e => setFilterStatus(e.target.value)}
                className="px-2 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
              >
                <option value="">全部</option>
                <option value="published">已发布</option>
                <option value="approved">已通过</option>
                <option value="pending">待审核</option>
                <option value="rejected">已拒绝</option>
              </select>
            </div>
          )}

          {/* Author */}
          <div className="flex items-center gap-1.5">
            <label className="text-xs text-gray-500 font-medium">作者</label>
            <input
              type="text"
              value={filterAuthor}
              onChange={e => setFilterAuthor(e.target.value)}
              placeholder="作者名..."
              className="w-28 px-2 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
            />
          </div>

          {/* Reset */}
          {hasActiveFilters && (
            <button
              onClick={resetFilters}
              className="px-2 py-1.5 text-xs text-gray-500 hover:text-gray-800 hover:bg-gray-100 border border-gray-200"
            >
              清除筛选
            </button>
          )}
        </div>
      </div>
    )
  }

  // ====== Submit Form ======

  const renderSubmitForm = () => {
    if (!canSubmit) {
      return (
        <div className="text-center py-12">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">投稿题目</h2>
          <p className="text-gray-600 mb-6">只有团队成员才能投稿题目</p>
          {user?.team_status === 'pending' ? (
            <div className="bg-yellow-50 border border-yellow-300 p-4 max-w-md mx-auto"><p className="text-yellow-700">您的入队申请正在审核中...</p></div>
          ) : (
            <Link to="/apply" className="inline-block px-6 py-3 bg-gray-800 text-white font-medium hover:bg-gray-700">申请加入团队</Link>
          )}
        </div>
      )
    }

    return (
      <form onSubmit={handleSubmitProblem} className="max-w-3xl">
        {formError && <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm">{formError}</div>}
        {submitted && (
          <div className="mb-4 p-3 bg-green-50 border border-green-300 text-green-700 text-sm">提交成功！题目已进入审核流程。</div>
        )}

        <div className="mb-4">
          <label className="block text-sm font-medium mb-2 text-gray-700">题目标题</label>
          <input type="text" value={formTitle} onChange={e => setFormTitle(e.target.value)} required
            className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500"
            placeholder="请输入题目标题" disabled={submitted} />
        </div>

        <div className="grid grid-cols-2 gap-4 mb-4">
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700">比赛/赛事</label>
            <div className="space-y-2">
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input type="radio" name="contestMode" checked={contestMode === 'none'}
                  onChange={() => setContestMode('none')} disabled={submitted} className="accent-gray-800" />
                <span className="text-gray-600">无</span>
              </label>
              {contests.length > 0 && (
                <label className="flex items-center gap-2 text-sm cursor-pointer">
                  <input type="radio" name="contestMode" checked={contestMode === 'select'}
                    onChange={() => setContestMode('select')} disabled={submitted} className="accent-gray-800" />
                  <span className="text-gray-600">从已有比赛选择</span>
                </label>
              )}
              {contestMode === 'select' && (
                <select
                  value={selectedContestId}
                  onChange={e => setSelectedContestId(e.target.value)}
                  disabled={submitted}
                  className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm ml-6"
                >
                  <option value="">-- 选择比赛 --</option>
                  {contests.map(c => (
                    <option key={c.id} value={c.id}>{c.name}</option>
                  ))}
                </select>
              )}
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input type="radio" name="contestMode" checked={contestMode === 'custom'}
                  onChange={() => setContestMode('custom')} disabled={submitted} className="accent-gray-800" />
                <span className="text-gray-600">自行输入</span>
              </label>
              {contestMode === 'custom' && (
                <input type="text" value={customContest} onChange={e => setCustomContest(e.target.value)}
                  disabled={submitted}
                  className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm ml-6"
                  placeholder="如：LeetCode周赛" />
              )}
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700">难度</label>
            <select value={formDifficulty} onChange={e => setFormDifficulty(e.target.value)}
              disabled={submitted}
              className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500">
              {difficulties.map(d => (
                <option key={d.name} value={d.name}>{d.label}</option>
              ))}
            </select>
          </div>
        </div>

        <div className="mb-6">
          <label className="block text-sm font-medium mb-2 text-gray-700">题目内容 (Markdown)</label>
          <textarea value={formContent} onChange={e => setFormContent(e.target.value)} required
            rows={15} disabled={submitted}
            className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 font-mono text-sm"
            placeholder="# 题目描述&#10;&#10;请在这里编写题目..." />
        </div>

        <div className="mb-6">
          <label className="block text-sm font-medium mb-2 text-gray-700">
            题解 (Markdown) <span className="text-gray-400 font-normal">— 可选</span>
          </label>
          <textarea value={formSolution} onChange={e => setFormSolution(e.target.value)}
            rows={10} disabled={submitted}
            className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 font-mono text-sm"
            placeholder="# 题解&#10;&#10;请在这里编写题解（可选）..." />
        </div>

        <button type="submit" disabled={submitted}
          className="px-6 py-3 bg-gray-800 text-white font-medium border border-gray-900 hover:bg-gray-700 disabled:opacity-50">
          {submitted ? '提交成功!' : '提交题目'}
        </button>
      </form>
    )
  }

  // ====== Tab: All Problems ======

  const renderProblemList = () => {
    if (loading) return <div className="text-center py-12 text-gray-400">加载中...</div>
    return (
      <>
        {renderFilterBar()}
        {filteredProblems.length === 0 ? (
          <div className="text-center py-12 text-gray-400">
            {searchText || filterDifficulty || filterStatus || filterAuthor
              ? '没有符合条件的题目'
              : '暂无题目'}
          </div>
        ) : (
          <div className="space-y-4">
            {filteredProblems.map(p => (
              <div key={p.id} className={cardClass} onClick={goDetail(p.id)}>
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-lg font-semibold text-gray-800 truncate">{p.title}</span>
                      {statusBadge(p.status)}
                      {p.claimed_by && (
                        <span className="text-xs px-2 py-0.5 bg-purple-100 text-purple-700 font-medium">已认领</span>
                      )}
                      {p.has_verifier_solution && (
                        <span className="text-xs px-2 py-0.5 bg-purple-50 text-purple-500 font-medium">有验题题解</span>
                      )}
                    </div>
                    {renderMeta(p)}
                  </div>
                  <div className="flex items-center gap-2 ml-4 shrink-0" onClick={e => e.stopPropagation()}>
                    {canSubmit && p.status === 'approved' && !p.claimed_by && user?.id !== p.author_id && (
                      <button onClick={() => handleClaim(p.id)} className="px-3 py-1.5 text-xs bg-white border border-gray-300 text-gray-700 hover:bg-gray-100">认领验题</button>
                    )}
                    {canSubmit && p.claimed_by === user?.id && (
                      <button onClick={() => handleUnclaim(p.id)} className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50">取消认领</button>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </>
    )
  }

  // ====== Tab: Pending ======

  const renderPending = () => {
    if (pendingProblems.length === 0) return <div className="text-gray-400 text-sm py-8 text-center">暂无待审核题目</div>
    return (
      <div className="space-y-4">
        {pendingProblems.map(p => (
          <div key={p.id} className={cardClass} onClick={goDetail(p.id)}>
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-lg font-semibold text-gray-800 truncate">{p.title}</span>
                  {statusBadge(p.status)}
                </div>
                {renderMeta(p)}
              </div>
              <div className="flex items-center gap-2 ml-4 shrink-0" onClick={e => e.stopPropagation()}>
                <button onClick={() => handleReview(p.id, 'approve')} className="px-3 py-1.5 text-xs bg-gray-800 text-white hover:bg-gray-700">通过</button>
                <button onClick={() => handleReview(p.id, 'reject')} className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50">拒绝</button>
                <button onClick={() => setExpandedId(expandedId === p.id ? null : p.id)} className="px-3 py-1.5 text-xs border border-gray-300 text-gray-600 hover:bg-gray-100">
                  {expandedId === p.id ? '收起' : '详情'}
                </button>
                <button onClick={() => handleDelete(p.id, p.title)} className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50">删除</button>
              </div>
            </div>

            {expandedId === p.id && (
              <div className="mt-4 border-t border-gray-200 pt-4" onClick={e => e.stopPropagation()}>
                <div className="mb-4">
                  <h4 className="text-sm font-semibold text-gray-700 mb-1">题目内容</h4>
                  <div className="text-sm text-gray-800 whitespace-pre-wrap font-mono bg-gray-50 p-3 border border-gray-200 max-h-60 overflow-y-auto">
                    {p.content}
                  </div>
                </div>
                {p.solution && (
                  <div className="mb-4">
                    <h4 className="text-sm font-semibold text-gray-700 mb-1">出题人题解</h4>
                    <div className="text-sm text-gray-800 whitespace-pre-wrap font-mono bg-blue-50 p-3 border border-blue-200 max-h-60 overflow-y-auto">
                      {p.solution}
                    </div>
                  </div>
                )}

                <div className="mb-4">
                  <h4 className="text-sm font-semibold text-gray-700 mb-2">关联比赛</h4>
                  <div className="flex items-center gap-2">
                    <select
                      defaultValue={p.contest || ''}
                      onChange={e => handleSetContest(p.id, e.target.value)}
                      className="px-3 py-1.5 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"
                    >
                      <option value="">无</option>
                      {contests.map(c => (
                        <option key={c.id} value={c.id}>{c.name}</option>
                      ))}
                    </select>
                    <span className="text-xs text-gray-400">选择后自动保存</span>
                  </div>
                </div>

                <div className="mb-2">
                  <h4 className="text-sm font-semibold text-gray-700 mb-2">可见性设置（选择可查看此题目的成员）</h4>
                  <div className="flex flex-wrap gap-2 mb-2">
                    {members.map(m => (
                      <label key={m.user_id} className="flex items-center gap-1.5 text-sm cursor-pointer">
                        <input
                          type="checkbox"
                          checked={(visibilityMap[p.id] || []).includes(m.user_id)}
                          onChange={() => toggleMember(p.id, m.user_id)}
                          className="accent-gray-800"
                        />
                        {m.name}
                      </label>
                    ))}
                  </div>
                  <button onClick={() => handleSetVisibility(p.id)} className="text-xs px-3 py-1 border border-gray-300 text-gray-600 hover:bg-gray-100">保存可见性</button>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    )
  }

  // ====== Tab: Approved ======

  const renderApproved = () => {
    if (approvedProblems.length === 0) return <div className="text-gray-400 text-sm py-8 text-center">暂无待发布题目</div>
    return (
      <div className="space-y-4">
        {approvedProblems.map(p => (
          <div key={p.id} className={cardClass} onClick={goDetail(p.id)}>
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-lg font-semibold text-gray-800 truncate">{p.title}</span>
                  {statusBadge(p.status)}
                </div>
                {renderMeta(p)}
              </div>
              <div className="flex items-center gap-2 ml-4 shrink-0" onClick={e => e.stopPropagation()}>
                <button onClick={() => handleReview(p.id, 'publish')} className="px-4 py-2 text-sm bg-green-700 text-white hover:bg-green-600">发布</button>
                <button onClick={() => handleReview(p.id, 'return')} className="px-3 py-2 text-sm border border-yellow-500 text-yellow-700 hover:bg-yellow-50">退回</button>
                <button onClick={() => handleDelete(p.id, p.title)} className="px-3 py-2 text-sm border border-red-300 text-red-600 hover:bg-red-50">删除</button>
              </div>
            </div>
          </div>
        ))}
      </div>
    )
  }

  // ====== Tab: Published ======

  const renderPublished = () => {
    if (publishedProblems.length === 0) return <div className="text-gray-400 text-sm py-8 text-center">暂无已发布题目</div>
    return (
      <div className="space-y-4">
        {publishedProblems.map(p => (
          <div key={p.id} className={cardClass} onClick={goDetail(p.id)}>
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-lg font-semibold text-gray-800 truncate">{p.title}</span>
                  {statusBadge(p.status)}
                </div>
                {renderMeta(p)}
              </div>
              <div className="flex items-center gap-2 ml-4 shrink-0" onClick={e => e.stopPropagation()}>
                <button onClick={() => handleReview(p.id, 'unpublish')} className="px-4 py-2 text-sm bg-orange-600 text-white hover:bg-orange-500">取消发布</button>
                <button onClick={() => handleDelete(p.id, p.title)} className="px-3 py-2 text-sm border border-red-300 text-red-600 hover:bg-red-50">删除</button>
              </div>
            </div>
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="p-6 max-w-5xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800">题目</h1>
        {canSubmit && !showSubmit && (
          <button
            onClick={() => setShowSubmit(true)}
            className="px-4 py-2 bg-gray-800 text-white text-sm font-medium hover:bg-gray-700"
          >
            投稿题目
          </button>
        )}
        {showSubmit && (
          <button
            onClick={() => setShowSubmit(false)}
            className="px-4 py-2 border border-gray-300 text-gray-600 text-sm hover:bg-gray-100"
          >
            返回列表
          </button>
        )}
      </div>

      {showSubmit ? (
        renderSubmitForm()
      ) : (
        <>
          {/* Tabs */}
          <div className="flex items-center gap-1 border-b border-gray-300 mb-6">
            {tabs.map(tab => (
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
                {tab.count !== undefined && (
                  <span className={`ml-1.5 px-1.5 py-0.5 text-xs rounded ${
                    activeTab === tab.id ? 'bg-gray-800 text-white' : 'bg-gray-200 text-gray-600'
                  }`}>
                    {tab.count}
                  </span>
                )}
              </button>
            ))}
          </div>

          {/* Tab content */}
          {activeTab === 'list' && renderProblemList()}
          {activeTab === 'pending' && renderPending()}
          {activeTab === 'approved' && renderApproved()}
          {activeTab === 'published' && renderPublished()}
        </>
      )}
    </div>
  )
}
