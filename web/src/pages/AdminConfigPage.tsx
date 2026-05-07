import { useState, useEffect } from 'react'
import { useAuth } from '../AuthContext'
import { apiFetch } from '../api'

interface ConfigData {
  server: { site_url: string; port: number; data_file: string }
  admin: { password: string; display_name: string }
  site: { name: string; title?: string | null; showcase_problems: number; showcase_contests: number; difficulty_order: string[] }
  oauth: { cp_client_id: string; cp_client_secret: string }
  difficulty: Record<string, { label: string; color: string }>
}

interface DifficultyEntry {
  name: string
  label: string
  color: string
}

interface ShowcaseItem {
  id: string
  title: string
  name?: string
  status?: string
}

type TabId = 'server' | 'admin' | 'site' | 'oauth' | 'difficulty' | 'showcase'

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
  const [siteTitle, setSiteTitle] = useState('')
  const [siteShowcaseProblems, setSiteShowcaseProblems] = useState('8')
  const [siteShowcaseContests, setSiteShowcaseContests] = useState('3')
  const [cpClientId, setCpClientId] = useState('')
  const [cpClientSecret, setCpClientSecret] = useState('')
  const [difficulties, setDifficulties] = useState<DifficultyEntry[]>([])
  const [difficultyOrder, setDifficultyOrder] = useState<string[]>([])
  const [newDiffName, setNewDiffName] = useState('')
  const [newDiffLabel, setNewDiffLabel] = useState('')
  const [newDiffColor, setNewDiffColor] = useState('#888888')

  // Showcase state
  const [showcaseProblems, setShowcaseProblems] = useState<ShowcaseItem[]>([])
  const [showcaseContests, setShowcaseContests] = useState<ShowcaseItem[]>([])
  const [allProblems, setAllProblems] = useState<ShowcaseItem[]>([])
  const [allContests, setAllContests] = useState<ShowcaseItem[]>([])
  const [selectedShowcaseProblems, setSelectedShowcaseProblems] = useState<string[]>([])
  const [selectedShowcaseContests, setSelectedShowcaseContests] = useState<string[]>([])
  const [showcaseMsg, setShowcaseMsg] = useState('')
  const [showcaseSaving, setShowcaseSaving] = useState(false)

  const isSuperadmin = user?.role === 'superadmin'

  const tabs: { id: TabId; label: string }[] = [
    { id: 'server', label: '服务器' },
    { id: 'admin', label: '管理员' },
    { id: 'site', label: '站点' },
    { id: 'oauth', label: 'OAuth' },
    { id: 'difficulty', label: '难度' },
    { id: 'showcase', label: '展板管理' },
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
      setSiteTitle(res.config.site.title ?? '')
      setSiteShowcaseProblems(String(res.config.site.showcase_problems ?? 8))
      setSiteShowcaseContests(String(res.config.site.showcase_contests ?? 3))
      setCpClientId(res.config.oauth.cp_client_id)
      setCpClientSecret(res.config.oauth.cp_client_secret)
      const diffArr: DifficultyEntry[] = Object.entries(res.config.difficulty).map(([name, fields]) => ({
        name,
        label: fields.label,
        color: fields.color,
      }))
      setDifficulties(diffArr)
      setDifficultyOrder(res.config.site.difficulty_order ?? [])
    } catch (err) {
      setMsg(`加载配置失败: ${err}`)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { if (isSuperadmin) loadConfig() }, [isSuperadmin])

  // Load showcase data when tab switches to 'showcase'
  useEffect(() => {
    if (activeTab !== 'showcase' || !isSuperadmin) return
    const loadShowcase = async () => {
      try {
        const [probRes, contestRes, showcaseRes] = await Promise.all([
          apiFetch<ShowcaseItem[]>('/problems').catch(() => [] as ShowcaseItem[]),
          apiFetch<ShowcaseItem[]>('/contests').catch(() => [] as ShowcaseItem[]),
          apiFetch<{ success: boolean; problem_ids: string[]; contest_ids: string[] }>('/admin/showcase').catch(() => ({ success: true, problem_ids: [], contest_ids: [] })),
        ])
        setAllProblems(probRes)
        setAllContests(contestRes)
        setSelectedShowcaseProblems(showcaseRes.problem_ids || [])
        setSelectedShowcaseContests(showcaseRes.contest_ids || [])
      } catch { /* ignore */ }
    }
    loadShowcase()
  }, [activeTab, isSuperadmin])

  const updateDiff = (idx: number, field: keyof DifficultyEntry, value: string) => {
    setDifficulties(prev => {
      const next = [...prev]
      next[idx] = { ...next[idx], [field]: value }
      return next
    })
  }

  const moveDiff = (idx: number, direction: -1 | 1) => {
    const target = idx + direction
    if (target < 0 || target >= difficulties.length) return
    setDifficulties(prev => {
      const next = [...prev]
      const temp = next[idx]
      next[idx] = next[target]
      next[target] = temp
      return next
    })
    // Also update the order
    setDifficultyOrder(prev => {
      const next = [...prev]
      const temp = next[idx]
      next[idx] = next[target]
      next[target] = temp
      return next
    })
  }

  const removeDiff = (idx: number) => {
    const removedName = difficulties[idx].name
    setDifficulties(prev => prev.filter((_, i) => i !== idx))
    setDifficultyOrder(prev => prev.filter(n => n !== removedName))
  }

  const addDiff = () => {
    const name = newDiffName.trim()
    if (!name) return
    if (difficulties.some(d => d.name === name)) {
      setMsg(`难度 "${name}" 已存在`)
      return
    }
    setDifficulties(prev => {
      const next = [...prev, { name, label: newDiffLabel.trim() || name, color: newDiffColor }]
      setDifficultyOrder(prevOrder => [...prevOrder, name])
      return next
    })
    setNewDiffName('')
    setNewDiffLabel('')
    setNewDiffColor('#888888')
  }

  const toggleShowcaseProblem = (id: string) => {
    setSelectedShowcaseProblems(prev =>
      prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]
    )
  }

  const toggleShowcaseContest = (id: string) => {
    setSelectedShowcaseContests(prev =>
      prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]
    )
  }

  const handleMoveShowcase = (type: 'problem' | 'contest', idx: number, direction: -1 | 1) => {
    const setter = type === 'problem' ? setSelectedShowcaseProblems : setSelectedShowcaseContests
    const list = type === 'problem' ? selectedShowcaseProblems : selectedShowcaseContests
    const target = idx + direction
    if (target < 0 || target >= list.length) return
    setter(prev => {
      const next = [...prev]
      const temp = next[idx]
      next[idx] = next[target]
      next[target] = temp
      return next
    })
  }

  const handleSaveShowcase = async () => {
    setShowcaseSaving(true)
    setShowcaseMsg('')
    try {
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/showcase', {
        method: 'PUT',
        body: JSON.stringify({
          problem_ids: selectedShowcaseProblems,
          contest_ids: selectedShowcaseContests,
        }),
      })
      if (!res.success) { setShowcaseMsg(`保存失败: ${res.message}`); return }
      setShowcaseMsg(res.message)
      setTimeout(() => setShowcaseMsg(''), 3000)
    } catch (err) {
      setShowcaseMsg(`保存失败: ${err}`)
    } finally {
      setShowcaseSaving(false)
    }
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
      // Build difficulty_order: use current array order, fall back to difficulties order
      const order = difficultyOrder.length > 0
        ? difficultyOrder
        : difficulties.filter(d => d.name.trim()).map(d => d.name.trim())
      const res = await apiFetch<{ success: boolean; message: string }>('/admin/config', {
        method: 'PUT',
        body: JSON.stringify({
          server: { site_url: siteUrl, port: parseInt(port) || 3000, data_file: dataFile },
          admin: { password: adminPassword, display_name: displayName },
          site: { name: siteName, title: siteTitle || undefined, showcase_problems: parseInt(siteShowcaseProblems) || 8, showcase_contests: parseInt(siteShowcaseContests) || 3, difficulty_order: order },
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
    return <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">权限不足</div>
  }

  if (loading) {
    return <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">加载配置中...</div>
  }

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
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">展板题目数</label>
                <input type="number" value={siteShowcaseProblems} onChange={e => setSiteShowcaseProblems(e.target.value)} className={inputClass} placeholder="8" min={0} />
                <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">首页展板显示的题目数量，默认 8</p>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">展板比赛数</label>
                <input type="number" value={siteShowcaseContests} onChange={e => setSiteShowcaseContests(e.target.value)} className={inputClass} placeholder="3" min={0} />
                <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">首页展板显示的比赛数量，默认 3</p>
              </div>
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
              添加、编辑或删除难度等级。名称用作内部标识（如 Easy），标签显示给用户（如 简单），颜色用于 UI 展示。
              使用 ↑↓ 按钮调整显示顺序。
            </p>
            <div className="space-y-3">
              {difficulties.map((d, i) => (
                <div key={i} className="flex items-center gap-2 bg-gray-50 dark:bg-gray-800/50 p-2">
                  <div className="flex flex-col gap-0.5">
                    <button
                      onClick={() => moveDiff(i, -1)}
                      disabled={i === 0}
                      className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1"
                      title="上移"
                    >↑</button>
                    <button
                      onClick={() => moveDiff(i, 1)}
                      disabled={i === difficulties.length - 1}
                      className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1"
                      title="下移"
                    >↓</button>
                  </div>
                  <span className="text-xs text-gray-400 w-5 text-right">{i + 1}</span>
                  <input
                    type="text" value={d.name} onChange={e => updateDiff(i, 'name', e.target.value)}
                    className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
                    placeholder="名称"
                  />
                  <input
                    type="text" value={d.label} onChange={e => updateDiff(i, 'label', e.target.value)}
                    className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
                    placeholder="标签"
                  />
                  <input
                    type="color" value={d.color} onChange={e => updateDiff(i, 'color', e.target.value)}
                    className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer"
                  />
                  <span className="text-xs text-gray-500 dark:text-gray-400 w-20">{d.color}</span>
                  <button onClick={() => removeDiff(i)} className="px-2 py-1 text-red-600 text-sm hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20">删除</button>
                </div>
              ))}
              {/* Add new */}
              <div className="flex items-center gap-2 bg-blue-50 dark:bg-blue-900/30 p-2 border border-dashed border-blue-300 dark:border-blue-800">
                <input
                  type="text" value={newDiffName} onChange={e => setNewDiffName(e.target.value)}
                  className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
                  placeholder="新难度名称"
                />
                <input
                  type="text" value={newDiffLabel} onChange={e => setNewDiffLabel(e.target.value)}
                  className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
                  placeholder="显示标签"
                />
                <input
                  type="color" value={newDiffColor} onChange={e => setNewDiffColor(e.target.value)}
                  className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer"
                />
                <button onClick={addDiff} className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600">添加</button>
              </div>
            </div>
          </div>
        )

      case 'showcase':
        return (
          <div>
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
              选择要在首页展板展示的题目和比赛。勾选项目并按 ↑↓ 调整顺序。展板最多显示数量在「站点」标签页中设置。
            </p>

            {showcaseMsg && (
              <div className={`mb-4 p-3 text-sm border ${
                showcaseMsg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
              }`}>
                {showcaseMsg}
              </div>
            )}

            {/* Problems */}
            <div className="mb-6">
              <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">
                展板题目 ({selectedShowcaseProblems.length})
              </h3>
              <div className="space-y-1 max-h-64 overflow-y-auto">
                {allProblems.filter(p => p.status === 'published').map((p, i) => {
                  const isSelected = selectedShowcaseProblems.includes(p.id)
                  const selectedIdx = selectedShowcaseProblems.indexOf(p.id)
                  return (
                    <div key={p.id} className="flex items-center gap-2 py-1.5 px-2 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                      <input
                        type="checkbox"
                        checked={isSelected}
                        onChange={() => toggleShowcaseProblem(p.id)}
                        className="accent-gray-800 dark:accent-gray-400"
                      />
                      <span className={`text-sm flex-1 ${isSelected ? 'text-gray-800 dark:text-gray-100' : 'text-gray-400 dark:text-gray-500'}`}>
                        {p.title}
                      </span>
                      {isSelected && (
                        <div className="flex gap-1 shrink-0">
                          <button
                            onClick={() => handleMoveShowcase('problem', selectedIdx, -1)}
                            disabled={selectedIdx === 0}
                            className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                          >↑</button>
                          <button
                            onClick={() => handleMoveShowcase('problem', selectedIdx, 1)}
                            disabled={selectedIdx === selectedShowcaseProblems.length - 1}
                            className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                          >↓</button>
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>
            </div>

            {/* Contests */}
            <div className="mb-6">
              <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">
                展板比赛 ({selectedShowcaseContests.length})
              </h3>
              <div className="space-y-1 max-h-64 overflow-y-auto">
                {allContests.filter(c => c.status === 'public').map((c, i) => {
                  const isSelected = selectedShowcaseContests.includes(c.id)
                  const selectedIdx = selectedShowcaseContests.indexOf(c.id)
                  return (
                    <div key={c.id} className="flex items-center gap-2 py-1.5 px-2 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                      <input
                        type="checkbox"
                        checked={isSelected}
                        onChange={() => toggleShowcaseContest(c.id)}
                        className="accent-gray-800 dark:accent-gray-400"
                      />
                      <span className={`text-sm flex-1 ${isSelected ? 'text-gray-800 dark:text-gray-100' : 'text-gray-400 dark:text-gray-500'}`}>
                        {c.name}
                      </span>
                      {isSelected && (
                        <div className="flex gap-1 shrink-0">
                          <button
                            onClick={() => handleMoveShowcase('contest', selectedIdx, -1)}
                            disabled={selectedIdx === 0}
                            className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                          >↑</button>
                          <button
                            onClick={() => handleMoveShowcase('contest', selectedIdx, 1)}
                            disabled={selectedIdx === selectedShowcaseContests.length - 1}
                            className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                          >↓</button>
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>
            </div>

            <div className="flex gap-3">
              <button
                onClick={handleSaveShowcase}
                disabled={showcaseSaving}
                className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
              >
                {showcaseSaving ? '保存中...' : '保存展板'}
              </button>
              <p className="text-xs text-gray-400 dark:text-gray-500 self-center">保存后立即生效，无需重启服务</p>
            </div>
          </div>
        )
    }
  }

  return (
    <div className="p-6 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-6">系统配置</h1>

      {msg && (
        <div className={`mb-4 p-3 text-sm border ${
          msg.includes('失败') ? 'bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300' : 'bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300'
        }`}>
          {msg}
        </div>
      )}

      {/* Tabs */}
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

      {/* Tab content */}
      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 p-5 mb-6">
        {renderTabContent()}
      </div>

      {activeTab !== 'showcase' && (
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
      )}
    </div>
  )
}
