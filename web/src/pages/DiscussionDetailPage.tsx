import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownRenderer from '../components/MarkdownRenderer'
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

interface DiscussionReply {
  id: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  content: string
  reactions: Record<string, string[]>
  parent_id: string | null
  reply_to: string | null
  created_at: string
}

interface DiscussionDetail {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  tags: DiscussionTag[]
  emoji: string
  reactions: Record<string, string[]>
  replies: DiscussionReply[]
  created_at: string
  updated_at: string
}

// ============== Constants ==============

const REPLY_MAX_LEN = 300

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

// Group flat replies into parent→children tree
function groupReplies(replies: DiscussionReply[]) {
  const topLevel: DiscussionReply[] = []
  const childrenMap: Record<string, DiscussionReply[]> = {}
  for (const r of replies) {
    if (r.parent_id) {
      if (!childrenMap[r.parent_id]) childrenMap[r.parent_id] = []
      childrenMap[r.parent_id].push(r)
    } else {
      topLevel.push(r)
    }
  }
  return { topLevel, childrenMap }
}

// ============== Reaction Row ==============

function ReactionRow({
  reactions,
  emojis,
  currentUserId,
  onReact,
}: {
  reactions: Record<string, string[]>
  emojis: DiscussionEmoji[]
  currentUserId: string | undefined
  onReact: (emoji: string) => void
}) {
  if (emojis.length === 0) return null

  return (
    <div className="flex flex-wrap items-center gap-1.5 mt-2 ml-0">
      {emojis.map(e => {
        const users = reactions[e.char] || []
        const count = users.length
        const active = currentUserId ? users.includes(currentUserId) : false
        return (
          <button
            key={e.id}
            onClick={() => onReact(e.char)}
            className={`inline-flex items-center gap-1 px-2 py-0.5 text-xs border transition-colors ${
              active
                ? 'border-gray-500 dark:border-gray-400 bg-gray-100 dark:bg-gray-800'
                : 'border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'
            }`}
            title={count > 0 ? `${count} 人` : e.name}
          >
            <span>{e.char}</span>
            {count > 0 && <span className="text-gray-500 dark:text-gray-400">{count}</span>}
          </button>
        )
      })}
    </div>
  )
}

// ============== Single Reply Card ==============

function ReplyCard({
  reply,
  emojis,
  currentUserId,
  isAdmin,
  children,
  onDelete,
  onReact,
  onReply,
  hideReplyButton,
}: {
  reply: DiscussionReply
  emojis: DiscussionEmoji[]
  currentUserId: string | undefined
  isAdmin: boolean
  children?: React.ReactNode
  onDelete: (id: string) => void
  onReact: (id: string, emoji: string) => void
  onReply: (reply: DiscussionReply) => void
  hideReplyButton?: boolean
}) {
  return (
    <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-4">
      <div className="flex items-start gap-3">
        {/* Avatar */}
        {reply.author_avatar_url ? (
          <img src={reply.author_avatar_url} className="w-8 h-8 rounded-full object-cover shrink-0" alt="" />
        ) : (
          <div className="w-8 h-8 bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-gray-600 dark:text-gray-300 text-sm font-bold shrink-0">
            {reply.author_name?.charAt(0) || '?'}
          </div>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2 mb-1">
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-200">{reply.author_name}</span>
              {reply.reply_to && (
                <span className="text-xs text-gray-400 dark:text-gray-500">
                  回复 <span className="text-gray-500 dark:text-gray-400">@{reply.reply_to}</span>
                </span>
              )}
              <span className="text-xs text-gray-400 dark:text-gray-500">{formatTime(reply.created_at)}</span>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              {!hideReplyButton && (
                <button
                  onClick={() => onReply(reply)}
                  className="text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                >
                  回复
                </button>
              )}
              {(isAdmin || reply.author_id === currentUserId) && (
                <button
                  onClick={() => onDelete(reply.id)}
                  className="text-xs text-red-400 hover:text-red-500"
                >
                  删除
                </button>
              )}
            </div>
          </div>
          <div className="text-sm text-gray-700 dark:text-gray-300 prose prose-sm max-w-none">
            <MarkdownRenderer content={reply.content} />
          </div>

          {/* Reactions on reply */}
          <ReactionRow
            reactions={reply.reactions}
            emojis={emojis}
            currentUserId={currentUserId}
            onReact={(emoji) => onReact(reply.id, emoji)}
          />

          {/* Children (nested replies) */}
          {children}
        </div>
      </div>
    </div>
  )
}

// ============== Component ==============

export default function DiscussionDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { user } = useAuth()
  const [discussion, setDiscussion] = useState<DiscussionDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [replyContent, setReplyContent] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [emojis, setEmojis] = useState<DiscussionEmoji[]>([])
  const [allTags, setAllTags] = useState<DiscussionTag[]>([])
  const [editingTags, setEditingTags] = useState(false)
  const [editTagIds, setEditTagIds] = useState<string[]>([])
  const [savingTags, setSavingTags] = useState(false)
  // Sub-reply state
  const [replyTo, setReplyTo] = useState<DiscussionReply | null>(null)

  const isAdmin = user?.role === 'superadmin' || user?.role === 'admin'
  const canDeleteDiscussion = discussion && (isAdmin || discussion.author_id === user?.id)
  const replyCharsLeft = REPLY_MAX_LEN - replyContent.length

  const loadDiscussion = () => {
    if (!id) return
    apiFetch<DiscussionDetail>(`/discussions/${id}`)
      .then(setDiscussion)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => {
    loadDiscussion()
    apiFetch<DiscussionEmoji[]>('/discussions/emojis')
      .then(setEmojis)
      .catch(() => {})
    apiFetch<DiscussionTag[]>('/discussions/tags')
      .then(setAllTags)
      .catch(() => {})
  }, [id])

  const handleReply = async () => {
    if (!replyContent.trim() || !id) return
    if (replyContent.length > REPLY_MAX_LEN) {
      alert(`回复不能超过${REPLY_MAX_LEN}字`)
      return
    }
    setSubmitting(true)
    try {
      const body: Record<string, any> = { content: replyContent.trim() }
      if (replyTo) {
        body.parent_id = replyTo.id
        body.reply_to = replyTo.author_name
      }
      await apiFetch(`/discussions/${id}/reply`, {
        method: 'POST',
        body: JSON.stringify(body),
      })
      setReplyContent('')
      setReplyTo(null)
      loadDiscussion()
    } catch (err) { alert(`回复失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  const handleDeleteDiscussion = async () => {
    if (!discussion || !id) return
    if (!confirm('确定删除此讨论？')) return
    try {
      const res = await apiFetch<any>(`/discussions/${id}`, { method: 'DELETE' })
      if (res.success) {
        navigate('/discussions')
      } else {
        alert(res.message || '删除失败')
      }
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  const handleDeleteReply = async (replyId: string) => {
    if (!id) return
    if (!confirm('确定删除此回复？')) return
    try {
      const res = await apiFetch<any>(`/discussions/${id}/reply/${replyId}`, { method: 'DELETE' })
      if (res.success) {
        loadDiscussion()
      } else {
        alert(res.message || '删除失败')
      }
    } catch (err) { alert(`删除失败: ${err}`) }
  }

  const handleReactDiscussion = async (emoji: string) => {
    if (!id) return
    try {
      await apiFetch(`/discussions/${id}/react`, {
        method: 'POST',
        body: JSON.stringify({ emoji }),
      })
      loadDiscussion()
    } catch { /* ignore */ }
  }

  const toggleEditTag = (tagId: string) => {
    setEditTagIds(prev =>
      prev.includes(tagId) ? prev.filter(t => t !== tagId) : [...prev, tagId]
    )
  }

  const handleSaveTags = async () => {
    if (!id) return
    setSavingTags(true)
    try {
      await apiFetch(`/discussions/${id}`, {
        method: 'PUT',
        body: JSON.stringify({ tags: editTagIds }),
      })
      setEditingTags(false)
      loadDiscussion()
    } catch (err) { alert(`保存失败: ${err}`) }
    finally { setSavingTags(false) }
  }

  const handleReactReply = async (replyId: string, emoji: string) => {
    if (!id) return
    try {
      await apiFetch(`/discussions/${id}/reply/${replyId}/react`, {
        method: 'POST',
        body: JSON.stringify({ emoji }),
      })
      loadDiscussion()
    } catch { /* ignore */ }
  }

  const handleStartReply = (reply: DiscussionReply) => {
    setReplyTo(replyTo?.id === reply.id ? null : reply)
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>
  if (!discussion) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">讨论不存在</div>

  const { topLevel, childrenMap } = groupReplies(discussion.replies)

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      {/* Back button */}
      <button
        onClick={() => navigate('/discussions')}
        className="mb-6 inline-flex items-center gap-1 px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
      >
        <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        返回讨论列表
      </button>

      {/* Discussion card */}
      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-6 mb-6">
        {/* Emoji + Title + Delete */}
        <div className="flex items-start gap-3 mb-3">
          {discussion.emoji && (
            <span className="text-4xl leading-none shrink-0 mt-0.5">{discussion.emoji}</span>
          )}
          <div className="min-w-0 flex-1">
            <h1 className="text-xl font-bold text-gray-800 dark:text-gray-100">{discussion.title}</h1>
          </div>
          {canDeleteDiscussion && (
            <button
              onClick={handleDeleteDiscussion}
              className="shrink-0 px-2 py-1 text-xs text-red-500 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20"
            >
              删除
            </button>
          )}
        </div>

        {/* Meta */}
        <div className="flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500 mb-3 ml-0">
          <span className="flex items-center gap-1.5">
            {discussion.author_avatar_url ? (
              <img src={discussion.author_avatar_url} className="w-5 h-5 rounded-full object-cover" alt="" />
            ) : (
              <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-[10px] font-bold shrink-0">
                {discussion.author_name?.charAt(0) || '?'}
              </span>
            )}
            <span>{discussion.author_name}</span>
          </span>
          <span>{formatTime(discussion.created_at)}</span>
        </div>

        {/* Tags */}
        {editingTags ? (
          <div className="mb-4">
            <div className="flex flex-wrap gap-1.5 mb-2">
              {allTags.map(tag => (
                <button
                  key={tag.id}
                  type="button"
                  onClick={() => toggleEditTag(tag.id)}
                  className={`text-xs px-2 py-0.5 inline-flex items-center gap-1 border ${
                    editTagIds.includes(tag.id)
                      ? 'border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700'
                      : 'border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'
                  }`}
                >
                  <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />
                  {tag.name}
                </button>
              ))}
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleSaveTags}
                disabled={savingTags}
                className="px-3 py-1 text-xs bg-gray-800 text-white hover:bg-gray-700 disabled:opacity-50"
              >
                {savingTags ? '保存中...' : '保存'}
              </button>
              <button
                onClick={() => { setEditingTags(false) }}
                className="px-3 py-1 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800"
              >
                取消
              </button>
            </div>
          </div>
        ) : (discussion.tags && discussion.tags.length > 0 || isAdmin) && (
          <div className="flex flex-wrap items-center gap-1.5 mb-4 ml-0">
            {discussion.tags.map(tag => (
              <span
                key={tag.id}
                className="text-xs px-2 py-0.5 inline-flex items-center gap-1 border border-gray-300 dark:border-gray-700"
              >
                <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />
                {tag.name}
              </span>
            ))}
            {isAdmin && (
              <button
                onClick={() => {
                  setEditTagIds(discussion.tags.map(t => t.id))
                  setEditingTags(true)
                }}
                className="text-xs px-2 py-0.5 border border-dashed border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
                title="编辑标签"
              >
                + 编辑标签
              </button>
            )}
          </div>
        )}

        {/* Content */}
        <div className="prose prose-sm max-w-none text-gray-700 dark:text-gray-200">
          <MarkdownRenderer content={discussion.content} />
        </div>

        {/* Reactions on discussion */}
        <ReactionRow
          reactions={discussion.reactions}
          emojis={emojis}
          currentUserId={user?.id}
          onReact={handleReactDiscussion}
        />
      </div>

      {/* Replies section */}
      <div className="mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3">
          回复 ({discussion.replies?.length || 0})
        </h2>

        {topLevel.length === 0 ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500 text-sm">暂无回复</div>
        ) : (
          <div className="space-y-3">
            {topLevel.map(reply => (
              <div key={reply.id}>
                <ReplyCard
                  reply={reply}
                  emojis={emojis}
                  currentUserId={user?.id}
                  isAdmin={isAdmin}
                  onDelete={handleDeleteReply}
                  onReact={handleReactReply}
                  onReply={handleStartReply}
                >
                  {/* Children (sub-replies) */}
                  {childrenMap[reply.id] && childrenMap[reply.id].length > 0 && (
                    <div className="mt-3 space-y-2 ml-0 border-l-2 border-gray-200 dark:border-gray-700 pl-4">
                      {childrenMap[reply.id].map(child => (
                        <ReplyCard
                          key={child.id}
                          reply={child}
                          emojis={emojis}
                          currentUserId={user?.id}
                          isAdmin={isAdmin}
                          onDelete={handleDeleteReply}
                          onReact={handleReactReply}
                          onReply={handleStartReply}
                          hideReplyButton
                        />
                      ))}
                    </div>
                  )}

                  {/* Sub-reply form inline */}
                  {replyTo?.id === reply.id && (
                    <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                      <div className="text-xs text-gray-400 dark:text-gray-500 mb-2">
                        回复 <span className="font-medium text-gray-600 dark:text-gray-300">@{replyTo.author_name}</span>
                      </div>
                      <div className="flex gap-2 items-start">
                        <input
                          type="text"
                          value={replyContent}
                          onChange={e => {
                            if (e.target.value.length <= REPLY_MAX_LEN) setReplyContent(e.target.value)
                          }}
                          onKeyDown={(e) => {
                            if (e.key === 'Escape') { setReplyTo(null); setReplyContent('') }
                            if (e.ctrlKey && e.key === 'Enter') { e.preventDefault(); handleReply() }
                          }}
                          placeholder="输入回复..."
                          autoFocus
                          className="flex-1 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-3 py-2 text-sm"
                        />
                        <button
                          onClick={handleReply}
                          disabled={submitting || !replyContent.trim()}
                          className="px-3 py-2 bg-gray-800 text-white text-xs hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
                        >
                          {submitting ? '...' : '发送'}
                        </button>
                      </div>
                      <div className="flex justify-between mt-1">
                        <span className="text-xs text-gray-400 dark:text-gray-500">Esc 取消</span>
                        <span className={`text-xs ${replyCharsLeft < 0 ? 'text-red-500 font-bold' : replyCharsLeft < 30 ? 'text-yellow-500' : 'text-gray-400 dark:text-gray-500'}`}>
                          {replyContent.length} / {REPLY_MAX_LEN}
                        </span>
                      </div>
                    </div>
                  )}
                </ReplyCard>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Top-level reply form */}
      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-4">
        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">回复</h3>
        <MarkdownEditor
          value={replyContent}
          onChange={setReplyContent}
          placeholder="输入回复（支持 Markdown）"
          rows={10}
          onKeyDown={(e) => {
            if (e.ctrlKey && e.key === 'Enter') {
              e.preventDefault()
              handleReply()
            }
          }}
        />
        <div className="flex items-center justify-between mt-2">
          <span className={`text-xs ${replyCharsLeft < 0 ? 'text-red-500 font-bold' : replyCharsLeft < 30 ? 'text-yellow-500' : 'text-gray-400 dark:text-gray-500'}`}>
            {replyContent.length} / {REPLY_MAX_LEN}
          </span>
          <button
            onClick={handleReply}
            disabled={submitting || !replyContent.trim() || replyContent.length > REPLY_MAX_LEN}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
          >
            {submitting ? '提交中...' : '发送回复'}
          </button>
        </div>
      </div>
    </div>
  )
}
