import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { useSite } from '../SiteContext'
import { apiFetch } from '../api'
import MarkdownRenderer from '../components/MarkdownRenderer'
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
  if (now < s) return { label: '未开始', color: 'bg-blue-100 text-blue-700' }
  if (now > e) return { label: '已结束', color: 'bg-gray-200 text-gray-500' }
  return { label: '进行中', color: 'bg-green-100 text-green-700' }
}

/** Render a problem card: if link exists, click goes to external URL; otherwise goes to internal detail page */
function ProblemCard({ p, difficultyMap }: { p: ProblemItem; difficultyMap: Map<string, any> }) {
  if (p.link) {
    return (
      <a
        href={p.link}
        target="_blank"
        rel="noopener noreferrer"
        className="block bg-white border border-gray-300 p-4 hover:bg-gray-50 transition-colors"
      >
        <div className="flex items-center justify-between">
          <div>
            <span className="font-medium text-gray-800">{p.title}</span>
            <div className="flex items-center gap-3 mt-0.5 text-xs text-gray-400">
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
      className="block bg-white border border-gray-300 p-4 hover:bg-gray-50 transition-colors"
    >
      <div className="flex items-center justify-between">
        <div>
          <span className="font-medium text-gray-800">{p.title}</span>
          <div className="flex items-center gap-3 mt-0.5 text-xs text-gray-400">
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
        className="flex items-center justify-between border border-gray-200 p-2.5 hover:bg-gray-50 transition-colors"
      >
        <span className="text-sm text-gray-800">{p.title}</span>
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-400">作者：{p.author_name}</span>
          <DiffBadge difficulty={p.difficulty} map={difficultyMap} className="px-1.5 py-0.5 text-xs" />
        </div>
      </a>
    )
  }
  return (
    <Link
      to={`/problems/${p.id}`}
      className="flex items-center justify-between border border-gray-200 p-2.5 hover:bg-gray-50 transition-colors"
    >
      <span className="text-sm text-gray-800">{p.title}</span>
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-400">作者：{p.author_name}</span>
        <DiffBadge difficulty={p.difficulty} map={difficultyMap} className="px-1.5 py-0.5 text-xs" />
      </div>
    </Link>
  )
}

// ============== Main Component ==============

export default function ShowcasePage() {
  const { user, hasPermission } = useAuth()
  const { siteInfo, updateDescription } = useSite()
  const [allContests, setAllContests] = useState<ContestItem[]>([])
  const [allProblems, setAllProblems] = useState<ProblemItem[]>([])
  const [announcements, setAnnouncements] = useState<Announcement[]>([])
  const [loading, setLoading] = useState(true)

  // Description editing state
  const [editing, setEditing] = useState(false)
  const [draftDescription, setDraftDescription] = useState('')

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

  const handleSaveDescription = async () => {
    const res = await updateDescription(draftDescription)
    if (!res.success) { alert(res.message); return }
    setEditing(false)
  }

  // Filter: showcase shows ONLY public contests and published problems (for everyone)
  const contests = allContests.filter(c => c.status === 'public')
  const problems = allProblems.filter(p => p.status === 'published')

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

  if (loading) return <div className="p-6 text-center py-12 text-gray-400">加载中...</div>

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-8">
      {/* ===== 团队简介 ===== */}
      <section className="bg-white border border-gray-300 p-6">
        <div className="flex items-center justify-between mb-3">
          <h1 className="text-2xl font-bold text-gray-800">
            {siteInfo?.name || 'McGuffin'}
          </h1>
          {isAdmin && !editing && (
            <button
              onClick={() => setEditing(true)}
              className="text-xs px-3 py-1 border border-gray-300 text-gray-500 hover:bg-gray-100"
            >
              编辑简介
            </button>
          )}
        </div>

        {editing ? (
          <div className="space-y-3">
            <textarea
              value={draftDescription}
              onChange={e => setDraftDescription(e.target.value)}
              placeholder="在此输入团队简介..."
              rows={5}
              className="w-full border border-gray-300 p-3 text-sm text-gray-700 resize-y focus:outline-none focus:border-gray-500"
            />
            <div className="flex gap-2">
              <button
                onClick={handleSaveDescription}
                className="px-4 py-2 text-sm bg-gray-800 text-white hover:bg-gray-700"
              >
                保存
              </button>
              <button
                onClick={() => setEditing(false)}
                className="px-4 py-2 text-sm border border-gray-300 text-gray-600 hover:bg-gray-100"
              >
                取消
              </button>
            </div>
          </div>
        ) : (
          <MarkdownRenderer content={siteInfo?.description || (
            isAdmin
              ? '<span class="text-gray-300 italic">点击「编辑简介」添加团队介绍</span>'
              : '<span class="text-gray-300 italic">暂无团队简介</span>'
          )} />
        )}
      </section>

      {/* ===== 公告 ===== */}
      {announcements.length > 0 && (
        <section>
          <h2 className="text-lg font-semibold mb-4 text-gray-700">
            公告
            <Link to="/announcements" className="text-xs font-normal text-gray-400 hover:text-gray-600 ml-2">查看全部</Link>
          </h2>
          <div className="space-y-2">
            {announcements.slice(0, 3).map(a => (
              <div
                key={a.id}
                className={`bg-white border ${a.pinned ? 'border-yellow-400' : 'border-gray-300'} p-4`}
              >
                <div className="flex items-center gap-2 mb-1">
                  {a.pinned && (
                    <span className="text-xs px-1.5 py-0.5 bg-yellow-100 text-yellow-700 border border-yellow-200">置顶</span>
                  )}
                  <span className="font-medium text-gray-800 text-sm">{a.title}</span>
                </div>
                <div className="text-xs text-gray-400 mb-2">
                  {a.author_name} · {new Date(a.created_at).toLocaleDateString('zh-CN')}
                </div>
                <div className="text-sm text-gray-600 prose prose-sm max-w-none">
                  <MarkdownRenderer content={a.content} />
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* ===== 已发布题目 ===== */}
      <section>
        <h2 className="text-lg font-semibold mb-4 text-gray-700">公开题目 ({problems.length})</h2>
        {problems.length === 0 ? (
          <div className="text-gray-400 text-sm">暂无公开题目</div>
        ) : (
          <div className="space-y-2">
            {problems.map(p => <ProblemCard key={p.id} p={p} difficultyMap={difficultyMap} />)}
          </div>
        )}
      </section>

      {/* ===== 比赛列表 ===== */}
      <section>
        <h2 className="text-lg font-semibold mb-4 text-gray-700">比赛 ({contests.length})</h2>
        {contests.length === 0 ? (
          <div className="text-gray-400 text-sm">暂无比赛</div>
        ) : (
          <div className="space-y-4">
            {contests.map(c => {
              const status = contestStatus(c.start_time, c.end_time)
              const cProblems = contestProblems[c.id] || []
              return (
                <div key={c.id} className="bg-white border border-gray-300 p-5">
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <h3 className="text-lg font-bold text-gray-800">{c.name}</h3>
                      <div className="flex items-center gap-3 mt-1">
                        <span className="text-xs text-gray-500">{c.start_time} ~ {c.end_time}</span>
                        <span className={`px-2 py-0.5 text-xs font-medium ${status.color}`}>{status.label}</span>
                      </div>
                    </div>
                    {c.link && (
                      <a
                        href={c.link}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="shrink-0 px-3 py-1.5 text-xs border border-blue-300 text-blue-600 hover:bg-blue-50 rounded"
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
                    <div className="text-sm text-gray-400">暂无题目</div>
                  )}
                </div>
              )
            })}
          </div>
        )}
      </section>

      {/* ===== 未关联比赛的已发布题目 ===== */}
      {unassigned.length > 0 && (
        <section>
          <h2 className="text-lg font-semibold mb-4 text-gray-700">其他公示题目 ({unassigned.length})</h2>
          <div className="grid gap-2">
            {unassigned.map(p => <ProblemCard key={p.id} p={p} difficultyMap={difficultyMap} />)}
          </div>
        </section>
      )}
    </div>
  )
}
