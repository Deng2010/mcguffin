import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownRenderer from '../components/MarkdownRenderer'
import MarkdownEditor from '../components/MarkdownEditor'
import { formatTime } from '../utils/time'

interface SuggestionReply {
  id: string
  author_id: string
  author_name: string
  content: string
  created_at: string
}

interface SuggestionDetail {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  status: string
  replies: SuggestionReply[]
  created_at: string
  updated_at: string
}

const STATUS_LABEL: Record<string, string> = {
  open: '待处理',
  in_progress: '处理中',
  resolved: '已解决',
  closed: '已关闭',
}

const STATUS_COLOR: Record<string, string> = {
  open: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300',
  in_progress: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300',
  resolved: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300',
  closed: 'bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-400',
}

export default function SuggestionDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { user, hasPermission } = useAuth()
  const [suggestion, setSuggestion] = useState<SuggestionDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [replyContent, setReplyContent] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const canManage = hasPermission('manage_team')
  const canReply = hasPermission('view_suggestions')

  const loadSuggestion = () => {
    if (!id) return
    apiFetch<SuggestionDetail>(`/suggestions/${id}`)
      .then(setSuggestion)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadSuggestion() }, [id])

  const handleStatusChange = async (status: string) => {
    if (!id) return
    try {
      await apiFetch(`/suggestions/${id}`, {
        method: 'PUT',
        body: JSON.stringify({ status }),
      })
      loadSuggestion()
    } catch (err) { alert(`更新失败: ${err}`) }
  }

  const handleReply = async () => {
    if (!id || !replyContent.trim()) return
    setSubmitting(true)
    try {
      await apiFetch(`/suggestions/${id}/reply`, {
        method: 'POST',
        body: JSON.stringify({ content: replyContent.trim() }),
      })
      setReplyContent('')
      loadSuggestion()
    } catch (err) { alert(`回复失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const handleDelete = async () => {
    if (!id || !window.confirm('确定要删除这条建议吗？')) return
    try {
      await apiFetch(`/suggestions/${id}`, { method: 'DELETE' })
      navigate('/community')
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  const handleDeleteReply = async (replyId: string) => {
    if (!id || !window.confirm('确定要删除这条回复吗？')) return
    try {
      await apiFetch(`/suggestions/${id}/reply/${replyId}`, { method: 'DELETE' })
      loadSuggestion()
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>
  if (!suggestion) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">建议不存在</div>

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      <button
        onClick={() => navigate('/community')}
        className="mb-6 inline-flex items-center gap-1 px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
      >
        <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        返回社区
      </button>

      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-6">
        {/* Header */}
        <div className="flex items-start gap-3 mb-3">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 flex-wrap mb-1">
              <h1 className="text-xl font-bold text-gray-800 dark:text-gray-100">{suggestion.title}</h1>
              <span className={`text-xs leading-none ${STATUS_COLOR[suggestion.status] || ''}`}>
                {STATUS_LABEL[suggestion.status] || suggestion.status}
              </span>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {canManage && (
              <select
                value={suggestion.status}
                onChange={e => handleStatusChange(e.target.value)}
                className="text-xs border border-gray-300 dark:border-gray-700 px-2 py-1 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-200 focus:outline-none"
              >
                <option value="open">待处理</option>
                <option value="in_progress">处理中</option>
                <option value="resolved">已解决</option>
                <option value="closed">已关闭</option>
              </select>
            )}
            {(canManage || suggestion.author_id === user?.id) && (
              <button
                onClick={handleDelete}
                className="shrink-0 px-2 py-1 text-xs text-red-500 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20"
              >
                删除
              </button>
            )}
          </div>
        </div>

        {/* Meta */}
        <div className="flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500 mb-4">
          <span className="flex items-center gap-1.5">
            <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-[10px] font-bold shrink-0">
              {suggestion.author_name?.charAt(0) || '?'}
            </span>
            <span>{suggestion.author_name}</span>
          </span>
          <span>{formatTime(suggestion.created_at)}</span>
          {suggestion.updated_at !== suggestion.created_at && <span>(最后编辑 {formatTime(suggestion.updated_at)})</span>}
        </div>

        {/* Content */}
        <div className="prose prose-sm max-w-none text-gray-700 dark:text-gray-200 mb-6">
          <MarkdownRenderer content={suggestion.content} />
        </div>

          {/* Replies */}
          {suggestion.replies.length > 0 && (
            <div className="border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
              <h3 className="text-sm font-medium text-gray-600 dark:text-gray-300 mb-3">
                回复 ({suggestion.replies.length})
              </h3>
              <div className="space-y-3">
                {suggestion.replies.map(reply => (
                  <div key={reply.id} className="p-3 bg-gray-50 border border-gray-200 dark:bg-gray-800/50 dark:border-gray-700">
                    <div className="flex items-center justify-between mb-1">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-medium text-gray-700 dark:text-gray-200">{reply.author_name}</span>
                        <span className="text-xs text-gray-400 dark:text-gray-500">{new Date(reply.created_at).toLocaleDateString('zh-CN')}</span>
                      </div>
                      {(canManage || reply.author_id === user?.id) && (
                        <button
                          onClick={() => handleDeleteReply(reply.id)}
                          className="text-xs text-red-400 hover:text-red-600 dark:text-red-400 dark:hover:text-red-300"
                        >
                          删除
                        </button>
                      )}
                    </div>
                    <div className="text-sm text-gray-700 dark:text-gray-300 prose prose-sm max-w-none">
                      <MarkdownRenderer content={reply.content} />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Reply form */}
          {canReply && (
            <div className="border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
              <h3 className="text-sm font-medium text-gray-600 dark:text-gray-300 mb-2">回复</h3>
              <MarkdownEditor
                value={replyContent}
                onChange={setReplyContent}
                placeholder="输入回复（支持 Markdown）"
                rows={6}
              />
              <button
                onClick={handleReply}
                disabled={submitting || !replyContent.trim()}
                className="mt-2 px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
              >
                {submitting ? '发送中...' : '发送'}
              </button>
            </div>
          )}
      </div>
    </div>
  )
}
