import { useState, useEffect } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

interface ConfigData {
  server: { site_url: string; port: number; data_file: string }
  admin: { password: string; display_name: string }
  site: { name: string }
  oauth: { cp_client_id: string; cp_client_secret: string }
  difficulty: Record<string, { label: string; color: string }>
}

interface DifficultyEntry {
  name: string
  label: string
  color: string
}

type TabId = 'server' | 'admin' | 'site' | 'oauth' | 'difficulty'

export default function AdminConfigPage() {
  const { user } = useAuth()
  const [config, setConfig] = useState<ConfigData | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [restarting, setRestarting] = useState(false)
  const [msg, setMsg] = useState('')
  const [activeTab, setActiveTab] = useState<TabId>('server')

  // Edit state
  const [siteUrl, setSiteUrl] = useState('')
  const [port, setPort] = useState('')
  const [dataFile, setDataFile] = useState('')
  const [adminPassword, setAdminPassword] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [siteName, setSiteName] = useState('')
  const [cpClientId, setCpClientId] = useState('')
  const [cpClientSecret, setCpClientSecret] = useState('')
  const [difficulties, setDifficulties] = useState<DifficultyEntry[]>([])
  const [newDiffName, setNewDiffName] = useState('')
  const [newDiffLabel, setNewDiffLabel] = useState('')
  const [newDiffColor, setNewDiffColor] = useState('#888888')

  const isSuperadmin = user?.role === 'superadmin'

  const tabs: { id: TabId; label: string }[] = [
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
      if (!res.success || !res.config) {
        setMsg(`加载配置失败: ${res.message}`)
        return
      }
      setConfig(res.config)
      setSiteUrl(res.config.server.site_url)
      setPort(String(res.config.server.port))
      setDataFile(res.config.server.data_file)
      setAdminPassword(res.config.admin.password)
      setDisplayName(res.config.admin.display_name)
      setSiteName(res.config.site.name)
      setCpClientId(res.config.oauth.cp_client_id)
      setCpClientSecret(res.config.oauth.cp_client_secret)
      const diffArr: DifficultyEntry[] = Object.entries(res.config.difficulty).map(([name, fields]) => ({
        name,
        label: fields.label,
        color: fields.color,
      }))
      setDifficulties(diffArr)
    } catch (err) {
      setMsg(`加载配置失败: ${err}`)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { if (isSuperadmin) loadConfig() }, [isSuperadmin])

  const updateDiff = (idx: number, field: keyof DifficultyEntry, value: string) => {
    setDifficulties(prev => {
      const next = [...prev]
      next[idx] = { ...next[idx], [field]: value }
      return next
    })
  }

  const removeDiff = (idx: number) => {
    setDifficulties(prev => prev.filter((_, i) => i !== idx))
  }

  const addDiff = () => {
    const name = newDiffName.trim()
    if (!name) return
    if (difficulties.some(d => d.name === name)) {
      setMsg(`难度 "${name}" 已存在`)
      return
    }
    setDifficulties(prev => [...prev, { name, label: newDiffLabel.trim() || name, color: newDiffColor }])
    setNewDiffName('')
    setNewDiffLabel('')
    setNewDiffColor('#888888')
  }

  const handleSave = async () => {
    if (!config) return
    setSaving(true)
    setMsg('')
    try {
      const diffObj: Record<string, { label: string; color: string }> = {}
      for (const d of difficulties) {
        if (d.name.trim()) diffObj[d.name.trim()] = { label: d.label.trim() || d.name, color: d.color }
      }
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/config', {
        method: 'PUT',
        body: JSON.stringify({
          server: { site_url: siteUrl, port: parseInt(port) || 3000, data_file: dataFile },
          admin: { password: adminPassword, display_name: displayName },
          site: { name: siteName },
          oauth: { cp_client_id: cpClientId, cp_client_secret: cpClientSecret },
          difficulty: diffObj,
        }),
      })
      if (!res.success) { setMsg(`保存失败: ${res.message}`); return }
      setMsg(res.message)
      setTimeout(() => setMsg(''), 5000)
    } catch (err) {
      setMsg(`保存失败: ${err}`)
    } finally {
      setSaving(false)
    }
  }

  const handleRestart = async () => {
    if (!window.confirm('确定要重启服务吗？服务会短暂中断（约2-3秒）。')) return
    setRestarting(true)
    setMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/restart', { method: 'POST' })
      if (!res.success) { setMsg(`重启失败: ${res.message}`); setRestarting(false); return }
      setMsg('服务正在重启，页面将在几秒后重载...')
      setTimeout(() => window.location.reload(), 5000)
    } catch (err) {
      setMsg(`重启失败: ${err}`)
      setRestarting(false)
    }
  }

  if (!isSuperadmin) {
    return <div className="p-6 text-center py-12 text-gray-400">权限不足</div>
  }

  if (loading) {
    return <div className="p-6 text-center py-12 text-gray-400">加载配置中...</div>
  }

  const inputClass = "w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm"

  const renderTabContent = () => {
    switch (activeTab) {
      case 'server':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">站点 URL</label>
              <input type="text" value={siteUrl} onChange={e => setSiteUrl(e.target.value)} className={inputClass} placeholder="https://lba-oi.team" />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">端口</label>
                <input type="number" value={port} onChange={e => setPort(e.target.value)} className={inputClass} placeholder="3000" />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">数据文件</label>
                <input type="text" value={dataFile} onChange={e => setDataFile(e.target.value)} className={inputClass} placeholder="mcguffin_data.json" />
              </div>
            </div>
          </div>
        )

      case 'admin':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">登录密码</label>
              <input type="text" value={adminPassword} onChange={e => setAdminPassword(e.target.value)} className={inputClass} />
              <p className="text-xs text-gray-400 mt-1">修改后需重启服务生效</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">显示名称</label>
              <input type="text" value={displayName} onChange={e => setDisplayName(e.target.value)} className={inputClass} />
            </div>
          </div>
        )

      case 'site':
        return (
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">站点名称</label>
            <input type="text" value={siteName} onChange={e => setSiteName(e.target.value)} className={inputClass} placeholder="McGuffin" />
          </div>
        )

      case 'oauth':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Client ID</label>
              <input type="text" value={cpClientId} onChange={e => setCpClientId(e.target.value)} className={inputClass} />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Client Secret</label>
              <input type="text" value={cpClientSecret} onChange={e => setCpClientSecret(e.target.value)} className={inputClass} />
            </div>
            <p className="text-xs text-gray-400">修改后需重启服务生效</p>
          </div>
        )

      case 'difficulty':
        return (
          <div>
            <p className="text-xs text-gray-500 mb-3">添加、编辑或删除难度等级。名称用作内部标识（如 Easy），标签显示给用户（如 简单），颜色用于 UI 展示。</p>
            <div className="space-y-3">
              {difficulties.map((d, i) => (
                <div key={i} className="flex items-center gap-2 bg-gray-50 p-2">
                  <input
                    type="text" value={d.name} onChange={e => updateDiff(i, 'name', e.target.value)}
                    className="flex-1 px-3 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
                    placeholder="名称"
                  />
                  <input
                    type="text" value={d.label} onChange={e => updateDiff(i, 'label', e.target.value)}
                    className="flex-1 px-3 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
                    placeholder="标签"
                  />
                  <input
                    type="color" value={d.color} onChange={e => updateDiff(i, 'color', e.target.value)}
                    className="w-10 h-9 p-0.5 border border-gray-300 cursor-pointer"
                  />
                  <span className="text-xs text-gray-500 w-20">{d.color}</span>
                  <button onClick={() => removeDiff(i)} className="px-2 py-1 text-red-600 text-sm hover:bg-red-50">删除</button>
                </div>
              ))}
              {/* Add new */}
              <div className="flex items-center gap-2 bg-blue-50 p-2 border border-dashed border-blue-300">
                <input
                  type="text" value={newDiffName} onChange={e => setNewDiffName(e.target.value)}
                  className="flex-1 px-3 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
                  placeholder="新难度名称"
                />
                <input
                  type="text" value={newDiffLabel} onChange={e => setNewDiffLabel(e.target.value)}
                  className="flex-1 px-3 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500"
                  placeholder="显示标签"
                />
                <input
                  type="color" value={newDiffColor} onChange={e => setNewDiffColor(e.target.value)}
                  className="w-10 h-9 p-0.5 border border-gray-300 cursor-pointer"
                />
                <button onClick={addDiff} className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700">添加</button>
              </div>
            </div>
          </div>
        )
    }
  }

  return (
    <div className="p-6 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-800 mb-6">系统配置</h1>

      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700' : 'bg-green-50 border-green-300 text-green-700'
        }`}>
          {msg}
        </div>
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 mb-6">
        {tabs.map(tab => (
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
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="bg-white border border-gray-300 p-5 mb-6">
        {renderTabContent()}
      </div>

      {/* Actions */}
      <div className="flex gap-3 items-center">
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50"
        >
          {saving ? '保存中...' : '保存配置'}
        </button>
        <button
          onClick={handleRestart}
          disabled={restarting}
          className="px-6 py-2 border border-yellow-500 text-yellow-700 text-sm hover:bg-yellow-50 disabled:opacity-50"
        >
          {restarting ? '重启中...' : '重启服务'}
        </button>
        <p className="text-xs text-gray-400 ml-2">服务器/OAuth/管理员密码修改需重启服务才能生效。难度配置保存后立即生效。</p>
      </div>
    </div>
  )
}
