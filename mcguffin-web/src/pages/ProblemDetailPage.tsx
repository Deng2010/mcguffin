import { useState, useEffect } from 'react'
import { useParams, Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownRenderer from '../components/MarkdownRenderer'
import { useDifficulties, DiffBadge } from '../hooks/useDifficulties'
import type { ProblemDetail } from '../types'

interface Contest {
  id: string
  name: string
}

export default function ProblemDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { user } = useAuth()
  const { difficultyMap, difficulties } = useDifficulties()
  const [problem, setProblem] = useState<ProblemDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [verifierSolution, setVerifierSolution] = useState('')
  const [vsSaved, setVsSaved] = useState(false)

  // Edit state
  const [editing, setEditing] = useState(false)
  const [editDifficulty, setEditDifficulty] = useState('Medium')
  const [editContent, setEditContent] = useState('')
  const [editSolution, setEditSolution] = useState('')
  const [editContestId, setEditContestId] = useState('')
  const [editLink, setEditLink] = useState('')
  const [editAuthorName, setEditAuthorName] = useState('')
  const [contests, setContests] = useState<Contest[]>([])
  const [saving, setSaving] = useState(false)
  const [editMsg, setEditMsg] = useState('')

  useEffect(() => {
    apiFetch<ProblemDetail>(`/problems/detail/${id}`)
      .then(p => {
        setProblem(p)
        if (p.verifier_solution) setVerifierSolution(p.verifier_solution)
      })
      .catch(() => setError('无法加载题目'))
      .finally(() => setLoading(false))
    apiFetch<Contest[]>('/contests')
      .then(setContests)
      .catch(() => {})
  }, [id])

  const isAdmin = user?.role === 'admin' || user?.role === 'superadmin'
  const canEdit = problem && user && (
    isAdmin || problem.author_id === user.id
  )

  const openEdit = () => {
    if (!problem) return
    setEditDifficulty(problem.difficulty)
    setEditContent(problem.content || '')
    setEditSolution(problem.solution || '')
    setEditContestId(problem.contest_id || '')
    setEditLink((problem as any).link || '')
    setEditAuthorName(problem.author_name)
    setEditMsg('')
    setEditing(true)
  }

  const handleSaveEdit = async () => {
    if (!problem) return
    setSaving(true)
    setEditMsg('')
    try {
      const body: Record<string, any> = {}
      if (editDifficulty !== problem.difficulty) body.difficulty = editDifficulty
      if (editContent !== problem.content) body.content = editContent
      if (editSolution !== (problem.solution || '')) body.solution = editSolution
      // Contest change (admin only)
      if (isAdmin && editContestId !== (problem.contest_id || '')) {
        body.contest_id = editContestId || null  // empty string → null (clear)
      }
      // Link change (admin only)
      if (isAdmin && editLink !== ((problem as any).link || '')) {
        body.link = editLink || null
      }
      // Author name change (admin only)
      if (isAdmin && editAuthorName !== problem.author_name) {
        body.author_name = editAuthorName
      }
      if (Object.keys(body).length === 0) {
        setEditMsg('没有修改')
        setSaving(false)
        return
      }
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/problems/${problem.id}`,
        { method: 'PUT', body: JSON.stringify(body) },
      )
      if (!res.success) { setEditMsg(res.message); setSaving(false); return }
      // Reload problem details
      const updated = await apiFetch<ProblemDetail>(`/problems/detail/${id}`)
      setProblem(updated)
      setEditing(false)
      setEditMsg('已保存')
      setTimeout(() => setEditMsg(''), 2000)
    } catch (err) {
      setEditMsg(`保存失败: ${err}`)
    } finally {
      setSaving(false)
    }
  }

  const handleSaveVerifierSolution = async () => {
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/problems/verifier-solution/${id}`,
        { method: 'POST', body: JSON.stringify({ solution: verifierSolution }) },
      )
      if (!res.success) { alert(res.message); return }
      setVsSaved(true)
      setTimeout(() => setVsSaved(false), 2000)
    } catch (err) { alert(`保存失败: ${err}`) }
  }

  if (loading) return <div className="p-6 text-center py-12 text-gray-400">加载中...</div>
  if (error) return <div className="p-6 text-center py-12 text-red-600">{error}</div>
  if (!problem) return null

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <Link to="/problems" className="text-sm text-gray-500 hover:text-gray-800 mb-4 inline-block">← 返回列表</Link>

      <div className="bg-white border border-gray-300 p-6 mb-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-800">{problem.title}</h1>
            <div className="flex gap-4 text-sm text-gray-500 mt-2">
              <span>作者：{problem.author_name}</span>
              <span>赛事：{problem.contest || '无'}</span>
              <span>难度：<DiffBadge difficulty={problem.difficulty} map={difficultyMap} /></span>
              {(problem as any).link && (
                <a href={(problem as any).link} target="_blank" rel="noopener noreferrer" className="text-blue-600 underline hover:text-blue-800">外部链接 ↗</a>
              )}
            </div>
          </div>
          {canEdit && !editing && (
            <button
              onClick={openEdit}
              className="px-3 py-1.5 text-sm border border-gray-300 text-gray-600 hover:bg-gray-100"
            >
              编辑
            </button>
          )}
        </div>

        {editMsg && (
          <div className={`mb-4 p-3 text-sm border ${editMsg === '已保存' ? 'bg-green-50 border-green-300 text-green-700' : 'bg-red-50 border-red-300 text-red-700'}`}>
            {editMsg}
          </div>
        )}

        {editing ? (
          <div className="border-t border-gray-200 pt-4 mt-4">
            <h2 className="text-lg font-semibold mb-4 text-gray-700">编辑题目</h2>

            {isAdmin && (
              <div className="mb-4">
                <label className="block text-sm font-medium mb-2 text-gray-700">关联比赛</label>
                <select
                  value={editContestId}
                  onChange={e => setEditContestId(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500"
                >
                  <option value="">无</option>
                  {contests.map(c => (
                    <option key={c.id} value={c.id}>{c.name}</option>
                  ))}
                </select>
              </div>
            )}

            {isAdmin && (
              <div className="mb-4">
                <label className="block text-sm font-medium mb-2 text-gray-700">外部链接（发布时必填）</label>
                <input type="url" value={editLink} onChange={e => setEditLink(e.target.value)}
                  placeholder="https://..."
                  className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm" />
              </div>
            )}

            {isAdmin && (
              <div className="mb-4">
                <label className="block text-sm font-medium mb-2 text-gray-700">出题人</label>
                <input type="text" value={editAuthorName} onChange={e => setEditAuthorName(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"
                  placeholder="修改出题人名称" />
              </div>
            )}

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2 text-gray-700">难度</label>
              <select
                value={editDifficulty}
                onChange={e => setEditDifficulty(e.target.value)}
                className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500"
              >
                {difficulties.map(d => (
                  <option key={d.name} value={d.name}>{d.label}</option>
                ))}
              </select>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2 text-gray-700">题目内容 (Markdown)</label>
              <textarea
                value={editContent}
                onChange={e => setEditContent(e.target.value)}
                rows={15}
                className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 font-mono text-sm"
                placeholder="# 题目描述"
              />
            </div>

            <div className="mb-6">
              <label className="block text-sm font-medium mb-2 text-gray-700">
                题解 (Markdown) <span className="text-gray-400 font-normal">— 留空表示清除题解</span>
              </label>
              <textarea
                value={editSolution}
                onChange={e => setEditSolution(e.target.value)}
                rows={10}
                className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 font-mono text-sm"
                placeholder="# 题解"
              />
            </div>

            <div className="flex gap-3">
              <button
                onClick={handleSaveEdit}
                disabled={saving}
                className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50"
              >
                {saving ? '保存中...' : '保存修改'}
              </button>
              <button
                onClick={() => setEditing(false)}
                className="px-6 py-2 border border-gray-300 text-gray-600 text-sm hover:bg-gray-100"
              >
                取消
              </button>
            </div>
          </div>
        ) : (
          <>
            {problem.content && (
              <div className="mt-4">
                <h2 className="text-lg font-semibold mb-2 text-gray-700">题目内容</h2>
                <MarkdownRenderer content={problem.content} />
              </div>
            )}

            {problem.solution !== undefined && problem.solution !== '' && (
              <div className="mt-6">
                <h2 className="text-lg font-semibold mb-2 text-gray-700">出题人题解</h2>
                <MarkdownRenderer content={problem.solution || '(空)'} className="bg-blue-50 p-4 border border-blue-200" />
              </div>
            )}
          </>
        )}
      </div>

      {/* Verifier solution — editable for the verifier, read-only for other members */}
      {problem.can_submit_verifier_solution ? (
        <div className="bg-white border border-gray-300 p-6">
          <h2 className="text-lg font-semibold mb-2 text-gray-700">验题人题解</h2>
          <p className="text-sm text-gray-500 mb-3">您已认领此题，可以编写验题人题解（出题人题解不可见）</p>
          <textarea
            value={verifierSolution}
            onChange={e => setVerifierSolution(e.target.value)}
            rows={10}
            className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 font-mono text-sm mb-3"
            placeholder="# 验题人题解\n\n请在这里编写您的题解..."
          />
          <button
            onClick={handleSaveVerifierSolution}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700"
          >
            {vsSaved ? '已保存!' : '保存题解'}
          </button>
        </div>
      ) : problem.verifier_solution && problem.claimed_by ? (
        <div className="bg-white border border-gray-300 p-6">
          <h2 className="text-lg font-semibold mb-2 text-gray-700">验题人题解</h2>
          <p className="text-sm text-gray-500 mb-3">此题目已被认领，以下为验题人题解</p>
          <MarkdownRenderer content={problem.verifier_solution} className="bg-purple-50 p-4 border border-purple-200" />
        </div>
      ) : null}

      {/* Claimed by info (no verifier solution yet) */}
      {problem.claimed_by && !problem.verifier_solution && !problem.can_submit_verifier_solution && (
        <div className="mt-4 text-sm text-gray-500">
          此题目已被认领验题
        </div>
      )}
    </div>
  )
}
