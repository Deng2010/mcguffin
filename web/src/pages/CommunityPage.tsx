import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'
import MarkdownEditor from '../components/MarkdownEditor'
import { formatTime } from '../utils/time'
import type { DiscussionTag } from '../types'

// ============== Types ==============

interface PostListItem {
  id: string
  kind: 'discussion' | 'suggestion' | 'announcement'
  title: string
  content_preview: string
  author_id: string
  author_name: string
  author_avatar_url?: string | null
  tags: string[]
  created_at: string
  updated_at: string
  pinned: boolean
  status?: string | null
  public?: boolean | null
  team_only?: boolean | null
  reply_count: number
  detail_url: string
}

type TabKind = 'all' | 'announcement' | 'discussion' | 'suggestion'
type CreateKind = 'discussion' | 'suggestion' | 'announcement' | null

const KIND_LABEL: Record<string, string> = {
  announcement: '公告',
  discussion: '讨论',
  suggestion: '建议',
}

const KIND_LEFT_COLOR: Record<string, string> = {
  announcement: 'border-l-red-400',
  discussion: 'border-l-blue-400',
  suggestion: 'border-l-purple-400',
}

const STATUS_LABEL: Record<string, string> = {
  open: '待处理',
  in_progress: '处理中',
  resolved: '已解决',
  closed: '已关闭',
}

const STATUS_COLOR: Record<string, string> = {
  open: 'text-blue-600 dark:text-blue-400',
  in_progress: 'text-yellow-600 dark:text-yellow-400',
  resolved: 'text-green-600 dark:text-green-400',
  closed: 'text-gray-400 dark:text-gray-500',
}

const CREATE_OPTIONS: { kind: CreateKind; label: string; desc: string; color: string }[] = [
  { kind: 'discussion', label: '发起讨论', desc: '公开讨论或团队内部讨论', color: 'border-blue-300 hover:bg-blue-50 dark:border-blue-800 dark:hover:bg-blue-900/20' },
  { kind: 'suggestion', label: '提交建议', desc: '向团队提出建议或反馈', color: 'border-purple-300 hover:bg-purple-50 dark:border-purple-800 dark:hover:bg-purple-900/20' },
  { kind: 'announcement', label: '发布公告', desc: '管理员发布公告', color: 'border-red-300 hover:bg-red-50 dark:border-red-800 dark:hover:bg-red-900/20' },
]

// ============== Component ==============

export default function CommunityPage() {
  const navigate = useNavigate()
  const { user, hasPermission, isAuthenticated } = useAuth()
  const [posts, setPosts] = useState<PostListItem[]>([])
  const [allTags, setAllTags] = useState<DiscussionTag[]>([])
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<TabKind | string>('all')

  // ── Create post ──
  const [showCreatePicker, setShowCreatePicker] = useState(false)
  const [createKind, setCreateKind] = useState<CreateKind>(null)
  const [createTitle, setCreateTitle] = useState('')
  const [createContent, setCreateContent] = useState('')
  const [createPinned, setCreatePinned] = useState(false)
  const [createPublic, setCreatePublic] = useState(true)
  const [createTeamOnly, setCreateTeamOnly] = useState(false)
  const [createTags, setCreateTags] = useState<string[]>([])
  const [submitting, setSubmitting] = useState(false)

  const isAdmin = hasPermission('manage_team')

  const loadPosts = () => {
    apiFetch<PostListItem[]>('/community/posts')
      .then(setPosts)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadPosts() }, [])

  useEffect(() => {
    apiFetch<DiscussionTag[]>('/discussions/tags')
      .then(setAllTags)
      .catch(() => {})
  }, [])

  const filteredPosts = posts.filter(p => {
    if (activeTab === 'all') return true
    if (activeTab === 'announcement') return p.kind === 'announcement'
    if (activeTab === 'discussion') return p.kind === 'discussion'
    if (activeTab === 'suggestion') return p.kind === 'suggestion'
    return p.kind === 'discussion' && p.tags.includes(activeTab)
  })

  const counts: Record<string, number> = {
    all: posts.length,
    announcement: posts.filter(p => p.kind === 'announcement').length,
    discussion: posts.filter(p => p.kind === 'discussion').length,
    suggestion: posts.filter(p => p.kind === 'suggestion').length,
  }
  allTags.forEach(t => {
    counts[t.id] = posts.filter(p => p.kind === 'discussion' && p.tags.includes(t.id)).length
  })

  const goToDetail = (p: PostListItem) => {
    if (p.kind === 'discussion') navigate(`/discussions/${p.id}`)
    else if (p.kind === 'suggestion') navigate(`/suggestions/${p.id}`)
    else navigate(`/announcements/${p.id}`)
  }

  const resetCreateForm = () => {
    setCreateKind(null)
    setCreateTitle('')
    setCreateContent('')
    setCreatePinned(false)
    setCreatePublic(true)
    setCreateTeamOnly(false)
    setCreateTags([])
    setShowCreatePicker(false)
  }

  const handleCreate = async () => {
    if (!createTitle.trim() || !createKind) return
    setSubmitting(true)
    try {
      if (createKind === 'discussion') {
        await apiFetch('/discussions', {
          method: 'POST',
          body: JSON.stringify({ title: createTitle.trim(), content: createContent, tags: createTags, pinned: createPinned, team_only: createTeamOnly }),
        })
      } else if (createKind === 'suggestion') {
        await apiFetch('/suggestions', {
          method: 'POST',
          body: JSON.stringify({ title: createTitle.trim(), content: createContent }),
        })
      } else if (createKind === 'announcement') {
        await apiFetch('/announcements', {
          method: 'POST',
          body: JSON.stringify({ title: createTitle.trim(), content: createContent, pinned: createPinned, public: createPublic }),
        })
      }
      resetCreateForm()
      loadPosts()
    } catch (err) { alert(`发布失败: ${err}`) }
    finally { setSubmitting(false) }
  }

  if (loading) return <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">加载中...</div>

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      {/* ── Header ── */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">社区</h1>
        <div className="flex items-center gap-3">
          {isAuthenticated && (
            <div className="relative">
              <button
                onClick={() => setShowCreatePicker(!showCreatePicker)}
                className="px-3 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
              >
                发帖
              </button>
              {showCreatePicker && (
                <div className="absolute right-0 top-full mt-1 w-48 bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 shadow-lg z-10">
                  {CREATE_OPTIONS.filter(o => o.kind !== 'announcement' || isAdmin).map(opt => (
                    <button
                      key={opt.kind}
                      onClick={() => { setCreateKind(opt.kind); setShowCreatePicker(false) }}
                      className={`w-full text-left px-4 py-3 text-sm border-b border-gray-200 dark:border-gray-700 last:border-0 ${opt.color} dark:text-gray-200`}
                    >
                      <div className="font-medium">{opt.label}</div>
                      <div className="text-xs text-gray-500 dark:text-gray-400">{opt.desc}</div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
          <span className="text-xs text-gray-400 dark:text-gray-500">{posts.length} 条帖子</span>
        </div>
      </div>

      {/* ── Create form ── */}
      {createKind && (
        <div className="mb-6 p-4 bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-gray-700 dark:text-gray-200">发布{KIND_LABEL[createKind]}</h2>
            <button onClick={resetCreateForm} className="text-xs text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300">取消</button>
          </div>
          <div className="mb-3">
            <input type="text" placeholder={`${KIND_LABEL[createKind]}标题`} value={createTitle} onChange={e => setCreateTitle(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 text-sm focus:outline-none focus:border-gray-500 dark:focus:border-gray-400 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100" />
          </div>
          <div className="mb-3">
            <MarkdownEditor value={createContent} onChange={setCreateContent} placeholder={`${KIND_LABEL[createKind]}内容（支持 Markdown）`} rows={12} />
          </div>
          {createKind === 'discussion' && (
            <div className="mb-3 flex flex-wrap gap-4">
              <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
                <input type="checkbox" checked={createTeamOnly} onChange={e => setCreateTeamOnly(e.target.checked)} className="w-4 h-4" />仅团队可见
              </label>
              {isAdmin && (
                <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
                  <input type="checkbox" checked={createPinned} onChange={e => setCreatePinned(e.target.checked)} className="w-4 h-4" />置顶
                </label>
              )}
              {allTags.length > 0 && (
                <div className="flex items-center gap-1">
                  <span className="text-xs text-gray-400 dark:text-gray-500">标签：</span>
                  {allTags.map(tag => (
                    <button key={tag.id} onClick={() => setCreateTags(prev => prev.includes(tag.id) ? prev.filter(t => t !== tag.id) : [...prev, tag.id])}
                      className={`text-xs px-2 py-0.5 inline-flex items-center gap-1 border ${createTags.includes(tag.id) ? 'border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700' : 'border-gray-300 dark:border-gray-700'}`}>
                      <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />{tag.name}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
          {createKind === 'announcement' && (
            <div className="mb-3 flex flex-wrap gap-4">
              <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
                <input type="checkbox" checked={createPinned} onChange={e => setCreatePinned(e.target.checked)} className="w-4 h-4" />置顶
              </label>
              <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300 cursor-pointer">
                <input type="checkbox" checked={createPublic} onChange={e => setCreatePublic(e.target.checked)} className="w-4 h-4" />公开（所有人可见）
              </label>
            </div>
          )}
          <button onClick={handleCreate} disabled={submitting || !createTitle.trim()}
            className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50">
            {submitting ? '发布中...' : '发布'}
          </button>
        </div>
      )}

      {/* ── Tab bar ── */}
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6 overflow-x-auto">
        {[
          { id: 'all', label: '全部' },
          { id: 'discussion', label: '讨论' },
        ].map(tab => (
          <button key={tab.id} onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 whitespace-nowrap transition-colors ${activeTab === tab.id ? 'border-gray-800 dark:border-gray-100 text-gray-900 dark:text-gray-100' : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-100'}`}>
            {tab.label}
            <span className={`ml-1.5 px-1.5 py-0.5 text-xs ${activeTab === tab.id ? 'bg-gray-800 dark:bg-gray-700 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400'}`}>{counts[tab.id] || 0}</span>
          </button>
        ))}
        <div className="w-px h-5 bg-gray-300 dark:bg-gray-700 mx-2" />
        {/* Synthetic type tags + user tags */}
        {[
          { id: 'announcement', label: '公告', count: counts.announcement || 0 },
          { id: 'suggestion', label: '建议', count: counts.suggestion || 0 },
          ...allTags.map(t => ({ id: t.id, label: t.name, count: counts[t.id] || 0 })),
        ].filter(t => t.count > 0 || t.id === activeTab).map(t => (
          <button key={t.id} onClick={() => setActiveTab(t.id)}
            className={`px-3 py-2 text-sm font-medium border-b-2 whitespace-nowrap transition-colors ${activeTab === t.id ? 'border-gray-800 dark:border-gray-100 text-gray-900 dark:text-gray-100' : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-100'}`}>
            {t.label}
            {t.count > 0 && (
              <span className={`ml-1.5 px-1.5 py-0.5 text-xs ${activeTab === t.id ? 'bg-gray-800 dark:bg-gray-700 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400'}`}>{t.count}</span>
            )}
          </button>
        ))}
      </div>

      {/* ── Post List ── */}
      {filteredPosts.length === 0 ? (
        <div className="text-center py-12 text-gray-400 dark:text-gray-500">暂无内容</div>
      ) : (
        <div className="space-y-3">
          {filteredPosts.map(p => (
            <div
              key={`${p.kind}-${p.id}`}
              className={`bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 border-l-4 ${KIND_LEFT_COLOR[p.kind]} cursor-pointer hover:border-gray-400 dark:hover:border-gray-500 transition-colors ${p.pinned ? 'border-yellow-400 border-l-4' : ''}`}
              onClick={() => goToDetail(p)}
            >
              <div className="p-4">
                {/* Title row */}
                <div className="flex items-start gap-2 mb-1">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-medium text-gray-800 dark:text-gray-100 hover:text-gray-600 dark:hover:text-gray-300 cursor-pointer">
                        {p.title}
                      </span>
                      {p.pinned && (
                        <span className="text-xs px-1.5 py-0.5 border border-red-300 dark:border-red-800 text-red-500 dark:text-red-400 leading-none">置顶</span>
                      )}
                      {p.public === false && (
                        <span className="text-xs px-1.5 py-0.5 border border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 leading-none">内部</span>
                      )}
                      {p.team_only === true && (
                        <span className="text-xs px-1.5 py-0.5 border border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 leading-none">内部</span>
                      )}
                      {p.status && p.status !== 'open' && (
                        <span className={`text-xs leading-none ${STATUS_COLOR[p.status] || ''}`}>{STATUS_LABEL[p.status] || p.status}</span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Tags */}
                {p.tags.length > 0 && (
                  <div className="flex flex-wrap gap-1.5 mb-2 ml-0">
                    {p.tags.map(tagId => {
                      const tag = allTags.find(t => t.id === tagId)
                      return tag ? (
                        <span key={tag.id} className="text-xs px-2 py-0.5 inline-flex items-center gap-1 border border-gray-300 dark:border-gray-700">
                          <span className="w-1.5 h-1.5 inline-block" style={{ backgroundColor: tag.color }} />
                          {tag.name}
                        </span>
                      ) : null
                    })}
                  </div>
                )}

                {/* Meta */}
                <div className="flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500 ml-0">
                  <span className="flex items-center gap-1.5">
                    {p.author_avatar_url ? (
                      <img src={p.author_avatar_url} className="w-5 h-5 rounded-full object-cover" alt="" />
                    ) : (
                      <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-[10px] font-bold shrink-0">
                        {p.author_name?.charAt(0) || '?'}
                      </span>
                    )}
                    <span>{p.author_name}</span>
                  </span>
                  {p.reply_count > 0 && <span>{p.reply_count} 条回复</span>}
                  <span>{formatTime(p.created_at)}</span>
                </div>

                {/* Content preview */}
                <div className="mt-2 ml-0 text-sm text-gray-500 dark:text-gray-400 line-clamp-2">
                  {p.content_preview.length > 100 ? p.content_preview.slice(0, 100) + '...' : p.content_preview}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
