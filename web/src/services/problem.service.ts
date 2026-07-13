import type { AdminPendingProblem, ProblemDetail, ProblemListItem, SubmitProblemPayload } from '../types'
import { apiFetch } from './api'

export async function getProblems(all?: boolean): Promise<ProblemListItem[]> {
  const path = all ? '/problems?all=true' : '/problems'
  return apiFetch<ProblemListItem[]>(path)
}

export async function getProblemDetail(id: string): Promise<ProblemDetail> {
  return apiFetch<ProblemDetail>(`/problems/detail/${id}`)
}

export async function getAdminMembers(): Promise<Record<string, any>[]> {
  return apiFetch<Record<string, any>[]>('/problems/admin/members')
}

export async function createProblem(body: SubmitProblemPayload): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/problems', {
    method: 'POST',
    body: JSON.stringify(body),
  })
}

export async function updateProblem(id: string, body: SubmitProblemPayload): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  })
}

export async function deleteProblem(id: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/${id}`, { method: 'DELETE' })
}

export async function claimProblem(id: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/claim/${id}`, { method: 'POST' })
}

export async function unclaimProblem(id: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/unclaim/${id}`, { method: 'POST' })
}

export async function reviewProblem(
  id: string,
  action: 'approve' | 'reject' | 'publish',
  reason?: string
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/review/${id}/${action}`, {
    method: 'POST',
    body: reason ? JSON.stringify({ reason }) : undefined,
  })
}

export async function setProblemVisibility(id: string, userIds: string[]): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/visibility/${id}`, {
    method: 'POST',
    body: JSON.stringify({ visible_to: userIds }),
  })
}

export async function setProblemContest(id: string, contestId: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/contest/${id}`, {
    method: 'POST',
    body: JSON.stringify({ contest_id: contestId }),
  })
}

export async function submitVerifierSolution(id: string, solution: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/problems/verifier-solution/${id}`, {
    method: 'POST',
    body: JSON.stringify({ solution }),
  })
}
