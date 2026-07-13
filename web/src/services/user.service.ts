import type { User } from '../types'
import { apiFetch } from './api'

export interface UpdateProfilePayload {
  display_name?: string
  bio?: string
  avatar_url?: string | null
  email?: string | null
}

export async function getMe(): Promise<User> {
  return apiFetch<User>('/user/me')
}

export async function checkNameAvailable(name: string): Promise<{ available: boolean }> {
  return apiFetch<{ available: boolean }>(`/user/check-name?name=${encodeURIComponent(name)}`)
}

export async function getPublicProfile(username: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/user/profile/${encodeURIComponent(username)}`)
}

export async function updateProfile(body: UpdateProfilePayload): Promise<User> {
  return apiFetch<User>('/user/profile', {
    method: 'PUT',
    body: JSON.stringify(body),
  })
}
