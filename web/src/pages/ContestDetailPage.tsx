import { useState, useEffect } from 'react'
import { useParams, Link } from 'react-router-dom'
import { apiFetch } from '../api'
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
}

interface ContestProblem {
  id: string
  title: string
  author_name: string
  difficulty: string
  status: string
}

export default function ContestDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { difficultyMap } = useDifficulties()
  const [contest, setContest] = useState<ContestDetail | null>(null)
  const [problems, setProblems] = useState<ContestProblem[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!id) return
    Promise.all([
      apiFetch<ContestDetail[] | ContestDetail>('/contests'),
      apiFetch<ContestProblem[]>(`/contests/${id}/problems`),
    ])
      .then(([contestsRes, problemsRes]) => {
        const list = Array.isArray(contestsRes) ? contestsRes : [contestsRes]
        const found = list.find((c: ContestDetail) => c.id === id)
        if (found) setContest(found)
        setProblems(problemsRes)
      })
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [id])

  if (loading) {
    return (
      <div className="p-6 max-w-4xl mx-auto text-center py-12">
        <p className="text-gray-400 dark:text-gray-500">加载中...</p>
      </div>
    )
  }

  if (!contest) {
    return (
      <div className="p-6 max-w-4xl mx-auto text-center py-12">
        <p className="text-gray-500 dark:text-gray-400 mb-4">比赛不存在</p>
        <Link to="/contests" className="text-sm text-blue-600 dark:text-blue-400 underline">返回比赛列表</Link>
      </div>
    )
  }

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <Link to="/contests" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 mb-4 inline-block">&larr; 返回比赛列表</Link>

      <div className="mg-box-shadow p-6 mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-3">{contest.name}</h1>
        <div className="space-y-2 text-sm text-gray-600 dark:text-gray-300">
          <p>时间：{contest.start_time} ~ {contest.end_time}</p>
          {contest.link && (
            <p>
              链接：<a href={contest.link} target="_blank" rel="noopener noreferrer"
                className="text-blue-600 dark:text-blue-400 underline">打开比赛 ↗</a>
            </p>
          )}
          {contest.description && (
            <div className="mt-4 whitespace-pre-wrap">{contest.description}</div>
          )}
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
                <Link to={`/problems/${p.id}`} className="flex-1 text-sm text-gray-800 dark:text-gray-100 hover:text-blue-600 dark:hover:text-blue-400">
                  {p.title}
                </Link>
                <span className="text-xs text-gray-500 dark:text-gray-400">{p.author_name}</span>
                <DiffBadge difficulty={p.difficulty} map={difficultyMap} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
