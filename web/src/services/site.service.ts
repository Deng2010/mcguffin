import { apiFetch } from './api'

export interface SiteInfo {
  name: string
  description: string
  version: string
  showcase?: {
    problems: Record<string, any>[]
    contests: Record<string, any>[]
  }
}

export interface ShowcasePayload {
  problem_ids: string[]
  contest_ids: string[]
}

export async function getSiteInfo(): Promise<SiteInfo> {
  return apiFetch<SiteInfo>('/site/info')
}

export async function updateSiteDescription(description: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/site/description', {
    method: 'PUT',
    body: JSON.stringify({ description }),
  })
}

export async function updateShowcase(problemIds: string[], contestIds: string[]): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/showcase', {
    method: 'PUT',
    body: JSON.stringify({ problem_ids: problemIds, contest_ids: contestIds }),
  })
}
