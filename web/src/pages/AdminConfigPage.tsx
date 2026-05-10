import { useState, useEffect } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

// ============== Config Types ==============

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

type ConfigTabId = 'server' | 'admin' | 'site' | 'oauth' | 'difficulty'

// ============== Backup Types ==============

interface BackupEntry {
  name: string
  size: number
  modified: string
}

// ============== Discussion Types ==============

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

// ============== Component ==============

type TopTab = 'config' | 'backup' | 'discussions'

export default function AdminConfigPage() {
  const { user } = useAuth()
  const isSuperadmin = user?.role === 'superadmin'
  const [topTab, setTopTab] = useState<TopTab>('config')

  if (!isSuperadmin) {
    return <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">权限不足</div>
  }

  const topTabs: { id: TopTab; label: string }[] = [
    { id: 'config', label: '配置' },
    { id: 'backup', label: '备份' },
    { id: 'discussions', label: '讨论管理' },
  ]

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-6">系统管理</h1>

      {/* Top tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6">
        {topTabs.map(tab => (
          <button
            key={tab.id}
            onClick={() => setTopTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              topTab === tab.id
                ? 'border-gray-800 text-gray-900 dark:border-gray-100 dark:text-gray-100'
                : 'border-transparent text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-100'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {topTab === 'config' && <ConfigPanel />}
      {topTab === 'backup' && <BackupPanel />}
      {topTab === 'discussions' && <DiscussionsPanel />}
    </div>
  )
}

// ====================================================================
//  Config Panel
// ====================================================================

function ConfigPanel() {
  const [config, setConfig] = useState<ConfigData | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [restarting, setRestarting] = useState(false)
  const [msg, setMsg] = useState('')
  const [activeTab, setActiveTab] = useState<ConfigTabId>('server')

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

  const tabs: { id: ConfigTabId; label: string }[] = [
    { id: 'server', label: '服务器' },
    { id: 'admin', label: '管理员' },
    { id: 'site', label: '站点' },
    { id: 'oauth', label: 'OAuth' },
    { id: 'difficulty', label: '难度' },
  ]

  const loadConfig = async () => {
    setLoading(true)
    try {
      const res = await apiFetch<{ success: boolean; config?: ConfigData; message?: string }>('/admin/config')
      if (!res.success || !res.config) { setMsg(`加载配置失败: ${res.message}`); return }
      setConfig(res.config)
      setSiteUrl(res.config.server.site_url)
      setPort(String(res.config.server.port))
      setDataFile(res.config.server.data_file)
      setAdminPassword(res.config.admin.password)
      setDisplayName(res.config.admin.display_name)
      setSiteName(res.config.site.name)
      setSiteTitle(res.config.site.title ?? '')
      setCpClientId(res.config.oauth.cp_client_id)
      setCpClientSecret(res.config.oauth.cp_client_secret)
      const diffArr: DifficultyEntry[] = Object.entries(res.config.difficulty).map(([name, fields]) => ({
        name,
        label: fields.label,
        color: fields.color,
      }))
      setDifficulties(diffArr)
      setDifficultyOrder(res.config.site.difficulty_order ?? [])
    } catch (err) { setMsg(`加载配置失败: ${err}`) }
    finally { setLoading(false) }
  }

  useEffect(() => { loadConfig() }, [])

  const updateDiff = (idx: number, field: keyof DifficultyEntry, value: string) => {
    setDifficulties(prev => { const next = [...prev]; next[idx] = { ...next[idx], [field]: value }; return next })
  }

  const moveDiff = (idx: number, direction: -1 | 1) => {
    const target = idx + direction
    if (target < 0 || target >= difficulties.length) return
    setDifficulties(prev => { const next = [...prev]; [next[idx], next[target]] = [next[target], next[idx]]; return next })
    setDifficultyOrder(prev => { const next = [...prev]; [next[idx], next[target]] = [next[target], next[idx]]; return next })
  }

  const removeDiff = (idx: number) => {
    const removedName = difficulties[idx].name
    setDifficulties(prev => prev.filter((_, i) => i !== idx))
    setDifficultyOrder(prev => prev.filter(n => n !== removedName))
  }

  const addDiff = () => {
    const name = newDiffName.trim()
    if (!name) return
    if (difficulties.some(d => d.name === name)) { setMsg(`难度 "${name}" 已存在`); return }
    setDifficulties(prev => { setDifficultyOrder(prevOrder => [...prevOrder, name]); return [...prev, { name, label: newDiffLabel.trim() || name, color: newDiffColor }] })
    setNewDiffName('')
    setNewDiffLabel('')
    setNewDiffColor('#888888')
  }

  const handleSave = async () => {
    if (!config) return
    setSaving(true); setMsg('')
    try {
      const diffObj: Record<string, { label: string; color: string }> = {}
      for (const d of difficulties) { if (d.name.trim()) diffObj[d.name.trim()] = { label: d.label.trim() || d.name, color: d.color } }
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

  if (loading) return <div className="text-center py-8 text-gray-400 dark:text-gray-500">加载配置中...</div>

  const inputClass = "w-full px-4 py-2 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 focus:outline-none focus:border-gray-500 text-sm"

  const renderTabContent = () => {
    switch (activeTab) {
      case 'server':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">站点 URL</label>
              <input type="text" value={siteUrl} onChange={e => setSiteUrl(e.target.value)} className={inputClass} placeholder="https://lba-oi.team" />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">端口</label>
                <input type="number" value={port} onChange={e => setPort(e.target.value)} className={inputClass} placeholder="3000" />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">数据文件</label>
                <input type="text" value={dataFile} onChange={e => setDataFile(e.target.value)} className={inputClass} placeholder="mcguffin_data.json" />
              </div>
            </div>
          </div>
        )
      case 'admin':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">登录密码</label>
              <input type="text" value={adminPassword} onChange={e => setAdminPassword(e.target.value)} className={inputClass} />
              <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">修改后需重启服务生效</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">显示名称</label>
              <input type="text" value={displayName} onChange={e => setDisplayName(e.target.value)} className={inputClass} />
            </div>
          </div>
        )
      case 'site':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">站点名称</label>
              <input type="text" value={siteName} onChange={e => setSiteName(e.target.value)} className={inputClass} placeholder="McGuffin" />
              <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">导航栏和首页展示的团队名称</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">网页标题</label>
              <input type="text" value={siteTitle} onChange={e => setSiteTitle(e.target.value)} className={inputClass} placeholder="与站点名称相同" />
              <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">浏览器标签页显示的标题，留空则使用站点名称</p>
            </div>
          </div>
        )
      case 'oauth':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">Client ID</label>
              <input type="text" value={cpClientId} onChange={e => setCpClientId(e.target.value)} className={inputClass} />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">Client Secret</label>
              <input type="text" value={cpClientSecret} onChange={e => setCpClientSecret(e.target.value)} className={inputClass} />
            </div>
            <p className="text-xs text-gray-400 dark:text-gray-500">修改后需重启服务生效</p>
          </div>
        )
      case 'difficulty':
        return (
          <div>
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
              添加、编辑或删除难度等级。名称用作内部标识（如 Easy），标签显示给用户（如 简单），颜色用于 UI 展示。使用 ↑↓ 按钮调整显示顺序。
            </p>
            <div className="space-y-3">
              {difficulties.map((d, i) => (
                <div key={i} className="flex items-center gap-2 bg-gray-50 dark:bg-gray-800/50 p-2">
                  <div className="flex flex-col gap-0.5">
                    <button onClick={() => moveDiff(i, -1)} disabled={i === 0} className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1" title="上移">↑</button>
                    <button onClick={() => moveDiff(i, 1)} disabled={i === difficulties.length - 1} className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1" title="下移">↓</button>
                  </div>
                  <span className="text-xs text-gray-400 w-5 text-right">{i + 1}</span>
                  <input type="text" value={d.name} onChange={e => updateDiff(i, 'name', e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="名称" />
                  <input type="text" value={d.label} onChange={e => updateDiff(i, 'label', e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="标签" />
                  <input type="color" value={d.color} onChange={e => updateDiff(i, 'color', e.target.value)} className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer" />
                  <span className="text-xs text-gray-500 dark:text-gray-400 w-20">{d.color}</span>
                  <button onClick={() => removeDiff(i)} className="px-2 py-1 text-red-600 text-sm hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20">删除</button>
                </div>
              ))}
              <div className="flex items-center gap-2 bg-blue-50 dark:bg-blue-900/30 p-2 border border-dashed border-blue-300 dark:border-blue-800">
                <input type="text" value={newDiffName} onChange={e => setNewDiffName(e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="新难度名称" />
                <input type="text" value={newDiffLabel} onChange={e => setNewDiffLabel(e.target.value)} className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500" placeholder="显示标签" />
                <input type="color" value={newDiffColor} onChange={e => setNewDiffColor(e.target.value)} className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer" />
                <button onClick={addDiff} className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600">添加</button>
              </div>
            </div>
          </div>
        )
    }
  }

  return (
    <div>
      {/* Sub-tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6">
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

      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
        }`}>
          {msg}
        </div>
      )}

      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-5 mb-6">
        {renderTabContent()}
      </div>

      <div className="flex gap-3 items-center">
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
        >
          {saving ? '保存中...' : '保存配置'}
        </button>
        <button
          onClick={handleRestart}
          disabled={restarting}
          className="px-6 py-2 border border-yellow-500 text-yellow-700 text-sm hover:bg-yellow-50 disabled:opacity-50 dark:border-yellow-800 dark:text-yellow-400 dark:hover:bg-yellow-900/20"
        >
          {restarting ? '重启中...' : '重启服务'}
        </button>
        <p className="text-xs text-gray-400 dark:text-gray-500 ml-2">服务器/OAuth/管理员密码修改需重启服务才能生效。难度配置保存后立即生效。</p>
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

  const loadBackups = async () => {
    setLoading(true)
    try {
      const res = await apiFetch<{ success: boolean; backups: BackupEntry[] }>('/admin/backups')
      if (res.success) setBackups(res.backups)
    } catch (err) { setMsg(`加载备份列表失败: ${err}`) }
    finally { setLoading(false) }
  }

  useEffect(() => { loadBackups() }, [])

  const handleCreate = async () => {
    setCreating(true); setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string; backup?: string }>('/admin/backup', { method: 'POST' })
      if (!res.success) { setMsg(`备份失败: ${res.message}`); return }
      setMsg(`备份已创建: ${res.backup}`)
      loadBackups()
      setTimeout(() => setMsg(''), 5000)
    } catch (err) { setMsg(`备份失败: ${err}`) }
    finally { setCreating(false) }
  }

  const handleRestore = async (name: string) => {
    if (!window.confirm(`确定要从备份「${name}」恢复吗？当前数据将被覆盖，但会自动生成安全快照。`)) return
    setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/admin/backup/restore/${encodeURIComponent(name)}`, { method: 'POST' })
      if (!res.success) { setMsg(`恢复失败: ${res.message}`); return }
      setMsg(`✅ ${res.message}`)
      loadBackups()
      setTimeout(() => setMsg(''), 8000)
    } catch (err) { setMsg(`恢复失败: ${err}`) }
  }

  const handleDelete = async (name: string) => {
    if (!window.confirm(`确定要删除备份「${name}」吗？此操作不可撤销。`)) return
    setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/admin/backup/${encodeURIComponent(name)}`, { method: 'DELETE' })
      if (!res.success) { setMsg(`删除失败: ${res.message}`); return }
      setMsg(`已删除: ${name}`)
      loadBackups()
      setTimeout(() => setMsg(''), 3000)
    } catch (err) { setMsg(`删除失败: ${err}`) }
  }

  const triggerDownload = (content: string, filename: string, mime: string) => {
    const blob = new Blob([content], { type: mime })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url; a.download = filename
    document.body.appendChild(a); a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  const handleExportData = async () => {
    setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; content?: string; filename?: string; mime?: string; message?: string }>('/admin/export/data')
      if (!res.success) { setMsg(`导出失败: ${res.message}`); return }
      triggerDownload(res.content!, res.filename!, res.mime!)
      setMsg(`已导出: ${res.filename}`)
      setTimeout(() => setMsg(''), 3000)
    } catch (err) { setMsg(`导出失败: ${err}`) }
  }

  const handleExportConfig = async () => {
    setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; content?: string; filename?: string; mime?: string; message?: string }>('/admin/export/config')
      if (!res.success) { setMsg(`导出失败: ${res.message}`); return }
      triggerDownload(res.content!, res.filename!, res.mime!)
      setMsg(`已导出: ${res.filename}`)
      setTimeout(() => setMsg(''), 3000)
    } catch (err) { setMsg(`导出失败: ${err}`) }
  }

  const formatSize = (bytes: number) => {
    if (bytes > 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    if (bytes > 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${bytes} B`
  }

  return (
    <div>
      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.startsWith('✅') ? 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
          : msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300'
          : 'bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-700'
        }`}>
          {msg}
        </div>
      )}

      {/* Export */}
      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">导出</h2>
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">导出当前数据或配置文件，下载到本地。</p>
        <div className="flex items-center gap-3">
          <button onClick={handleExportData} className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800">
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
              导出数据文件
            </span>
          </button>
          <button onClick={handleExportConfig} className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-200 text-sm hover:bg-gray-100 dark:hover:bg-gray-800">
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
              导出配置文件
            </span>
          </button>
        </div>
      </div>

      {/* Backup */}
      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-5">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">备份管理</h2>
        <div className="flex items-center gap-3 mb-4">
          <button onClick={handleCreate} disabled={creating} className="px-5 py-2 bg-gray-800 dark:bg-gray-700 text-white text-sm font-medium hover:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50">
            {creating ? '创建中...' : '创建备份'}
          </button>
          <button onClick={loadBackups} disabled={loading} className="px-5 py-2 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50">
            刷新
          </button>
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
                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 ml-7">{formatSize(b.size)} · {b.modified}</div>
                </div>
                <div className="flex items-center gap-2 ml-4 shrink-0">
                  <button onClick={() => handleRestore(b.name)} className="px-3 py-1.5 text-xs border border-green-600 dark:border-green-800 text-green-700 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20">恢复</button>
                  <button onClick={() => handleDelete(b.name)} className="px-3 py-1.5 text-xs border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20">删除</button>
                </div>
              </div>
            ))}
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">共 {backups.length} 个备份 · 恢复操作会自动创建当前数据的 `pre_restore_` 安全快照</p>
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
  // Tags state
  const [tags, setTags] = useState<DiscussionTag[]>([])
  const [newTagName, setNewTagName] = useState('')
  const [newTagColor, setNewTagColor] = useState('#6366f1')
  const [newTagDesc, setNewTagDesc] = useState('')
  const [editingTag, setEditingTag] = useState<string | null>(null)
  const [editTagName, setEditTagName] = useState('')
  const [editTagColor, setEditTagColor] = useState('')
  const [editTagDesc, setEditTagDesc] = useState('')
  const [tagMsg, setTagMsg] = useState('')

  // Emojis state
  const [emojis, setEmojis] = useState<DiscussionEmoji[]>([])
  const [newEmojiChar, setNewEmojiChar] = useState('')
  const [newEmojiName, setNewEmojiName] = useState('')
  const [editingEmoji, setEditingEmoji] = useState<string | null>(null)
  const [editEmojiChar, setEditEmojiChar] = useState('')
  const [editEmojiName, setEditEmojiName] = useState('')
  const [emojiMsg, setEmojiMsg] = useState('')

  const loadTags = () => { apiFetch<DiscussionTag[]>('/discussions/tags').then(setTags).catch(() => {}) }
  const loadEmojis = () => { apiFetch<DiscussionEmoji[]>('/discussions/emojis').then(setEmojis).catch(() => {}) }
  useEffect(() => { loadTags(); loadEmojis() }, [])

  // Tag handlers
  const handleCreateTag = async () => {
    if (!newTagName.trim()) { setTagMsg('标签名不能为空'); return }
    setTagMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/discussions/tags', { method: 'POST', body: JSON.stringify({ name: newTagName.trim(), color: newTagColor, description: newTagDesc.trim() }) })
      if (res.success) { setNewTagName(''); setNewTagColor('#6366f1'); setNewTagDesc(''); loadTags() }
      setTagMsg(res.message)
    } catch { setTagMsg('创建失败') }
  }

  const handleUpdateTag = async (id: string) => {
    setTagMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/discussions/tags/${id}`, { method: 'PUT', body: JSON.stringify({ name: editTagName.trim() || undefined, color: editTagColor.trim() || undefined, description: editTagDesc.trim() || undefined }) })
      if (res.success) { setEditingTag(null); loadTags() }
      setTagMsg(res.message)
    } catch { setTagMsg('更新失败') }
  }

  const handleDeleteTag = async (id: string) => {
    if (!confirm('确定删除此标签？')) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/discussions/tags/${id}`, { method: 'DELETE' })
      setTagMsg(res.message); if (res.success) loadTags()
    } catch { setTagMsg('删除失败') }
  }

  // Emoji handlers
  const handleCreateEmoji = async () => {
    if (!newEmojiChar.trim()) { setEmojiMsg('表情字符不能为空'); return }
    if ([...newEmojiChar.trim()].length !== 1) { setEmojiMsg('表情必须是单个 Unicode 字符'); return }
    if (!newEmojiName.trim()) { setEmojiMsg('表情名称不能为空'); return }
    setEmojiMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/discussions/emojis', { method: 'POST', body: JSON.stringify({ char: newEmojiChar.trim(), name: newEmojiName.trim() }) })
      if (res.success) { setNewEmojiChar(''); setNewEmojiName(''); loadEmojis() }
      setEmojiMsg(res.message)
    } catch { setEmojiMsg('创建失败') }
  }

  const handleUpdateEmoji = async (id: string) => {
    setEmojiMsg('')
    try {
      const body: Record<string, string> = {}
      if (editEmojiChar.trim()) body.char = editEmojiChar.trim()
      if (editEmojiName.trim()) body.name = editEmojiName.trim()
      const res = await apiFetch<{ success: boolean; message: string }>(`/discussions/emojis/${id}`, { method: 'PUT', body: JSON.stringify(body) })
      if (res.success) { setEditingEmoji(null); loadEmojis() }
      setEmojiMsg(res.message)
    } catch { setEmojiMsg('更新失败') }
  }

  const handleDeleteEmoji = async (id: string) => {
    if (!confirm('确定删除此表情？')) return
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(`/discussions/emojis/${id}`, { method: 'DELETE' })
      setEmojiMsg(res.message); if (res.success) loadEmojis()
    } catch { setEmojiMsg('删除失败') }
  }

  return (
    <div>
      {/* ──── Tags ──── */}
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

      {/* ──── Emojis ──── */}
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
