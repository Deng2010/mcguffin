import { useState, useEffect, createContext, useContext } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

// ============== Types ==============

interface ConfigData {
  server: { site_url: string; port: number; data_file: string }
  admin: { password: string; display_name: string }
  site: { name: string; title?: string | null; difficulty_order: string[] }
  oauth: { cp_client_id: string; cp_client_secret: string }
  difficulty: Record<string, { label: string; color: string }>
}

interface DifficultyEntry {
  name: string
  label: string
  color: string
}

interface BackupEntry {
  name: string
  size: number
  modified: string
}

interface DiscussionTag {
  id: string
  name: string
  color: string
  description: string
}

interface DiscussionEmoji {
  id: string
  char: string
  name: string
}

type TabId = 'server' | 'admin' | 'site' | 'oauth' | 'difficulty' | 'backup' | 'discussions'

// ============== Component ==============

export default function AdminConfigPage() {
  const { user } = useAuth()
  const isSuperadmin = user?.role === 'superadmin'
  const [activeTab, setActiveTab] = useState<TabId>('server')

  if (!isSuperadmin) {
    return <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">权限不足</div>
  }

  const tabs: { id: TabId; label: string }[] = [
    { id: 'server', label: '服务器' },
    { id: 'admin', label: '管理员' },
    { id: 'site', label: '站点' },
    { id: 'oauth', label: 'OAuth' },
    { id: 'difficulty', label: '难度' },
    { id: 'discussions', label: '讨论管理' },
    { id: 'backup', label: '备份' },
  ]

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-6">系统管理</h1>

      {/* Flat tab bar */}
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6 flex-wrap">
        {tabs.map(tab => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-gray-800 text-gray-900 dark:border-gray-100 dark:text-gray-100'
                : 'border-transparent text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-100'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Config tabs (server/admin/site/oauth/difficulty) share the config provider */}
      {['server', 'admin', 'site', 'oauth', 'difficulty'].includes(activeTab) ? (
        <ConfigWrapper tab={activeTab as 'server' | 'admin' | 'site' | 'oauth' | 'difficulty'} />
      ) : activeTab === 'backup' ? (
        <BackupPanel />
      ) : (
        <DiscussionsPanel />
      )}
    </div>
  )
}

// ====================================================================
//  Config wrapper — loads config, provides save/restart, renders tab
// ====================================================================

// Shared input class
const inputClass = "w-full px-4 py-2 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 focus:outline-none focus:border-gray-500 text-sm"

interface ConfigCtx {
  siteUrl: string; setSiteUrl: (v: string) => void
  port: string; setPort: (v: string) => void
  dataFile: string; setDataFile: (v: string) => void
  adminPassword: string; setAdminPassword: (v: string) => void
  displayName: string; setDisplayName: (v: string) => void
  siteName: string; setSiteName: (v: string) => void
  siteTitle: string; setSiteTitle: (v: string) => void
  cpClientId: string; setCpClientId: (v: string) => void
  cpClientSecret: string; setCpClientSecret: (v: string) => void
  difficulties: DifficultyEntry[]
  difficultyOrder: string[]
  newDiffName: string; setNewDiffName: (v: string) => void
  newDiffLabel: string; setNewDiffLabel: (v: string) => void
  newDiffColor: string; setNewDiffColor: (v: string) => void
  updateDiff: (idx: number, field: keyof DifficultyEntry, value: string) => void
  moveDiff: (idx: number, direction: -1 | 1) => void
  removeDiff: (idx: number) => void
  addDiff: () => void
}

const ConfigCtx = createContext<ConfigCtx>(null!)

function ConfigWrapper({ tab }: { tab: 'server' | 'admin' | 'site' | 'oauth' | 'difficulty' }) {
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [restarting, setRestarting] = useState(false)
  const [msg, setMsg] = useState('')

  const [siteUrl, setSiteUrl] = useState('')
  const [port, setPort] = useState('')
  const [dataFile, setDataFile] = useState('')
  const [adminPassword, setAdminPassword] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [siteName, setSiteName] = useState('')
  const [siteTitle, setSiteTitle] = useState('')
  const [cpClientId, setCpClientId] = useState('')
  const [cpClientSecret, setCpClientSecret] = useState('')
  const [difficulties, setDifficulties] = useState<DifficultyEntry[]>([])
  const [difficultyOrder, setDifficultyOrder] = useState<string[]>([])
  const [newDiffName, setNewDiffName] = useState('')
  const [newDiffLabel, setNewDiffLabel] = useState('')
  const [newDiffColor, setNewDiffColor] = useState('#888888')

  const loadConfig = async () => {
    setLoading(true)
    try {
      const res = await apiFetch<{ success: boolean; config?: ConfigData; message?: string }>('/admin/config')
      if (!res.success || !res.config) { setMsg(`加载配置失败: ${res.message}`); return }
      setSiteUrl(res.config.server.site_url)
      setPort(String(res.config.server.port))
      setDataFile(res.config.server.data_file)
      setAdminPassword(res.config.admin.password)
      setDisplayName(res.config.admin.display_name)
      setSiteName(res.config.site.name)
      setSiteTitle(res.config.site.title ?? '')
      setCpClientId(res.config.oauth.cp_client_id)
      setCpClientSecret(res.config.oauth.cp_client_secret)
      setDifficulties(Object.entries(res.config.difficulty).map(([name, fields]) => ({ name, label: fields.label, color: fields.color })))
      setDifficultyOrder(res.config.site.difficulty_order ?? [])
    } catch (err) { setMsg(`加载配置失败: ${err}`) }
    finally { setLoading(false) }
  }

  useEffect(() => { loadConfig() }, [])

  const updateDiff = (idx: number, field: keyof DifficultyEntry, value: string) =>
    setDifficulties(p => { const n = [...p]; n[idx] = { ...n[idx], [field]: value }; return n })

  const moveDiff = (idx: number, dir: -1 | 1) => {
    const t = idx + dir
    if (t < 0 || t >= difficulties.length) return
    setDifficulties(p => { const n = [...p]; [n[idx], n[t]] = [n[t], n[idx]]; return n })
    setDifficultyOrder(p => { const n = [...p]; [n[idx], n[t]] = [n[t], n[idx]]; return n })
  }

  const removeDiff = (idx: number) => {
    const name = difficulties[idx].name
    setDifficulties(p => p.filter((_, i) => i !== idx))
    setDifficultyOrder(p => p.filter(n => n !== name))
  }

  const addDiff = () => {
    const name = newDiffName.trim()
    if (!name) return
    if (difficulties.some(d => d.name === name)) { setMsg(`难度 "${name}" 已存在`); return }
    setDifficulties(p => { setDifficultyOrder(o => [...o, name]); return [...p, { name, label: newDiffLabel.trim() || name, color: newDiffColor }] })
    setNewDiffName(''); setNewDiffLabel(''); setNewDiffColor('#888888')
  }

  const handleSave = async () => {
    setSaving(true); setMsg('')
    try {
      const diffObj: Record<string, { label: string; color: string }> = {}
      for (const d of difficulties) if (d.name.trim()) diffObj[d.name.trim()] = { label: d.label.trim() || d.name, color: d.color }
      const order = difficultyOrder.length > 0 ? difficultyOrder : difficulties.filter(d => d.name.trim()).map(d => d.name.trim())
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/config', {
        method: 'PUT',
        body: JSON.stringify({
          server: { site_url: siteUrl, port: parseInt(port) || 3000, data_file: dataFile },
          admin: { password: adminPassword, display_name: displayName },
          site: { name: siteName, title: siteTitle || undefined, difficulty_order: order },
          oauth: { cp_client_id: cpClientId, cp_client_secret: cpClientSecret },
          difficulty: diffObj,
        }),
      })
      if (!res.success) { setMsg(`保存失败: ${res.message}`); return }
      setMsg(res.message)
      setTimeout(() => setMsg(''), 5000)
    } catch (err) { setMsg(`保存失败: ${err}`) }
    finally { setSaving(false) }
  }

  const handleRestart = async () => {
    if (!window.confirm('确定要重启服务吗？服务会短暂中断（约2-3秒）。')) return
    setRestarting(true); setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/restart', { method: 'POST' })
      if (!res.success) { setMsg(`重启失败: ${res.message}`); setRestarting(false); return }
      setMsg('服务正在重启，页面将在几秒后重载...')
      setTimeout(() => window.location.reload(), 5000)
    } catch (err) { setMsg(`重启失败: ${err}`); setRestarting(false) }
  }

  const ctx: ConfigCtx = {
    siteUrl, setSiteUrl, port, setPort, dataFile, setDataFile,
    adminPassword, setAdminPassword, displayName, setDisplayName,
    siteName, setSiteName, siteTitle, setSiteTitle,
    cpClientId, setCpClientId, cpClientSecret, setCpClientSecret,
    difficulties, difficultyOrder,
    newDiffName, setNewDiffName, newDiffLabel, setNewDiffLabel, newDiffColor, setNewDiffColor,
    updateDiff, moveDiff, removeDiff, addDiff,
  }

  if (loading) return (
    <div>
      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-5 mb-6">
        <div className="text-center py-8 text-gray-400 dark:text-gray-500">加载配置中...</div>
      </div>
    </div>
  )

  const tabContent = () => {
    switch (tab) {
      case 'server': return <ServerForm />
      case 'admin': return <AdminForm />
      case 'site': return <SiteForm />
      case 'oauth': return <OAuthForm />
      case 'difficulty': return <DifficultyForm />
    }
  }

  return (
    <ConfigCtx.Provider value={ctx}>
      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
        }`}>
          {msg}
        </div>
      )}

      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-5 mb-6">
        {tabContent()}
      </div>

      <div className="flex gap-3 items-center">
        <button onClick={handleSave} disabled={saving}
          className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600">
          {saving ? '保存中...' : '保存配置'}
        </button>
        <button onClick={handleRestart} disabled={restarting}
          className="px-6 py-2 border border-yellow-500 text-yellow-700 text-sm hover:bg-yellow-50 disabled:opacity-50 dark:border-yellow-800 dark:text-yellow-400 dark:hover:bg-yellow-900/20">
          {restarting ? '重启中...' : '重启服务'}
        </button>
        <p className="text-xs text-gray-400 dark:text-gray-500 ml-2">服务器/OAuth/管理员密码修改需重启服务才能生效。难度配置保存后立即生效。</p>
      </div>
    </ConfigCtx.Provider>
  )
}

// ============== Config Form Sub-Components ==============

function ServerForm() {
  const c = useContext(ConfigCtx)
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">站点 URL</label>
        <input type="text" value={c.siteUrl} onChange={e => c.setSiteUrl(e.target.value)} className={inputClass} placeholder="https://lba-oi.team" />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">端口</label>
          <input type="number" value={c.port} onChange={e => c.setPort(e.target.value)} className={inputClass} placeholder="3000" />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">数据文件</label>
          <input type="text" value={c.dataFile} onChange={e => c.setDataFile(e.target.value)} className={inputClass} placeholder="mcguffin_data.json" />
        </div>
      </div>
    </div>
  )
}

function AdminForm() {
  const c = useContext(ConfigCtx)
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">登录密码</label>
        <input type="text" value={c.adminPassword} onChange={e => c.setAdminPassword(e.target.value)} className={inputClass} />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">修改后需重启服务生效</p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">显示名称</label>
        <input type="text" value={c.displayName} onChange={e => c.setDisplayName(e.target.value)} className={inputClass} />
      </div>
    </div>
  )
}

function SiteForm() {
  const c = useContext(ConfigCtx)
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">站点名称</label>
        <input type="text" value={c.siteName} onChange={e => c.setSiteName(e.target.value)} className={inputClass} placeholder="McGuffin" />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">导航栏和首页展示的团队名称</p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">网页标题</label>
        <input type="text" value={c.siteTitle} onChange={e => c.setSiteTitle(e.target.value)} className={inputClass} placeholder="与站点名称相同" />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">浏览器标签页显示的标题，留空则使用站点名称</p>
      </div>
    </div>
  )
}

function OAuthForm() {
  const c = useContext(ConfigCtx)
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">Client ID</label>
        <input type="text" value={c.cpClientId} onChange={e => c.setCpClientId(e.target.value)} className={inputClass} />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">Client Secret</label>
        <input type="text" value={c.cpClientSecret} onChange={e => c.setCpClientSecret(e.target.value)} className={inputClass} />
      </div>
      <p className="text-xs text-gray-400 dark:text-gray-500">修改后需重启服务生效</p>
    </div>
  )
}

function DifficultyForm() {
  const c = useContext(ConfigCtx)
  return (
    <div>
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
        添加、编辑或删除难度等级。名称用作内部标识（如 Easy），标签显示给用户（如 简单），颜色用于 UI 展示。使用 ↑↓ 按钮调整显示顺序。
      </p>
      <div className="space-y-3">
        {c.difficulties.map((d, i) => (
          <div key={i} className="flex items-center gap-2 bg-gray-50 dark:bg-gray-800/50 p-2">
            <div className="flex flex-col gap-0.5">
              <button onClick={() => c.moveDiff(i, -1)} disabled={i === 0} className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1">↑</button>
              <button onClick={() => c.moveDiff(i, 1)} disabled={i === c.difficulties.length - 1} className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1">↓</button>
            </div>
            <span className="text-xs text-gray-400 w-5 text-right">{i + 1}</span>
            <input type="text" value={d.name} onChange={e => c.updateDiff(i, 'name', e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="名称" />
            <input type="text" value={d.label} onChange={e => c.updateDiff(i, 'label', e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="标签" />
            <input type="color" value={d.color} onChange={e => c.updateDiff(i, 'color', e.target.value)} className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer" />
            <span className="text-xs text-gray-500 dark:text-gray-400 w-20">{d.color}</span>
            <button onClick={() => c.removeDiff(i)} className="px-2 py-1 text-red-600 text-sm hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20">删除</button>
          </div>
        ))}
        <div className="flex items-center gap-2 bg-blue-50 dark:bg-blue-900/30 p-2 border border-dashed border-blue-300 dark:border-blue-800">
          <input type="text" value={c.newDiffName} onChange={e => c.setNewDiffName(e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="新难度名称" />
          <input type="text" value={c.newDiffLabel} onChange={e => c.setNewDiffLabel(e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="显示标签" />
          <input type="color" value={c.newDiffColor} onChange={e => c.setNewDiffColor(e.target.value)} className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer" />
          <button onClick={c.addDiff} className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600">添加</button>
        </div>
      </div>
    </div>
  )
}

// ====================================================================
//  Backup Panel
// ====================================================================

function BackupPanel() {
  const [backups, setBackups] = useState<BackupEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [msg, setMsg] = useState('')

  const load = async () => {
    setLoading(true)
    try {
      const res = await apiFetch<{ success: boolean; backups: BackupEntry[] }>('/admin/backups')
      if (res.success) setBackups(res.backups)
    } catch (err) { setMsg(`加载备份列表失败: ${err}`) }
    finally { setLoading(false) }
  }
  useEffect(() => { load() }, [])

  const handleCreate = async () => {
    setCreating(true); setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string; backup?: string }>('/admin/backup', { method: 'POST' })
      if (!res.success) { setMsg(`备份失败: ${res.message}`); return }
      setMsg(`备份已创建: ${res.backup}`); load()
      setTimeout(() => setMsg(''), 5000)
    } catch (err) { setMsg(`备份失败: ${err}`) }
    finally { setCreating(false) }
  }

  const handleRestore = async (name: string) => {
    if (!confirm(`确定要从备份「${name}」恢复吗？`)) return
    try {
      const res = await apiFetch<any>(`/admin/backup/restore/${encodeURIComponent(name)}`, { method: 'POST' })
      setMsg(res.success ? `✅ ${res.message}` : `恢复失败: ${res.message}`)
      if (res.success) load()
    } catch (err) { setMsg(`恢复失败: ${err}`) }
  }

  const handleDelete = async (name: string) => {
    if (!confirm(`确定要删除备份「${name}」吗？`)) return
    try {
      const res = await apiFetch<any>(`/admin/backup/${encodeURIComponent(name)}`, { method: 'DELETE' })
      setMsg(res.success ? `已删除: ${name}` : `删除失败: ${res.message}`)
      if (res.success) load()
    } catch (err) { setMsg(`删除失败: ${err}`) }
  }

  const fmtSize = (b: number) => b > 1048576 ? `${(b/1048576).toFixed(1)} MB` : b > 1024 ? `${(b/1024).toFixed(1)} KB` : `${b} B`

  const doExport = async (type: string) => {
    try {
      const res = await apiFetch<any>(`/admin/export/${type}`)
      if (!res.success) { setMsg(`导出失败: ${res.message}`); return }
      const blob = new Blob([res.content!], { type: res.mime })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a'); a.href = url; a.download = res.filename
      document.body.appendChild(a); a.click(); document.body.removeChild(a)
      URL.revokeObjectURL(url)
      setMsg(`已导出: ${res.filename}`)
    } catch (err) { setMsg(`导出失败: ${err}`) }
  }

  return (
    <div>
      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.startsWith('✅') ? 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
          : msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300'
          : 'bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-700'
        }`}>{msg}</div>
      )}

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">导出</h2>
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">导出当前数据或配置文件，下载到本地。</p>
        <div className="flex items-center gap-3">
          <button onClick={() => doExport('data')} className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800">
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
              导出数据文件
            </span>
          </button>
          <button onClick={() => doExport('config')} className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-200 text-sm hover:bg-gray-100 dark:hover:bg-gray-800">
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
              导出配置文件
            </span>
          </button>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-5">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">备份管理</h2>
        <div className="flex items-center gap-3 mb-4">
          <button onClick={handleCreate} disabled={creating} className="px-5 py-2 bg-gray-800 dark:bg-gray-700 text-white text-sm font-medium hover:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50">
            {creating ? '创建中...' : '创建备份'}
          </button>
          <button onClick={load} disabled={loading} className="px-5 py-2 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50">刷新</button>
        </div>
        {loading ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500">加载中...</div>
        ) : backups.length === 0 ? (
          <div className="text-center py-8 border border-dashed border-gray-300 dark:border-gray-700">
            <p className="text-gray-400 dark:text-gray-500">暂无备份</p>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">点击上方「创建备份」按钮创建第一个备份</p>
          </div>
        ) : (
          <div className="space-y-2">
            {backups.map(b => (
              <div key={b.name} className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <svg className="w-5 h-5 text-gray-400 dark:text-gray-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" /></svg>
                    <span className="font-medium text-gray-800 dark:text-gray-100 truncate text-sm">{b.name}</span>
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 ml-7">{fmtSize(b.size)} · {b.modified}</div>
                </div>
                <div className="flex items-center gap-2 ml-4 shrink-0">
                  <button onClick={() => handleRestore(b.name)} className="px-3 py-1.5 text-xs border border-green-600 dark:border-green-800 text-green-700 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20">恢复</button>
                  <button onClick={() => handleDelete(b.name)} className="px-3 py-1.5 text-xs border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20">删除</button>
                </div>
              </div>
            ))}
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">共 {backups.length} 个备份 · 恢复操作会自动创建当前数据的 pre_restore_ 安全快照</p>
          </div>
        )}
      </div>
    </div>
  )
}

// ====================================================================
//  Discussions Panel
// ====================================================================

function DiscussionsPanel() {
  const [tags, setTags] = useState<DiscussionTag[]>([])
  const [newTagName, setNewTagName] = useState('')
  const [newTagColor, setNewTagColor] = useState('#6366f1')
  const [newTagDesc, setNewTagDesc] = useState('')
  const [editingTag, setEditingTag] = useState<string | null>(null)
  const [editTagName, setEditTagName] = useState('')
  const [editTagColor, setEditTagColor] = useState('')
  const [editTagDesc, setEditTagDesc] = useState('')
  const [tagMsg, setTagMsg] = useState('')

  const [emojis, setEmojis] = useState<DiscussionEmoji[]>([])
  const [newEmojiChar, setNewEmojiChar] = useState('')
  const [newEmojiName, setNewEmojiName] = useState('')
  const [editingEmoji, setEditingEmoji] = useState<string | null>(null)
  const [editEmojiChar, setEditEmojiChar] = useState('')
  const [editEmojiName, setEditEmojiName] = useState('')
  const [emojiMsg, setEmojiMsg] = useState('')

  const loadTags = () => apiFetch<DiscussionTag[]>('/discussions/tags').then(setTags).catch(() => {})
  const loadEmojis = () => apiFetch<DiscussionEmoji[]>('/discussions/emojis').then(setEmojis).catch(() => {})
  useEffect(() => { loadTags(); loadEmojis() }, [])

  const crud = (endpoint: string, method: string, body?: any) =>
    apiFetch<{ success: boolean; message: string }>(endpoint, method === 'GET' ? undefined : { method, body: body ? JSON.stringify(body) : undefined })

  const handleCreateTag = async () => {
    if (!newTagName.trim()) { setTagMsg('标签名不能为空'); return }
    const res = await crud('/discussions/tags', 'POST', { name: newTagName.trim(), color: newTagColor, description: newTagDesc.trim() })
    if (res.success) { setNewTagName(''); setNewTagColor('#6366f1'); setNewTagDesc(''); loadTags() }
    setTagMsg(res.message)
  }

  const handleUpdateTag = async (id: string) => {
    const res = await crud(`/discussions/tags/${id}`, 'PUT', {
      name: editTagName.trim() || undefined, color: editTagColor.trim() || undefined, description: editTagDesc.trim() || undefined,
    })
    if (res.success) { setEditingTag(null); loadTags() }
    setTagMsg(res.message)
  }

  const handleDeleteTag = async (id: string) => {
    if (!confirm('确定删除此标签？')) return
    const res = await crud(`/discussions/tags/${id}`, 'DELETE')
    setTagMsg(res.message); if (res.success) loadTags()
  }

  const handleCreateEmoji = async () => {
    if (!newEmojiChar.trim()) { setEmojiMsg('表情字符不能为空'); return }
    if ([...newEmojiChar.trim()].length !== 1) { setEmojiMsg('表情必须是单个 Unicode 字符'); return }
    if (!newEmojiName.trim()) { setEmojiMsg('表情名称不能为空'); return }
    const res = await crud('/discussions/emojis', 'POST', { char: newEmojiChar.trim(), name: newEmojiName.trim() })
    if (res.success) { setNewEmojiChar(''); setNewEmojiName(''); loadEmojis() }
    setEmojiMsg(res.message)
  }

  const handleUpdateEmoji = async (id: string) => {
    const body: Record<string, string> = {}
    if (editEmojiChar.trim()) body.char = editEmojiChar.trim()
    if (editEmojiName.trim()) body.name = editEmojiName.trim()
    const res = await crud(`/discussions/emojis/${id}`, 'PUT', body)
    if (res.success) { setEditingEmoji(null); loadEmojis() }
    setEmojiMsg(res.message)
  }

  const handleDeleteEmoji = async (id: string) => {
    if (!confirm('确定删除此表情？')) return
    const res = await crud(`/discussions/emojis/${id}`, 'DELETE')
    setEmojiMsg(res.message); if (res.success) loadEmojis()
  }

  return (
    <div>
      <section className="mb-10">
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">标签管理</h2>
        <div className="flex flex-wrap items-end gap-3 mb-4 p-3 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700">
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">名称</label>
            <input type="text" value={newTagName} onChange={e => setNewTagName(e.target.value)} placeholder="标签名" className="w-28 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm" onKeyDown={e => e.key === 'Enter' && handleCreateTag()} />
          </div>
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">颜色</label>
            <input type="color" value={newTagColor} onChange={e => setNewTagColor(e.target.value)} className="w-10 h-8 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer bg-white dark:bg-gray-800" />
          </div>
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">备注</label>
            <input type="text" value={newTagDesc} onChange={e => setNewTagDesc(e.target.value)} placeholder="可选备注" className="w-36 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm" onKeyDown={e => e.key === 'Enter' && handleCreateTag()} />
          </div>
          <button onClick={handleCreateTag} className="px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600">添加</button>
        </div>
        {tagMsg && <p className="text-sm text-gray-600 dark:text-gray-400 mb-3">{tagMsg}</p>}
        <div className="space-y-1">
          {tags.length === 0 && <p className="text-sm text-gray-400 dark:text-gray-500">暂无标签</p>}
          {tags.map(tag => (
            <div key={tag.id} className="flex items-center gap-3 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700">
              {editingTag === tag.id ? (
                <>
                  <input type="text" value={editTagName} onChange={e => setEditTagName(e.target.value)} className="w-24 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1 text-sm" />
                  <input type="color" value={editTagColor} onChange={e => setEditTagColor(e.target.value)} className="w-9 h-7 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer bg-white dark:bg-gray-800" />
                  <input type="text" value={editTagDesc} onChange={e => setEditTagDesc(e.target.value)} className="w-28 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1 text-sm" />
                  <button onClick={() => handleUpdateTag(tag.id)} className="text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 px-2">保存</button>
                  <button onClick={() => setEditingTag(null)} className="text-xs text-gray-400 hover:text-gray-600">取消</button>
                </>
              ) : (
                <>
                  <span className="w-2.5 h-2.5 inline-block shrink-0" style={{ backgroundColor: tag.color }} />
                  <span className="text-sm text-gray-800 dark:text-gray-100 w-24">{tag.name}</span>
                  <span className="text-xs text-gray-400 dark:text-gray-500 flex-1">{tag.description}</span>
                  <button onClick={() => { setEditingTag(tag.id); setEditTagName(tag.name); setEditTagColor(tag.color); setEditTagDesc(tag.description) }} className="text-xs text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 px-2">编辑</button>
                  <button onClick={() => handleDeleteTag(tag.id)} className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300">删除</button>
                </>
              )}
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">表情管理</h2>
        <div className="flex flex-wrap items-end gap-3 mb-4 p-3 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700">
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">字符</label>
            <input type="text" value={newEmojiChar} onChange={e => setNewEmojiChar(e.target.value)} placeholder="如：💡" maxLength={2} className="w-16 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm text-center" onKeyDown={e => e.key === 'Enter' && handleCreateEmoji()} />
          </div>
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">名称</label>
            <input type="text" value={newEmojiName} onChange={e => setNewEmojiName(e.target.value)} placeholder="如：灯泡" className="w-28 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm" onKeyDown={e => e.key === 'Enter' && handleCreateEmoji()} />
          </div>
          {newEmojiChar && <div className="text-2xl leading-none pb-1">{newEmojiChar}</div>}
          <button onClick={handleCreateEmoji} className="px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600">添加</button>
        </div>
        {emojiMsg && <p className="text-sm text-gray-600 dark:text-gray-400 mb-3">{emojiMsg}</p>}
        <div className="space-y-1">
          {emojis.length === 0 && <p className="text-sm text-gray-400 dark:text-gray-500">暂无表情</p>}
          {emojis.map(emoji => (
            <div key={emoji.id} className="flex items-center gap-3 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700">
              {editingEmoji === emoji.id ? (
                <>
                  <input type="text" value={editEmojiChar} onChange={e => setEditEmojiChar(e.target.value)} maxLength={2} className="w-16 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1 text-sm text-center" />
                  <input type="text" value={editEmojiName} onChange={e => setEditEmojiName(e.target.value)} className="w-28 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1 text-sm" />
                  {editEmojiChar && <span className="text-2xl leading-none">{editEmojiChar}</span>}
                  <button onClick={() => handleUpdateEmoji(emoji.id)} className="text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 px-2">保存</button>
                  <button onClick={() => setEditingEmoji(null)} className="text-xs text-gray-400 hover:text-gray-600">取消</button>
                </>
              ) : (
                <>
                  <span className="text-xl w-8 text-center shrink-0">{emoji.char}</span>
                  <span className="text-sm text-gray-800 dark:text-gray-100 flex-1">{emoji.name}</span>
                  <button onClick={() => { setEditingEmoji(emoji.id); setEditEmojiChar(emoji.char); setEditEmojiName(emoji.name) }} className="text-xs text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 px-2">编辑</button>
                  <button onClick={() => handleDeleteEmoji(emoji.id)} className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300">删除</button>
                </>
              )}
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}
