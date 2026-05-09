import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { useSite } from '../SiteContext'
import { apiFetch } from '../api'
import MarkdownRenderer from '../components/MarkdownRenderer'
import MarkdownEditor from '../components/MarkdownEditor'
import { useDifficulties, DiffBadge } from '../hooks/useDifficulties'
import type { Announcement } from '../types'

// ============== Types ==============

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

interface ProblemItem {
  id: string
  title: string
  author_name: string
  contest: string
  contest_id?: string | null
  difficulty: string
  status: string
  created_at: string
  link?: string | null
}

// ============== Helpers ==============

function contestStatus(start: string, end: string): { label: string; color: string } {
  const now = new Date()
  const s = new Date(start)
  const e = new Date(end)
  if (now < s) return { label: '未开始', color: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300' }
  if (now > e) return { label: '已结束', color: 'bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-400' }
  return { label: '进行中', color: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300' }
}

/** Render a problem card: if link exists, click goes to external URL; otherwise goes to internal detail page */
function ProblemCard({ p, difficultyMap }: { p: ProblemItem; difficultyMap: Map<string, any> }) {
  if (p.link) {
    return (
      <a
        href={p.link}
        target="_blank"
        rel="noopener noreferrer"
        className="block bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-4 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
      >
        <div className="flex items-center justify-between">
          <div>
            <span className="font-medium text-gray-800 dark:text-gray-100">{p.title}</span>
            <div className="flex items-center gap-3 mt-0.5 text-xs text-gray-400 dark:text-gray-500">
              <span>作者：{p.author_name}</span>
              {p.contest && <span>{p.contest}</span>}
            </div>
          </div>
          <div className="flex items-center gap-2">
            <DiffBadge difficulty={p.difficulty} map={difficultyMap} className="px-2 py-0.5 text-xs font-medium" />
          </div>
        </div>
      </a>
    )
  }
  return (
    <Link
      to={`/problems/${p.id}`}
      className="block bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-4 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
    >
      <div className="flex items-center justify-between">
        <div>
          <span className="font-medium text-gray-800 dark:text-gray-100">{p.title}</span>
          <div className="flex items-center gap-3 mt-0.5 text-xs text-gray-400 dark:text-gray-500">
              <span>作者：{p.author_name}</span>
              {p.contest && <span>{p.contest}</span>}
            </div>
        </div>
        <div className="flex items-center gap-2">
          <DiffBadge difficulty={p.difficulty} map={difficultyMap} className="px-2 py-0.5 text-xs font-medium" />
        </div>
      </div>
    </Link>
  )
}

/** Compact problem card (used inside a contest section) */
function CompactProblemCard({ p, difficultyMap }: { p: ProblemItem; difficultyMap: Map<string, any> }) {
  if (p.link) {
    return (
      <a
        href={p.link}
        target="_blank"
        rel="noopener noreferrer"
        className="flex items-center justify-between border border-gray-200 dark:border-gray-700 p-2.5 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
      >
        <span className="text-sm text-gray-800 dark:text-gray-100">{p.title}</span>
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-400 dark:text-gray-500">作者：{p.author_name}</span>
          <DiffBadge difficulty={p.difficulty} map={difficultyMap} className="px-1.5 py-0.5 text-xs" />
        </div>
      </a>
    )
  }
  return (
    <Link
      to={`/problems/${p.id}`}
      className="flex items-center justify-between border border-gray-200 dark:border-gray-700 p-2.5 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
    >
      <span className="text-sm text-gray-800 dark:text-gray-100">{p.title}</span>
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-400 dark:text-gray-500">作者：{p.author_name}</span>
        <DiffBadge difficulty={p.difficulty} map={difficultyMap} className="px-1.5 py-0.5 text-xs" />
      </div>
    </Link>
  )
}

// ============== Main Component ==============

export default function ShowcasePage() {
  const { user, hasPermission } = useAuth()
  const { siteInfo, updateDescription, refresh: refreshSite } = useSite()
  const [allContests, setAllContests] = useState<ContestItem[]>([])
  const [allProblems, setAllProblems] = useState<ProblemItem[]>([])
  const [announcements, setAnnouncements] = useState<Announcement[]>([])
  const [loading, setLoading] = useState(true)

  // Description editing state
  const [editing, setEditing] = useState(false)
  const [draftDescription, setDraftDescription] = useState('')

  // Showcase management state
  const [showcaseMode, setShowcaseMode] = useState(false)
  const [selectedProblemIds, setSelectedProblemIds] = useState<string[]>([])
  const [selectedContestIds, setSelectedContestIds] = useState<string[]>([])
  const [showcaseMsg, setShowcaseMsg] = useState('')
  const [showcaseSaving, setShowcaseSaving] = useState(false)

  const isAdmin = user?.role === 'admin' || user?.role === 'superadmin'
  const { difficultyMap } = useDifficulties()

  useEffect(() => {
    Promise.all([
      apiFetch<ContestItem[]>('/contests').catch(() => [] as ContestItem[]),
      apiFetch<ProblemItem[]>('/problems').catch(() => [] as ProblemItem[]),
      apiFetch<Announcement[]>('/announcements').catch(() => [] as Announcement[]),
    ]).then(([c, p, a]) => {
      setAllContests(c)
      setAllProblems(p)
      setAnnouncements(a)
    }).finally(() => setLoading(false))
  }, [])

  // Sync draft when siteInfo loads or editing opens
  useEffect(() => {
    if (editing && siteInfo) {
      setDraftDescription(siteInfo.description)
    }
  }, [editing, siteInfo])

  // Sync showcase selections from siteInfo
  useEffect(() => {
    if (siteInfo) {
      setSelectedProblemIds(siteInfo.showcase_problem_ids ?? [])
      setSelectedContestIds(siteInfo.showcase_contest_ids ?? [])
    }
  }, [siteInfo])

  const handleSaveDescription = async () => {
    const res = await updateDescription(draftDescription)
    if (!res.success) { alert(res.message); return }
    setEditing(false)
  }

  const loadShowcaseSelections = () => {
    setShowcaseMode(true)
    setSelectedProblemIds(siteInfo?.showcase_problem_ids ?? [])
    setSelectedContestIds(siteInfo?.showcase_contest_ids ?? [])
  }

  const toggleProblem = (id: string) => {
    setSelectedProblemIds(prev => prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id])
  }

  const toggleContest = (id: string) => {
    setSelectedContestIds(prev => prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id])
  }

  const moveItem = (type: 'problem' | 'contest', idx: number, direction: -1 | 1) => {
    const setter = type === 'problem' ? setSelectedProblemIds : setSelectedContestIds
    const list = type === 'problem' ? selectedProblemIds : selectedContestIds
    const target = idx + direction
    if (target < 0 || target >= list.length) return
    setter(prev => {
      const next = [...prev]
      const temp = next[idx]
      next[idx] = next[target]
      next[target] = temp
      return next
    })
  }

  const handleSaveShowcase = async () => {
    setShowcaseSaving(true)
    setShowcaseMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/showcase', {
        method: 'PUT',
        body: JSON.stringify({
          problem_ids: selectedProblemIds,
          contest_ids: selectedContestIds,
        }),
      })
      if (!res.success) { setShowcaseMsg(`保存失败: ${res.message}`); return }
      setShowcaseMsg(res.message)
      setShowcaseMode(false)
      refreshSite()
    } catch (err) {
      setShowcaseMsg(`保存失败: ${err}`)
    } finally {
      setShowcaseSaving(false)
    }
  }

  // Filter: showcase shows ONLY public contests and published problems (for everyone)
  const contests = allContests.filter(c => c.status === 'public')
  const problems = allProblems.filter(p => p.status === 'published')

  // Use showcase selections if configured, otherwise show all
  const currentSelectedProblemIds = siteInfo?.showcase_problem_ids ?? []
  const currentSelectedContestIds = siteInfo?.showcase_contest_ids ?? []

  const showcaseProblems = currentSelectedProblemIds.length > 0
    ? currentSelectedProblemIds.map(id => problems.find(p => p.id === id)).filter(Boolean) as ProblemItem[]
    : problems

  const showcaseContests = currentSelectedContestIds.length > 0
    ? currentSelectedContestIds.map(id => contests.find(c => c.id === id)).filter(Boolean) as ContestItem[]
    : contests

  // Group problems by contest_id, respecting problem_order
  const contestProblems: Record<string, ProblemItem[]> = {}
  const unassigned: ProblemItem[] = []
  for (const p of problems) {
    if (p.contest_id) {
      if (!contestProblems[p.contest_id]) contestProblems[p.contest_id] = []
      contestProblems[p.contest_id].push(p)
    } else {
      unassigned.push(p)
    }
  }
  // Sort within each contest by problem_order
  for (const contest of contests) {
    const cp = contestProblems[contest.id]
    if (cp && contest.problem_order && contest.problem_order.length > 0) {
      const orderMap = new Map(contest.problem_order.map((id, i) => [id, i]))
      cp.sort((a, b) => {
        const ai = orderMap.get(a.id) ?? 999
        const bi = orderMap.get(b.id) ?? 999
        return ai - bi
      })
    }
  }

  if (loading) return <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">加载中...</div>

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-8">
      {/* ===== 团队简介 ===== */}
      <section className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-6">
        <div className="flex items-center justify-between mb-3">
          <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
            {siteInfo?.name || 'McGuffin'}
          </h1>
          <div className="flex gap-2">
            {isAdmin && !editing && (
              <button
                onClick={() => setEditing(true)}
                className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                编辑简介
              </button>
            )}
            {isAdmin && !showcaseMode && (
              <button
                onClick={loadShowcaseSelections}
                className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                展板管理
              </button>
            )}
          </div>
        </div>

        {editing ? (
          <div className="space-y-3">
            <MarkdownEditor
              value={draftDescription}
              onChange={setDraftDescription}
              placeholder="在此输入团队简介..."
              rows={20}
            />
            <div className="flex gap-2">
              <button
                onClick={handleSaveDescription}
                className="px-4 py-2 text-sm bg-gray-800 dark:bg-gray-700 text-white hover:bg-gray-700 dark:hover:bg-gray-600"
              >
                保存
              </button>
              <button
                onClick={() => setEditing(false)}
                className="px-4 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                取消
              </button>
            </div>
          </div>
        ) : (
          <MarkdownRenderer content={siteInfo?.description || (
            isAdmin
              ? '<span class="text-gray-300 italic dark:text-gray-600">点击「编辑简介」添加团队介绍</span>'
              : '<span class="text-gray-300 italic dark:text-gray-600">暂无团队简介</span>'
          )} />
        )}
      </section>

      {/* ===== 展板管理面板 ===== */}
      {showcaseMode && (
        <section className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-5">
          <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-1">展板管理</h2>
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">
            勾选要在首页展示的题目和比赛，按 ↑↓ 调整顺序。选多少就展示多少。不选则全部展示。
          </p>

          {showcaseMsg && (
            <div className={`mb-3 p-2 text-sm border ${
              showcaseMsg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
            }`}>
              {showcaseMsg}
            </div>
          )}

          {/* Problems */}
          <div className="mb-5">
            <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">
              题目 ({selectedProblemIds.length}/{problems.length})
            </h3>
            <div className="space-y-1 max-h-48 overflow-y-auto">
              {problems.map(p => {
                const isSelected = selectedProblemIds.includes(p.id)
                const idx = selectedProblemIds.indexOf(p.id)
                return (
                  <div key={p.id} className="flex items-center gap-2 py-1.5 px-2 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => toggleProblem(p.id)}
                      className="accent-gray-800 dark:accent-gray-400"
                    />
                    <span className={`text-sm flex-1 ${isSelected ? 'text-gray-800 dark:text-gray-100' : 'text-gray-400 dark:text-gray-500'}`}>
                      {p.title}
                    </span>
                    {isSelected && (
                      <div className="flex gap-1 shrink-0">
                        <button onClick={() => moveItem('problem', idx, -1)} disabled={idx === 0}
                          className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1">↑</button>
                        <button onClick={() => moveItem('problem', idx, 1)} disabled={idx === selectedProblemIds.length - 1}
                          className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1">↓</button>
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          </div>

          {/* Contests */}
          <div className="mb-5">
            <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">
              比赛 ({selectedContestIds.length}/{contests.length})
            </h3>
            <div className="space-y-1 max-h-48 overflow-y-auto">
              {contests.map(c => {
                const isSelected = selectedContestIds.includes(c.id)
                const idx = selectedContestIds.indexOf(c.id)
                return (
                  <div key={c.id} className="flex items-center gap-2 py-1.5 px-2 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => toggleContest(c.id)}
                      className="accent-gray-800 dark:accent-gray-400"
                    />
                    <span className={`text-sm flex-1 ${isSelected ? 'text-gray-800 dark:text-gray-100' : 'text-gray-400 dark:text-gray-500'}`}>
                      {c.name}
                    </span>
                    {isSelected && (
                      <div className="flex gap-1 shrink-0">
                        <button onClick={() => moveItem('contest', idx, -1)} disabled={idx === 0}
                          className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1">↑</button>
                        <button onClick={() => moveItem('contest', idx, 1)} disabled={idx === selectedContestIds.length - 1}
                          className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1">↓</button>
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          </div>

          <div className="flex gap-3 items-center">
            <button
              onClick={handleSaveShowcase}
              disabled={showcaseSaving}
              className="px-5 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
            >
              {showcaseSaving ? '保存中...' : '保存展板'}
            </button>
            <button
              onClick={() => setShowcaseMode(false)}
              className="px-5 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
            >
              取消
            </button>
          </div>
        </section>
      )}

      {/* ===== 公告 ===== */}
      {announcements.length > 0 && (
        <section>
          <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">
            公告
            <Link to="/announcements" className="text-xs font-normal text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 ml-2">查看全部</Link>
          </h2>
          <div className="space-y-2">
            {(() => {
              const pinned = announcements.filter(a => a.pinned)
              const showCount = Math.max(pinned.length, 3)
              return announcements.slice(0, showCount).map(a => (
              <div
                key={a.id}
                className={`bg-white dark:bg-gray-900 border ${a.pinned ? 'border-yellow-400' : 'border-gray-300 dark:border-gray-700'} p-4`}
              >
                <div className="flex items-center gap-2 mb-1">
                  {a.pinned && (
                    <span className="text-xs px-1.5 py-0.5 bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border border-yellow-200">置顶</span>
                  )}
                  <span className="font-medium text-gray-800 dark:text-gray-100 text-sm">{a.title}</span>
                </div>
                <div className="text-xs text-gray-400 dark:text-gray-500 mb-2">
                  {a.author_name} · {new Date(a.created_at).toLocaleDateString('zh-CN')}
                </div>
                <div className="text-sm text-gray-600 dark:text-gray-300 prose prose-sm max-w-none">
                  <MarkdownRenderer content={a.content} />
                </div>
              </div>
              ))
            })()}
          </div>
          {announcements.length > Math.max(announcements.filter(a => a.pinned).length, 3) && (
            <div className="text-center mt-4">
              <Link to="/announcements" className="inline-block px-6 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800">
                查看全部公告
              </Link>
            </div>
          )}
        </section>
      )}

      {/* ===== 已发布题目 ===== */}
      <section>
        <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">公开题目 ({problems.length})</h2>
        {showcaseProblems.length === 0 ? (
          <div className="text-gray-400 dark:text-gray-500 text-sm">暂无公开题目</div>
        ) : (
          <>
          <div className="space-y-2">
            {showcaseProblems.map(p => <ProblemCard key={p.id} p={p} difficultyMap={difficultyMap} />)}
          </div>
          {currentSelectedProblemIds.length > 0 && currentSelectedProblemIds.length < problems.length && (
            <div className="text-center mt-4">
              <Link to="/problems" className="inline-block px-6 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800">
                查看全部题目
              </Link>
            </div>
          )}
          </>
        )}
      </section>

      {/* ===== 比赛列表 ===== */}
      <section>
        <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">比赛 ({contests.length})</h2>
        {showcaseContests.length === 0 ? (
          <div className="text-gray-400 dark:text-gray-500 text-sm">暂无比赛</div>
        ) : (
          <>
          <div className="space-y-4">
            {showcaseContests.map(c => {
              const status = contestStatus(c.start_time, c.end_time)
              const cProblems = contestProblems[c.id] || []
              return (
                <div key={c.id} className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-5">
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <h3 className="text-lg font-bold text-gray-800 dark:text-gray-100">{c.name}</h3>
                      <div className="flex items-center gap-3 mt-1">
                        <span className="text-xs text-gray-500 dark:text-gray-400">{c.start_time} ~ {c.end_time}</span>
                        <span className={`px-2 py-0.5 text-xs font-medium ${status.color}`}>{status.label}</span>
                      </div>
                    </div>
                    {c.link && (
                      <a
                        href={c.link}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="shrink-0 px-3 py-1.5 text-xs border border-blue-300 dark:border-blue-800 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded"
                      >
                        进入比赛 ↗
                      </a>
                    )}
                  </div>
                  {c.description && (
                    <MarkdownRenderer content={c.description} className="mb-3" />
                  )}
                  {cProblems.length > 0 ? (
                    <div className="space-y-1.5">
                      {cProblems.map(p => <CompactProblemCard key={p.id} p={p} difficultyMap={difficultyMap} />)}
                    </div>
                  ) : (
                    <div className="text-sm text-gray-400 dark:text-gray-500">暂无题目</div>
                  )}
                </div>
              )
            })}
          </div>
          {currentSelectedContestIds.length > 0 && currentSelectedContestIds.length < contests.length && (
            <div className="text-center mt-4">
              <Link to="/contests" className="inline-block px-6 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800">
                查看全部比赛
              </Link>
            </div>
          )}
          </>
        )}
      </section>

      {/* ===== 未关联比赛的已发布题目 ===== */}
      {unassigned.length > 0 && showcaseProblems.length > 0 && (
        <section>
          <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">其他公示题目 ({unassigned.length})</h2>
          <div className="grid gap-2">
            {unassigned.filter(p => showcaseProblems.some(sp => sp.id === p.id)).map(p => <ProblemCard key={p.id} p={p} difficultyMap={difficultyMap} />)}
          </div>
        </section>
      )}
    </div>
  )
}
