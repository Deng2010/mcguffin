import { useState, useEffect } from 'react'
import { apiFetch } from '../../services/api'
import LoadingSpinner from '../../components/ui/LoadingSpinner'

interface SystemStats {
  users: number
  problems: number
  contests: number
  posts: number
  version: string
  uptime_secs: number
}

interface StatCardProps {
  label: string
  value: string | number
}

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60
  if (days > 0) {
    return `${days}天 ${hours}小时 ${minutes}分`
  }
  if (hours > 0) {
    return `${hours}小时 ${minutes}分 ${secs}秒`
  }
  return `${minutes}分 ${secs}秒`
}

function StatCard({ label, value }: StatCardProps) {
  return (
    <div className="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-1">{label}</p>
      <p className="text-2xl font-bold text-gray-800 dark:text-gray-100">{value}</p>
    </div>
  )
}

export default function SystemInfoPage() {
  const [stats, setStats] = useState<SystemStats | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    apiFetch<SystemStats>('/v1/plugins/system-info/stats')
      .then(setStats)
      .catch((err: Error) => setError(err.message))
      .finally(() => setLoading(false))
  }, [])

  if (loading) {
    return <LoadingSpinner text="加载系统信息..." />
  }

  if (error) {
    return (
      <div className="max-w-4xl mx-auto px-6 py-12">
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-6">
          <p className="text-red-600 dark:text-red-400">加载失败: {error}</p>
        </div>
      </div>
    )
  }

  if (!stats) {
    return (
      <div className="max-w-4xl mx-auto px-6 py-12">
        <p className="text-gray-500 dark:text-gray-400">暂无数据</p>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto px-6 py-12">
      <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-6">系统信息</h1>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        <StatCard label="用户数" value={stats.users} />
        <StatCard label="题目数" value={stats.problems} />
        <StatCard label="赛事数" value={stats.contests} />
        <StatCard label="帖子数" value={stats.posts} />
        <StatCard label="版本" value={stats.version} />
        <StatCard label="运行时间" value={formatUptime(stats.uptime_secs)} />
      </div>
    </div>
  )
}
