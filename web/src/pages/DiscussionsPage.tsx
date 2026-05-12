import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownEditor from '../components/MarkdownEditor'

// ============== Types ==============

interface DiscussionTag {
  id: string
  name: string
  color: string
}

interface DiscussionEmoji {
  id: string
  char: string
  name: string
}

interface Discussion {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  tags: DiscussionTag[]
  emoji: string
  reply_count: number
  created_at: string
  updated_at: string
}

// ============== Constants ==============

const TITLE_MAX_LEN = 30
const CONTENT_MAX_LEN = 3000

// ============== Helpers ==============

function formatTime(dateStr: string) {
  const d = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - d.getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 60) return `${mins}分钟前`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}小时前`
  const days = Math.floor(hours / 24)
  if (days < 7) return `${days}天前`
  return d.toLocaleDateString('zh-CN')
}

// ============== Component ==============

export default function DiscussionsPage() {
  const { user } = useAuth()
  const [discussions, setDiscussions] = useState<Discussion[]>([])
  const [loading, setLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [selectedEmoji, setSelectedEmoji] = useState('')
  const [selectedTags, setSelectedTags] = useState<string[]>([])
  const [submitting, setSubmitting] = useState(false)

  // Fetch options
  const [emojis, setEmojis] = useState<DiscussionEmoji[]>([])
  const [tags, setTags] = useState<DiscussionTag[]>([])

  const titleCharsLeft = TITLE_MAX_LEN - title.length
  const contentCharsLeft = CONTENT_MAX_LEN - content.length

  const loadDiscussions = () => {
    apiFetch<Discussion[]>('/discussions')
      .then(setDiscussions)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  const loadOptions = async () => {
    try {
      const [emojiData, tagData] = await Promise.all([
        apiFetch<DiscussionEmoji[]>('/discussions/emojis'),
        apiFetch<DiscussionTag[]>('/discussions/tags'),
      ])
      setEmojis(emojiData)
      setTags(tagData)
    } catch { /* ignore */ }
  }

  useEffect(() => {
    loadDiscussions()
    loadOptions()
  }, [])

  const handleCreate = async () => {
    if (!title.trim()) return
    setSubmitting(true)
    try {
      await apiFetch('/discussions', {
        method: 'POST',
        body: JSON.stringify({
          title: title.trim(),
          content,
          emoji: selectedEmoji || undefined,
          tags: selectedTags,
        }),
      })
      setTitle('')
      setContent('')
      setSelectedEmoji('')
      setSelectedTags([])
      setShowForm(false)
      loadDiscussions()
    } catch (err) { alert(`提交失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const toggleTag = (tagId: string) => {
    setSelectedTags(prev =>
      prev.includes(tagId) ? prev.filter(t => t !== tagId) : [...prev, tagId]
    )
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">讨论</h1>
        <button
          onClick={() => setShowForm(!showForm)}
          className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
        >
          {showForm ? '取消' : '发起讨论'}
        </button>
      </div>

      {/* Create form */}
      {showForm && (
        <div className="mb-6 p-4 bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700">
          {/* Title */}
          <div className="mb-3">
            <input
              type="text"
              placeholder="讨论标题"
              value={title}
              onChange={e => {
                if (e.target.value.length <= TITLE_MAX_LEN) setTitle(e.target.value)
              }}
              className="w-full border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-3 py-2 text-sm"
            />
            <div className="flex justify-end mt-1">
              <span className={`text-xs ${titleCharsLeft < 0 ? 'text-red-500 font-bold' : titleCharsLeft < 5 ? 'text-yellow-500' : 'text-gray-400 dark:text-gray-500'}`}>
                {title.length} / {TITLE_MAX_LEN}
              </span>
            </div>
          </div>

          {/* Content */}
          <div className="mb-3">
            <MarkdownEditor
              value={content}
              onChange={setContent}
              placeholder="详细描述（支持 Markdown）"
              rows={20}
            />
            <div className="flex justify-end mt-1">
              <span className={`text-xs ${contentCharsLeft < 0 ? 'text-red-500 font-bold' : contentCharsLeft < 100 ? 'text-yellow-500' : 'text-gray-400 dark:text-gray-500'}`}>
                {content.length} / {CONTENT_MAX_LEN}
              </span>
            </div>
          </div>

          {/* Emoji selector */}
          <div className="mb-3">
            <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1.5">表情符号</label>
            <div className="flex flex-wrap gap-1.5">
              {emojis.map(emoji => (
                <button
                  key={emoji.id}
                  type="button"
                  onClick={() => setSelectedEmoji(selectedEmoji === emoji.char ? '' : emoji.char)}
                  className={`px-2 py-1 border text-sm ${
                    selectedEmoji === emoji.char
                      ? 'border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700'
                      : 'border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'
                  }`}
                  title={emoji.name}
                >
                  <span className="mr-1">{emoji.char}</span>
                  <span className="text-xs text-gray-500 dark:text-gray-400">{emoji.name}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Tag selector */}
          <div className="mb-3">
            <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1.5">标签</label>
            <div className="flex flex-wrap gap-1.5">
              {tags.map(tag => (
                <button
                  key={tag.id}
                  type="button"
                  onClick={() => toggleTag(tag.id)}
                  className={`text-xs px-2 py-0.5 inline-flex items-center gap-1 border ${
                    selectedTags.includes(tag.id)
                      ? 'border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700'
                      : 'border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'
                  }`}
                >
                  <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />
                  {tag.name}
                </button>
              ))}
            </div>
          </div>

          {/* Submit */}
          <button
            onClick={handleCreate}
            disabled={submitting || !title.trim()}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
          >
            {submitting ? '提交中...' : '提交'}
          </button>
        </div>
      )}

      {/* Discussion list */}
      <div className="space-y-3">
        {discussions.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">暂无讨论</div>
        ) : (
          discussions.map(d => (
            <div
              key={d.id}
              className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 border-l-4 border-l-gray-300"
            >
              <div className="p-4">
                {/* Title row */}
                <div className="flex items-start gap-2 mb-1">
                  {d.emoji && (
                    <span className="text-2xl leading-none mt-0.5 shrink-0">{d.emoji}</span>
                  )}
                  <div className="min-w-0 flex-1">
                    <Link
                      to={`/discussions/${d.id}`}
                      className="font-medium text-gray-800 dark:text-gray-100 hover:text-gray-600 dark:hover:text-gray-300"
                    >
                      {d.title}
                    </Link>
                  </div>
                </div>

                {/* Tags */}
                {d.tags && d.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1.5 mb-2 ml-9">
                    {d.tags.map(tag => (
                      <span
                        key={tag.id}
                        className="text-xs px-2 py-0.5 inline-flex items-center gap-1 border border-gray-300 dark:border-gray-700"
                      >
                        <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />
                        {tag.name}
                      </span>
                    ))}
                  </div>
                )}

                {/* Meta */}
                <div className="flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500 ml-9">
                  <span className="flex items-center gap-1.5">
                    {d.author_avatar_url ? (
                      <img src={d.author_avatar_url} className="w-5 h-5 rounded-full object-cover" alt="" />
                    ) : (
                      <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-[10px] font-bold shrink-0">
                        {d.author_name?.charAt(0) || '?'}
                      </span>
                    )}
                    <span>{d.author_name}</span>
                  </span>
                  <span>{d.reply_count} 条回复</span>
                  <span>{formatTime(d.created_at)}</span>
                </div>

                {/* Content preview */}
                {d.content && (
                  <div className="mt-2 ml-9 text-sm text-gray-500 dark:text-gray-400 line-clamp-2">
                    {d.content.length > 100
                      ? d.content.slice(0, 100) + '...'
                      : d.content}
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
