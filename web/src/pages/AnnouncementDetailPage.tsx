import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownRenderer from '../components/MarkdownRenderer'
import MarkdownEditor from '../components/MarkdownEditor'
import { formatTime } from '../utils/time'

interface AnnouncementDetail {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  pinned: boolean
  public: boolean
  created_at: string
  updated_at: string
}

export default function AnnouncementDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { user, hasPermission } = useAuth()
  const [announcement, setAnnouncement] = useState<AnnouncementDetail | null>(null)
  const [loading, setLoading] = useState(true)

  // Edit form state
  const [editing, setEditing] = useState(false)
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [pinned, setPinned] = useState(false)
  const [isPublic, setIsPublic] = useState(true)
  const [submitting, setSubmitting] = useState(false)

  const canManage = hasPermission('manage_announcements')

  const loadAnnouncement = () => {
    if (!id) return
    apiFetch<AnnouncementDetail>(`/announcements/${id}`)
      .then(setAnnouncement)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadAnnouncement() }, [id])

  const startEdit = () => {
    if (!announcement) return
    setTitle(announcement.title)
    setContent(announcement.content)
    setPinned(announcement.pinned)
    setIsPublic(announcement.public)
    setEditing(true)
  }

  const handleSave = async () => {
    if (!id || !title.trim()) return
    setSubmitting(true)
    try {
      await apiFetch(`/announcements/${id}`, {
        method: 'PUT',
        body: JSON.stringify({ title: title.trim(), content, pinned, public: isPublic }),
      })
      setEditing(false)
      loadAnnouncement()
    } catch (err) { alert(`保存失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const handleDelete = async () => {
    if (!id || !window.confirm('确定要删除这条公告吗？')) return
    try {
      await apiFetch(`/announcements/${id}`, { method: 'DELETE' })
      navigate('/announcements')
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>
  if (!announcement) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">公告不存在</div>

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

      <div className={`bg-white border dark:bg-gray-900 p-6 ${
        announcement.pinned ? 'border-yellow-400 ring-1 ring-yellow-100 dark:border-yellow-800 dark:ring-yellow-900/30' : 'border-gray-300 dark:border-gray-700'
      }`}>
        {/* Header */}
        <div className="flex items-start gap-3 mb-3">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 flex-wrap mb-1">
              <h1 className="text-xl font-bold text-gray-800 dark:text-gray-100">{announcement.title}</h1>
              {announcement.pinned && (
                <span className="text-xs px-1.5 py-0.5 border border-red-300 dark:border-red-800 text-red-500 dark:text-red-400 leading-none">置顶</span>
              )}
              {!announcement.public && (
                <span className="text-xs px-1.5 py-0.5 border border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 leading-none">内部</span>
              )}
            </div>
          </div>
          {canManage && !editing && (
            <div className="flex items-center gap-2 shrink-0">
              <button onClick={startEdit} className="shrink-0 px-2 py-1 text-xs text-gray-500 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800">编辑</button>
              <button onClick={handleDelete} className="shrink-0 px-2 py-1 text-xs text-red-500 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20">删除</button>
            </div>
          )}
        </div>

          {/* Meta */}
          <div className="flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500 mb-4">
            <span className="flex items-center gap-1.5">
              <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-[10px] font-bold shrink-0">
                {announcement.author_name?.charAt(0) || '?'}
              </span>
              <span>{announcement.author_name}</span>
            </span>
            <span>{formatTime(announcement.created_at)}</span>
            {announcement.updated_at !== announcement.created_at && <span>(已编辑)</span>}
          </div>

          {/* Edit form */}
          {editing ? (
            <div>
              <div className="mb-3">
                <MarkdownEditor
                  value={content}
                  onChange={setContent}
                  placeholder="公告内容（支持 Markdown）"
                  rows={20}
                />
              </div>
              <div className="mb-3 flex gap-4">
                <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
                  <input type="checkbox" checked={pinned} onChange={e => setPinned(e.target.checked)} className="w-4 h-4" />
                  置顶
                </label>
                <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
                  <input type="checkbox" checked={isPublic} onChange={e => setIsPublic(e.target.checked)} className="w-4 h-4" />
                  公开
                </label>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={handleSave}
                  disabled={submitting || !title.trim()}
                  className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
                >
                  {submitting ? '保存中...' : '保存'}
                </button>
                <button
                  onClick={() => setEditing(false)}
                  className="px-4 py-2 text-sm text-gray-600 border border-gray-300 hover:bg-gray-100 dark:text-gray-300 dark:border-gray-700 dark:hover:bg-gray-800"
                >
                  取消
                </button>
              </div>
            </div>
          ) : (
            <div className="prose prose-sm max-w-none text-gray-700 dark:text-gray-200">
              <MarkdownRenderer content={announcement.content} />
            </div>
          )}
      </div>
    </div>
  )
}
