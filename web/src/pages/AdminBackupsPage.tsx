import { useState, useEffect } from 'react'
import { apiFetch } from '../api'

interface BackupEntry {
  name: string
  size: number
  modified: string
}

export default function AdminBackupsPage() {
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

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">导出</h2>
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">导出当前数据或配置文件，下载到本地。</p>
        <div className="flex items-center gap-3">
          <button onClick={() => doExport('data')} className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800">
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
              导出数据文件
            </span>
          </button>
          <button onClick={() => doExport('config')} className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow text-gray-700 dark:text-gray-200 text-sm hover:bg-gray-100 dark:hover:bg-gray-800">
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
              导出配置文件
            </span>
          </button>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow p-5">
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
