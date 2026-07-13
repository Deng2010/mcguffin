import type { JoinRequest, TeamMember } from '../types'
import { apiFetch } from './api'

export async function getMembers(): Promise<TeamMember[]> {
  return apiFetch<TeamMember[]>('/team/members')
}

export async function getRequests(): Promise<JoinRequest[]> {
  return apiFetch<JoinRequest[]>('/team/requests')
}

export async function reviewRequest(
  requestId: string,
  action: 'approve' | 'reject'
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/team/review/${requestId}/${action}`, {
    method: 'POST',
  })
}

export async function changeMemberRole(
  userId: string,
  role: 'admin' | 'member'
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/team/members/role/${userId}`, {
    method: 'POST',
    body: JSON.stringify({ role }),
  })
}

export async function removeMember(userId: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/team/members/remove/${userId}`, {
    method: 'POST',
  })
}

export async function applyToJoin(reason: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/team/apply', {
    method: 'POST',
    body: JSON.stringify({ reason }),
  })
}
