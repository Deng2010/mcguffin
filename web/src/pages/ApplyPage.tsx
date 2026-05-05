import { useState } from 'react'
import { Link, useNavigate, Navigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

export default function ApplyPage() {
  const { user, hasPermission } = useAuth()
  const navigate = useNavigate()
  const [reason, setReason] = useState('')
  const [submitted, setSubmitted] = useState(false)
  const [error, setError] = useState('')

  if (hasPermission('view_team') && user?.team_status === 'joined') {
    return <Navigate to="/team" replace />
  }

  if (user?.team_status === 'pending') {
    return (
      <div className="p-6">
        <div className="max-w-md mx-auto">
          <div className="bg-yellow-50 border border-yellow-300 p-6 text-center dark:bg-yellow-900/30 dark:border-yellow-800">
            <h2 className="text-lg font-semibold text-yellow-800 mb-2 dark:text-yellow-300">申请已提交</h2>
            <p className="text-yellow-700 mb-4 dark:text-yellow-300">您的入队申请正在审核中，请耐心等待</p>
            <Link to="/" className="text-yellow-800 underline dark:text-yellow-300">返回</Link>
          </div>
        </div>
      </div>
    )
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      await apiFetch('/team/apply', {
        method: 'POST',
        body: JSON.stringify({ reason }),
      })
      setSubmitted(true)
    } catch (err) {
      setError(`${err}`)
    }
  }

  if (submitted) {
    return (
      <div className="p-6">
        <div className="max-w-md mx-auto">
          <div className="bg-green-50 border border-green-300 p-6 text-center dark:bg-green-900/30 dark:border-green-800">
            <h2 className="text-lg font-semibold text-green-800 mb-2 dark:text-green-300">申请提交成功</h2>
            <p className="text-green-700 mb-4 dark:text-green-300">您的入队申请已提交，请等待管理员审核</p>
            <Link to="/" className="text-green-800 underline dark:text-green-300">返回成果展示</Link>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6">
      <div className="max-w-md mx-auto">
        <h1 className="text-2xl font-bold mb-6 text-gray-800 dark:text-gray-100">申请加入团队</h1>
        {error && <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">{error}</div>}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">您的昵称</label>
            <input type="text" value={user?.display_name || ''} disabled className="w-full px-4 py-2 border border-gray-300 bg-gray-100 text-gray-600 dark:border-gray-700 dark:bg-gray-700 dark:text-gray-400" />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">申请理由</label>
            <textarea required rows={4} value={reason} onChange={e => setReason(e.target.value)} className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100" placeholder="请简单介绍一下自己..." />
          </div>
          <button type="submit" className="w-full py-3 bg-gray-800 text-white font-medium border border-gray-900 hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600">提交申请</button>
          <Link to="/" className="block text-center text-gray-500 underline dark:text-gray-400">返回</Link>
        </form>
      </div>
    </div>
  )
}
