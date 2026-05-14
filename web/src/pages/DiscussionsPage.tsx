import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownEditor from '../components/MarkdownEditor'
import MentionDropdown from '../components/MentionDropdown'
import { useMention } from '../hooks/useMention'
import { formatTime } from '../utils/time'
import type { MentionMember } from '../hooks/useMention'
import type { DiscussionTag, DiscussionEmoji, Discussion } from '../types'

// ============== Constants ==============

const TITLE_MAX_LEN = 30
const CONTENT_MAX_LEN = 3000
const PAGE_SIZE = 10

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
  const [teamOnly, setTeamOnly] = useState(false)
  const [submitting, setSubmitting] = useState(false)

  // Filter state
  const [activeTags, setActiveTags] = useState<string[]>([])

  // Fetch options
  const [emojis, setEmojis] = useState<DiscussionEmoji[]>([])
  const [tags, setTags] = useState<DiscussionTag[]>([])
  const [emojiBarOpen, setEmojiBarOpen] = useState(false)
  const [page, setPage] = useState(1)
  const [teamMembers, setTeamMembers] = useState<MentionMember[]>([])
  const createMention = useMention(teamMembers)

  const titleCharsLeft = TITLE_MAX_LEN - title.length
  const contentCharsLeft = CONTENT_MAX_LEN - content.length

  const loadDiscussions = (tags?: string[]) => {
    const query = tags && tags.length > 0 ? `?tags=${encodeURIComponent(tags.join(','))}` : ''
    apiFetch<Discussion[]>(`/discussions${query}`)
      .then(data => {
        setDiscussions(data)
        setPage(1)
      })
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
    // Load team members for @mention
    apiFetch<MentionMember[]>('/team/members')
      .then(setTeamMembers)
      .catch(() => {})
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
          team_only: teamOnly,
          mentioned_user_ids: createMention.getMentionedUserIds(content),
        }),
      })
      setTitle('')
      setContent('')
      setSelectedEmoji('')
      setSelectedTags([])
      setTeamOnly(false)
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

  const totalPages = Math.max(1, Math.ceil(discussions.length / PAGE_SIZE))
  const pagedDiscussions = discussions.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE)

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

      {/* Tag filter pills */}
      <div className="flex flex-wrap gap-1.5 items-center mb-4">
        <span className="text-xs text-gray-400 dark:text-gray-500 mr-1">筛选:</span>
        <button
          type="button"
          onClick={() => {
            setActiveTags([])
            loadDiscussions()
          }}
          className={`text-xs px-2 py-0.5 border transition-colors ${
            activeTags.length === 0
              ? 'border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-100'
              : 'border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800'
          }`}
        >
          全部
        </button>
        {tags.map(tag => (
          <button
            key={tag.id}
            type="button"
            onClick={() => {
              const next = activeTags.includes(tag.id)
                ? activeTags.filter(t => t !== tag.id)
                : [...activeTags, tag.id]
              setActiveTags(next)
              loadDiscussions(next)
            }}
            className={`text-xs px-2 py-0.5 inline-flex items-center gap-1 border transition-colors ${
              activeTags.includes(tag.id)
                ? 'border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-100'
                : 'border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800'
            }`}
          >
            <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />
            {tag.name}
          </button>
        ))}
        {activeTags.length > 1 && (
          <span className="text-xs text-gray-400 dark:text-gray-500 ml-1">
            已选 {activeTags.length} 个标签
          </span>
        )}
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
            <div className="relative">
              <MarkdownEditor
                value={content}
                onChange={setContent}
                placeholder="详细描述（支持 Markdown）"
                rows={20}
                onCursorChange={(val, pos) => createMention.handleTextChange(val, pos)}
                onKeyDown={(e) => {
                  createMention.handleKeyDown(e)
                }}
              />
              <MentionDropdown
                open={createMention.open}
                filtered={createMention.filtered}
                selectedIndex={createMention.selectedIndex}
                className="top-0 left-0"
                onSelect={(m) => {
                  const ta = document.querySelector<HTMLTextAreaElement>('textarea')
                  const cursorPos = ta?.selectionStart ?? content.length
                  const newText = createMention.insertMention(content, cursorPos, m)
                  setContent(newText)
                }}
              />
            </div>
            <div className="flex justify-end mt-1">
              <span className={`text-xs ${contentCharsLeft < 0 ? 'text-red-500 font-bold' : contentCharsLeft < 100 ? 'text-yellow-500' : 'text-gray-400 dark:text-gray-500'}`}>
                {content.length} / {CONTENT_MAX_LEN}
              </span>
            </div>
          </div>

          {/* Emoji selector — collapsible */}
          <div className="mb-3">
            <button
              type="button"
              onClick={() => setEmojiBarOpen(!emojiBarOpen)}
              className="flex items-center gap-1 text-sm text-gray-600 dark:text-gray-400 mb-1.5 hover:text-gray-800 dark:hover:text-gray-200"
            >
              <svg
                className={`w-3 h-3 transition-transform ${emojiBarOpen ? 'rotate-90' : ''}`}
                fill="none" stroke="currentColor" viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
              </svg>
              表情符号 {selectedEmoji && <span className="ml-1">{selectedEmoji}</span>}
            </button>
            {emojiBarOpen && (
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
            )}
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

          {/* Team-only toggle */}
          <div className="mb-3">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={teamOnly}
                onChange={e => setTeamOnly(e.target.checked)}
                className="w-3.5 h-3.5"
              />
              <span className="text-sm text-gray-600 dark:text-gray-400">仅团队成员可见</span>
            </label>
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
          pagedDiscussions.map(d => (
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
                    <div className="flex items-center gap-2 flex-wrap">
                      <Link
                        to={`/discussions/${d.id}`}
                        className="font-medium text-gray-800 dark:text-gray-100 hover:text-gray-600 dark:hover:text-gray-300"
                      >
                        {d.title}
                      </Link>
                      {d.pinned && (
                        <span className="text-xs px-1.5 py-0.5 border border-red-300 dark:border-red-800 text-red-500 dark:text-red-400 leading-none">
                          置顶
                        </span>
                      )}
                      {d.team_only && (
                        <span className="text-xs px-1.5 py-0.5 border border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 leading-none">
                          内部
                        </span>
                      )}
                    </div>
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

                {/* Reactions */}
                {d.reactions && Object.keys(d.reactions).length > 0 && (
                  <div className="flex flex-wrap items-center gap-1.5 mt-2 ml-9">
                    {Object.entries(d.reactions).map(([emoji, users]) => (
                      <span
                        key={emoji}
                        className="inline-flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500"
                      >
                        <span>{emoji}</span>
                        <span>{users.length}</span>
                      </span>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-2 mt-6">
          <button
            disabled={page <= 1}
            onClick={() => setPage(p => Math.max(1, p - 1))}
            className="px-3 py-1 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
          >
            上一页
          </button>
          <span className="text-xs text-gray-400 dark:text-gray-500 px-2">
            {page} / {totalPages}
          </span>
          <button
            disabled={page >= totalPages}
            onClick={() => setPage(p => Math.min(totalPages, p + 1))}
            className="px-3 py-1 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
          >
            下一页
          </button>
        </div>
      )}
    </div>
  )
}