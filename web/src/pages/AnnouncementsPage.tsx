import { useState, useEffect } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import type { Announcement } from '../types'
import MarkdownRenderer from '../components/MarkdownRenderer'
import MarkdownEditor from '../components/MarkdownEditor'

export default function AnnouncementsPage() {
  const { hasPermission, isAuthenticated } = useAuth()
  const [announcements, setAnnouncements] = useState<Announcement[]>([])
  const [loading, setLoading] = useState(true)
  const canManage = hasPermission('manage_announcements')

  // Form state
  const [showForm, setShowForm] = useState(false)
  const [editId, setEditId] = useState<string | null>(null)
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [pinned, setPinned] = useState(false)
  const [isPublic, setIsPublic] = useState(true)
  const [submitting, setSubmitting] = useState(false)
  const [expandedId, setExpandedId] = useState<string | null>(null)

  const loadAnnouncements = () => {
    apiFetch<Announcement[]>('/announcements')
      .then(setAnnouncements)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadAnnouncements() }, [])

  const resetForm = () => {
    setEditId(null)
    setTitle('')
    setContent('')
    setPinned(false)
    setIsPublic(true)
    setShowForm(false)
  }

  const handleCreate = async () => {
    if (!title.trim()) return
    setSubmitting(true)
    try {
      await apiFetch('/announcements', {
        method: 'POST',
        body: JSON.stringify({ title: title.trim(), content, pinned, public: isPublic }),
      })
      resetForm()
      loadAnnouncements()
    } catch (err) { alert(`发布失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const handleUpdate = async (id: string) => {
    if (!title.trim()) return
    setSubmitting(true)
    try {
      await apiFetch(`/announcements/${id}`, {
        method: 'PUT',
        body: JSON.stringify({ title: title.trim(), content, pinned, public: isPublic }),
      })
      resetForm()
      loadAnnouncements()
    } catch (err) { alert(`更新失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('确定要删除这条公告吗？')) return
    try {
      await apiFetch(`/announcements/${id}`, { method: 'DELETE' })
      loadAnnouncements()
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  const startEdit = (a: Announcement) => {
    setEditId(a.id)
    setTitle(a.title)
    setContent(a.content)
    setPinned(a.pinned)
    setIsPublic(a.public)
    setShowForm(true)
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">公告</h1>
        {canManage && (
          <button
            onClick={() => { resetForm(); setShowForm(!showForm) }}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
          >
            {showForm ? '取消' : '发布公告'}
          </button>
        )}
      </div>

      {/* Create/Edit Form (admin only) */}
      {canManage && showForm && (
        <div className="mb-6 p-4 bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700">
          <h2 className="text-lg font-semibold mb-3 text-gray-700 dark:text-gray-200">
            {editId ? '编辑公告' : '发布新公告'}
          </h2>
          <div className="mb-3">
            <input
              type="text"
              placeholder="公告标题"
              value={title}
              onChange={e => setTitle(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 text-sm focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
            />
          </div>
          <div className="mb-3">
            <MarkdownEditor
              value={content}
              onChange={setContent}
              placeholder="公告内容（支持 Markdown）"
              rows={20}
            />
          </div>
          <div className="mb-3">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
              <input
                type="checkbox"
                checked={pinned}
                onChange={e => setPinned(e.target.checked)}
                className="w-4 h-4"
              />
              置顶公告
            </label>
          </div>
          <div className="mb-3">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
              <input
                type="checkbox"
                checked={isPublic}
                onChange={e => setIsPublic(e.target.checked)}
                className="w-4 h-4"
              />
              公开公告（所有人可见）
            </label>
          </div>
          <button
            onClick={editId ? () => handleUpdate(editId) : handleCreate}
            disabled={submitting || !title.trim()}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
          >
            {submitting ? '提交中...' : editId ? '保存修改' : '发布'}
          </button>
        </div>
      )}

      {/* Announcement List */}
      {announcements.length === 0 ? (
        <div className="text-center py-12 text-gray-400 dark:text-gray-500">
          {isAuthenticated ? '暂无公告' : '暂无公告，请先登录'}
        </div>
      ) : (
        <div className="space-y-3">
          {announcements.map(a => (
            <div
              key={a.id}
              className={`bg-white border dark:bg-gray-900 ${a.pinned ? 'border-yellow-400 ring-1 ring-yellow-100 dark:border-yellow-800 dark:ring-yellow-900/30' : 'border-gray-300 dark:border-gray-700'}`}
            >
              <div className="p-4">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      {a.pinned && (
                        <span className="text-xs px-1.5 py-0.5 bg-yellow-100 text-yellow-700 border border-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-300 dark:border-yellow-800">
                          置顶
                        </span>
                      )}
                      {!a.public && (
                        <span className="text-xs px-1.5 py-0.5 bg-gray-100 text-gray-500 border border-gray-200 dark:bg-gray-700 dark:text-gray-400 dark:border-gray-700">
                          非公开
                        </span>
                      )}
                      <span
                        className="font-medium text-gray-800 cursor-pointer hover:text-gray-600 dark:text-gray-100 dark:hover:text-gray-400"
                        onClick={() => setExpandedId(expandedId === a.id ? null : a.id)}
                      >
                        {a.title}
                      </span>
                    </div>
                    <div className="text-xs text-gray-400 dark:text-gray-500">
                      {a.author_name} · {new Date(a.created_at).toLocaleDateString('zh-CN')}
                      {a.updated_at !== a.created_at && ` (已编辑)`}
                    </div>
                  </div>
                  {canManage && (
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => startEdit(a)}
                        className="text-xs px-2 py-1 text-gray-600 border border-gray-300 hover:bg-gray-100 dark:text-gray-400 dark:border-gray-700 dark:hover:bg-gray-800"
                      >
                        编辑
                      </button>
                      <button
                        onClick={() => handleDelete(a.id)}
                        className="text-xs px-2 py-1 text-red-600 border border-red-300 hover:bg-red-50 dark:text-red-400 dark:border-red-800 dark:hover:bg-red-900/20"
                      >
                        删除
                      </button>
                    </div>
                  )}
                </div>
                {expandedId === a.id && (
                  <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                    <div className="prose prose-sm max-w-none text-gray-700 dark:text-gray-200">
                      <MarkdownRenderer content={a.content} />
                    </div>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
