import { useState, useEffect } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import type { Suggestion, SuggestionStatus } from '../types'
import MarkdownRenderer from '../components/MarkdownRenderer'

type TabId = 'all' | 'open' | 'in_progress' | 'resolved' | 'closed'

const statusLabel: Record<SuggestionStatus, string> = {
  open: '待处理',
  in_progress: '处理中',
  resolved: '已解决',
  closed: '已关闭',
}

const statusColor: Record<SuggestionStatus, string> = {
  open: 'bg-blue-100 text-blue-700 border-blue-200',
  in_progress: 'bg-yellow-100 text-yellow-700 border-yellow-200',
  resolved: 'bg-green-100 text-green-700 border-green-200',
  closed: 'bg-gray-200 text-gray-600 border-gray-300',
}

export default function SuggestionsPage() {
  const { user, hasPermission } = useAuth()
  const [suggestions, setSuggestions] = useState<Suggestion[]>([])
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<TabId>('all')
  const [showForm, setShowForm] = useState(false)
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [replyContent, setReplyContent] = useState('')
  const [replyingId, setReplyingId] = useState<string | null>(null)

  const canManage = hasPermission('manage_team') // admins can change status
  const canReply = hasPermission('view_suggestions') && user?.team_status === 'joined' // members/admins can reply

  const loadSuggestions = () => {
    apiFetch<Suggestion[]>('/suggestions')
      .then(setSuggestions)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadSuggestions() }, [])

  const filteredSuggestions = suggestions.filter(s => {
    if (activeTab === 'all') return true
    return s.status === activeTab
  })

  const counts = {
    open: suggestions.filter(s => s.status === 'open').length,
    in_progress: suggestions.filter(s => s.status === 'in_progress').length,
    resolved: suggestions.filter(s => s.status === 'resolved').length,
    closed: suggestions.filter(s => s.status === 'closed').length,
  }

  const handleCreate = async () => {
    if (!title.trim()) return
    setSubmitting(true)
    try {
      await apiFetch('/suggestions', {
        method: 'POST',
        body: JSON.stringify({ title: title.trim(), content }),
      })
      setTitle('')
      setContent('')
      setShowForm(false)
      loadSuggestions()
    } catch (err) { alert(`提交失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const handleUpdateStatus = async (id: string, status: SuggestionStatus) => {
    try {
      await apiFetch(`/suggestions/${id}`, {
        method: 'PUT',
        body: JSON.stringify({ status }),
      })
      loadSuggestions()
    } catch (err) { alert(`更新失败: ${err}`) }
  }

  const handleReply = async (id: string) => {
    if (!replyContent.trim()) return
    try {
      await apiFetch(`/suggestions/${id}/reply`, {
        method: 'POST',
        body: JSON.stringify({ content: replyContent.trim() }),
      })
      setReplyingId(null)
      setReplyContent('')
      loadSuggestions()
    } catch (err) { alert(`回复失败: ${err}`) }
  }

  const handleDeleteReply = async (suggestionId: string, replyId: string) => {
    if (!window.confirm('确定要删除这条回复吗？')) return
    try {
      await apiFetch(`/suggestions/${suggestionId}/reply/${replyId}`, { method: 'DELETE' })
      loadSuggestions()
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('确定要删除这条建议吗？')) return
    try {
      await apiFetch(`/suggestions/${id}`, { method: 'DELETE' })
      loadSuggestions()
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  if (loading) return <div className="p-6 text-center text-gray-400 py-12">加载中...</div>

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800">建议</h1>
        <button
          onClick={() => setShowForm(!showForm)}
          className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700"
        >
          {showForm ? '取消' : '提交建议'}
        </button>
      </div>

      {showForm && (
        <div className="mb-6 p-4 bg-white border border-gray-300">
          <div className="mb-3">
            <input
              type="text"
              placeholder="建议标题"
              value={title}
              onChange={e => setTitle(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 text-sm focus:outline-none focus:border-gray-500"
            />
          </div>
          <div className="mb-3">
            <textarea
              placeholder="详细描述（支持 Markdown）"
              value={content}
              onChange={e => setContent(e.target.value)}
              rows={6}
              className="w-full px-3 py-2 border border-gray-300 text-sm focus:outline-none focus:border-gray-500 resize-y"
            />
          </div>
          <button
            onClick={handleCreate}
            disabled={submitting || !title.trim()}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50"
          >
            {submitting ? '提交中...' : '提交'}
          </button>
        </div>
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 mb-6">
        {[
          { id: 'all' as TabId, label: '全部', count: suggestions.length },
          { id: 'open' as TabId, label: '待处理', count: counts.open },
          { id: 'in_progress' as TabId, label: '处理中', count: counts.in_progress },
          { id: 'resolved' as TabId, label: '已解决', count: counts.resolved },
          { id: 'closed' as TabId, label: '已关闭', count: counts.closed },
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

      {/* Suggestion List */}
      <div className="space-y-3">
        {filteredSuggestions.length === 0 ? (
          <div className="text-center py-12 text-gray-400">暂无建议</div>
        ) : (
          filteredSuggestions.map(s => (
            <div key={s.id} className="bg-white border border-gray-300">
              <div className="p-4">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span
                        className="font-medium text-gray-800 cursor-pointer hover:text-gray-600"
                        onClick={() => setExpandedId(expandedId === s.id ? null : s.id)}
                      >
                        {s.title}
                      </span>
                      <span className={`text-xs px-2 py-0.5 border ${statusColor[s.status as SuggestionStatus]}`}>
                        {statusLabel[s.status as SuggestionStatus]}
                      </span>
                    </div>
                    <div className="text-xs text-gray-400">
                      {s.author_name} · {new Date(s.created_at).toLocaleDateString('zh-CN')}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {canManage && (
                      <select
                        value={s.status}
                        onChange={e => handleUpdateStatus(s.id, e.target.value as SuggestionStatus)}
                        className="text-xs border border-gray-300 px-2 py-1 bg-white focus:outline-none"
                      >
                        <option value="open">待处理</option>
                        <option value="in_progress">处理中</option>
                        <option value="resolved">已解决</option>
                        <option value="closed">已关闭</option>
                      </select>
                    )}
                    {(canManage || s.author_id === user?.id) && (
                      <button
                        onClick={() => handleDelete(s.id)}
                        className="text-xs px-2 py-1 text-red-600 border border-red-300 hover:bg-red-50"
                      >
                        删除
                      </button>
                    )}
                  </div>
                </div>

                {/* Expanded content */}
                {expandedId === s.id && (
                  <div className="mt-3 pt-3 border-t border-gray-200">
                    <div className="prose prose-sm max-w-none text-gray-700">
                      <MarkdownRenderer content={s.content} />
                    </div>

                    {/* Replies thread */}
                    {s.replies && s.replies.length > 0 && (
                      <div className="mt-4 space-y-2">
                        {s.replies.map(reply => (
                          <div key={reply.id} className="p-3 bg-gray-50 border border-gray-200">
                            <div className="flex items-center justify-between mb-1">
                              <div className="flex items-center gap-2">
                                <span className="text-xs font-medium text-gray-700">{reply.author_name}</span>
                                <span className="text-xs text-gray-400">{new Date(reply.created_at).toLocaleDateString('zh-CN')}</span>
                              </div>
                              {(canManage || reply.author_id === user?.id) && (
                                <button
                                  onClick={() => handleDeleteReply(s.id, reply.id)}
                                  className="text-xs text-red-400 hover:text-red-600"
                                >
                                  删除
                                </button>
                              )}
                            </div>
                            <div className="text-sm text-gray-700 prose prose-sm max-w-none">
                              <MarkdownRenderer content={reply.content} />
                            </div>
                          </div>
                        ))}
                      </div>
                    )}

                    {/* Reply form */}
                    {canReply && (
                      <div className="mt-3">
                        {replyingId === s.id ? (
                          <div>
                            <textarea
                              value={replyContent}
                              onChange={e => setReplyContent(e.target.value)}
                              rows={3}
                              className="w-full px-3 py-2 border border-gray-300 text-sm focus:outline-none focus:border-gray-500 resize-y mb-2"
                              placeholder="输入回复（支持 Markdown）"
                            />
                            <div className="flex gap-2">
                              <button
                                onClick={() => handleReply(s.id)}
                                disabled={!replyContent.trim()}
                                className="px-3 py-1 bg-gray-800 text-white text-xs hover:bg-gray-700 disabled:opacity-50"
                              >
                                发送
                              </button>
                              <button
                                onClick={() => { setReplyingId(null); setReplyContent('') }}
                                className="px-3 py-1 text-xs text-gray-600 border border-gray-300 hover:bg-gray-100"
                              >
                                取消
                              </button>
                            </div>
                          </div>
                        ) : (
                          <button
                            onClick={() => setReplyingId(s.id)}
                            className="text-xs text-gray-500 hover:text-gray-700"
                          >
                            回复
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
