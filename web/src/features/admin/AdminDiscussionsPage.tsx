import { useState, useEffect } from 'react'
import { apiFetch } from '../../services/api'

interface ConfigData {
  server: { site_url: string; port: number; data_file: string }
  admin: { password: string; display_name: string }
  site: { name: string; title?: string | null; difficulty_order: string[] }
  oauth: { cp_client_id: string; cp_client_secret: string }
  difficulty: Record<string, { label: string; color: string }>
  discussion_tags?: Record<string, { color: string; description: string }>
  discussion_emojis?: Record<string, { char: string }>
}

export default function AdminDiscussionsPage() {
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')
  const [fullConfig, setFullConfig] = useState<ConfigData | null>(null)

  const [discussionTags, setDiscussionTags] = useState<Record<string, { color: string; description: string }>>({})
  const [newTagName, setNewTagName] = useState('')
  const [newTagColor, setNewTagColor] = useState('#6366f1')
  const [newTagDesc, setNewTagDesc] = useState('')
  const [discussionEmojis, setDiscussionEmojis] = useState<Record<string, { char: string }>>({})
  const [newEmojiName, setNewEmojiName] = useState('')
  const [newEmojiChar, setNewEmojiChar] = useState('')

  const inputClass = "w-full px-4 py-2 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 focus:outline-none focus:border-gray-500 text-sm"

  const loadConfig = async () => {
    setLoading(true)
    try {
      const res = await apiFetch<{ success: boolean; config?: ConfigData; message?: string }>('/admin/config')
      if (!res.success || !res.config) { setMsg(`加载配置失败: ${res.message}`); return }
      setFullConfig(res.config)
      setDiscussionTags(res.config.discussion_tags ?? {})
      setDiscussionEmojis(res.config.discussion_emojis ?? {})
    } catch (err) { setMsg(`加载配置失败: ${err}`) }
    finally { setLoading(false) }
  }

  useEffect(() => { loadConfig() }, [])

  const addTag = () => {
    const name = newTagName.trim()
    if (!name) return
    if (name in discussionTags) return
    setDiscussionTags({ ...discussionTags, [name]: { color: newTagColor, description: newTagDesc.trim() } })
    setNewTagName('')
    setNewTagColor('#6366f1')
    setNewTagDesc('')
  }

  const removeTag = (name: string) => {
    const { [name]: _, ...rest } = discussionTags
    setDiscussionTags(rest)
  }

  const addEmoji = () => {
    const name = newEmojiName.trim()
    if (!name) return
    if (!newEmojiChar.trim()) return
    if (name in discussionEmojis) return
    setDiscussionEmojis({ ...discussionEmojis, [name]: { char: newEmojiChar.trim() } })
    setNewEmojiName('')
    setNewEmojiChar('')
  }

  const removeEmoji = (name: string) => {
    const { [name]: _, ...rest } = discussionEmojis
    setDiscussionEmojis(rest)
  }

  const handleSave = async () => {
    if (!fullConfig) return
    setSaving(true); setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/config', {
        method: 'PUT',
        body: JSON.stringify({
          ...fullConfig,
          discussion_tags: discussionTags,
          discussion_emojis: discussionEmojis,
        }),
      })
      if (!res.success) { setMsg(`保存失败: ${res.message}`); return }
      setMsg(res.message)
      setTimeout(() => setMsg(''), 5000)
    } catch (err) { setMsg(`保存失败: ${err}`) }
    finally { setSaving(false) }
  }

  if (loading) {
    return <div className="text-center py-12 text-gray-400 dark:text-gray-500">加载配置中...</div>
  }

  return (
    <div>
      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
        }`}>
          {msg}
        </div>
      )}

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow p-5 mb-6">
        <section className="mb-10">
          <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">标签管理</h2>
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">添加或删除讨论区标签。保存后立即生效。</p>
          <div className="flex flex-wrap items-end gap-3 mb-4 p-3 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow">
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">名称</label>
              <input type="text" value={newTagName} onChange={e => setNewTagName(e.target.value)} placeholder="标签名" className="w-28 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm" onKeyDown={e => e.key === 'Enter' && addTag()} />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">颜色</label>
              <input type="color" value={newTagColor} onChange={e => setNewTagColor(e.target.value)} className="w-10 h-8 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer bg-white dark:bg-gray-800" />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">备注</label>
              <input type="text" value={newTagDesc} onChange={e => setNewTagDesc(e.target.value)} placeholder="可选备注" className="w-36 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm" onKeyDown={e => e.key === 'Enter' && addTag()} />
            </div>
            <button onClick={addTag} className="px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600">添加</button>
          </div>
          <div className="space-y-1">
            {Object.keys(discussionTags).length === 0 && <p className="text-sm text-gray-400 dark:text-gray-500">暂无标签</p>}
            {Object.entries(discussionTags).map(([name, fields]) => (
              <div key={name} className="flex items-center gap-3 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow">
                <span className="w-2.5 h-2.5 inline-block shrink-0" style={{ backgroundColor: fields.color }} />
                <span className="text-sm text-gray-800 dark:text-gray-100 w-24">{name}</span>
                <span className="text-xs text-gray-400 dark:text-gray-500 flex-1">{fields.description}</span>
                <button onClick={() => removeTag(name)} className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300">删除</button>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">表情管理</h2>
          <div className="flex flex-wrap items-end gap-3 mb-4 p-3 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow">
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">标识</label>
              <input type="text" value={newEmojiName} onChange={e => setNewEmojiName(e.target.value)} placeholder="如：fire" className="w-24 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm" onKeyDown={e => e.key === 'Enter' && addEmoji()} />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">字符</label>
              <input type="text" value={newEmojiChar} onChange={e => setNewEmojiChar(e.target.value)} placeholder="如：🔥" maxLength={2} className="w-16 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm text-center" onKeyDown={e => e.key === 'Enter' && addEmoji()} />
            </div>
            {newEmojiChar && <div className="text-2xl leading-none pb-1">{newEmojiChar}</div>}
            <button onClick={addEmoji} className="px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600">添加</button>
          </div>
          <div className="space-y-1">
            {Object.keys(discussionEmojis).length === 0 && <p className="text-sm text-gray-400 dark:text-gray-500">暂无表情</p>}
            {Object.entries(discussionEmojis).map(([name, fields]) => (
              <div key={name} className="flex items-center gap-3 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow">
                <span className="text-xl w-8 text-center shrink-0">{fields.char}</span>
                <span className="text-sm text-gray-800 dark:text-gray-100 flex-1">{name}</span>
                <button onClick={() => removeEmoji(name)} className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300">删除</button>
              </div>
            ))}
          </div>
        </section>
      </div>

      <div className="flex gap-3 items-center">
        <button onClick={handleSave} disabled={saving}
          className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600">
          {saving ? '保存中...' : '保存配置'}
        </button>
        <p className="text-xs text-gray-400 dark:text-gray-500">保存后立即生效，无需重启服务。</p>
      </div>
    </div>
  )
}
