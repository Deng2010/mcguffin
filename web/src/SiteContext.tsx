import { useState, useEffect, useContext, createContext, useCallback, type ReactNode } from 'react'
import { apiFetch } from './api'

// ============== Context Type ==============

interface SiteInfo {
  name: string
  version: string
  description: string
  title: string
  difficulty_order: string[]
  showcase_problem_ids: string[]
  showcase_contest_ids: string[]
}

interface SiteContextType {
  siteInfo: SiteInfo | null
  updateDescription: (description: string) => Promise<{ success: boolean; message: string }>
  refresh: () => void
}

const SiteContext = createContext<SiteContextType | null>(null)

// ============== Hook ==============

export function useSite(): SiteContextType {
  const ctx = useContext(SiteContext)
  if (!ctx) throw new Error('useSite must be used within SiteProvider')
  return ctx
}

// ============== Provider ==============

export function SiteProvider({ children }: { children: ReactNode }) {
  const [siteInfo, setSiteInfo] = useState<SiteInfo | null>(null)

  const fetchInfo = useCallback(() => {
    apiFetch<SiteInfo>('/site/info')
      .then(setSiteInfo)
      .catch(() => setSiteInfo({ name: 'McGuffin', version: '0.1.0', description: '', title: 'McGuffin', difficulty_order: [], showcase_problem_ids: [], showcase_contest_ids: [] }))
  }, [])

  useEffect(() => { fetchInfo() }, [fetchInfo])

  // Sync browser tab title with site name
  useEffect(() => {
    document.title = siteInfo?.title || 'McGuffin'
  }, [siteInfo])

  const updateDescription = async (description: string): Promise<{ success: boolean; message: string }> => {
    try {
      const res = await apiFetch<{ success: boolean; message: string; description: string }>(
        '/site/description',
        { method: 'PUT', body: JSON.stringify({ description }) },
      )
      if (res.success && siteInfo) {
        setSiteInfo({ ...siteInfo, description: res.description })
      }
      return { success: res.success, message: res.message }
    } catch (err) {
      return { success: false, message: `请求失败: ${err}` }
    }
  }

  return (
    <SiteContext.Provider value={{ siteInfo, updateDescription, refresh: fetchInfo }}>
      {children}
    </SiteContext.Provider>
  )
}
