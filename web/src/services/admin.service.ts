import type { User } from '../types'
import { apiFetch } from './api'

export interface ConfigValue {
  server?: Record<string, any>
  site?: Record<string, any>
  oauth?: Record<string, any>
  difficulty?: Record<string, any>
  permissions?: Record<string, string[]>
}

export interface Group {
  id: string
  name: string
  permissions: string[]
  created_at?: string
}

export interface BackupItem {
  name: string
  size: number
  created_at: string
}

export async function getConfig(): Promise<ConfigValue> {
  return apiFetch<ConfigValue>('/admin/config')
}

export async function updateConfig(body: ConfigValue): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/config', {
    method: 'PUT',
    body: JSON.stringify(body),
  })
}

export async function restartServer(): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/restart', { method: 'POST' })
}

export async function exportConfig(): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/export/config')
}

export async function getAdminUsers(): Promise<User[]> {
  return apiFetch<User[]>('/admin/users')
}

export async function changeUserRole(
  userId: string,
  role: 'admin' | 'member' | 'guest'
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/users/${userId}/role`, {
    method: 'POST',
    body: JSON.stringify({ role }),
  })
}

export async function removeUser(userId: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/users/${userId}/remove`, { method: 'POST' })
}

export async function updateUserPermissions(
  userId: string,
  permissions: string[]
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/users/${userId}/permissions`, {
    method: 'PUT',
    body: JSON.stringify({ permissions }),
  })
}

export async function updateUserGroups(
  userId: string,
  groupIds: string[]
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/users/${userId}/groups`, {
    method: 'PUT',
    body: JSON.stringify({ group_ids: groupIds }),
  })
}

export async function getGroups(): Promise<Group[]> {
  return apiFetch<Group[]>('/admin/groups')
}

export async function createGroup(body: Omit<Group, 'id'>): Promise<Group> {
  return apiFetch<Group>('/admin/groups', {
    method: 'POST',
    body: JSON.stringify(body),
  })
}

export async function updateGroup(id: string, body: Omit<Group, 'id'>): Promise<Group> {
  return apiFetch<Group>(`/admin/groups/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  })
}

export async function deleteGroup(id: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/groups/${id}`, { method: 'DELETE' })
}

export async function getBackups(): Promise<BackupItem[]> {
  return apiFetch<BackupItem[]>('/admin/backups')
}

export async function createBackup(): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/backup', { method: 'POST' })
}

export async function restoreBackup(name: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/backup/restore/${encodeURIComponent(name)}`, {
    method: 'POST',
  })
}

export async function deleteBackup(name: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/backup/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  })
}

export async function downloadBackup(name: string): Promise<Blob> {
  return apiFetch<Blob>(`/admin/backup/download/${encodeURIComponent(name)}`)
}

export async function restoreFromUpload(formData: FormData): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/backup/restore-upload', {
    method: 'POST',
    body: formData,
  })
}

export async function exportData(type: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/export/${encodeURIComponent(type)}`)
}

export async function exportDatabase(): Promise<Blob> {
  return apiFetch<Blob>('/admin/export/db')
}

export async function importData(content: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/import/data', {
    method: 'POST',
    body: JSON.stringify({ content }),
  })
}

export async function importConfig(content: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/import/config', {
    method: 'POST',
    body: JSON.stringify({ content }),
  })
}

export async function updateContestAcl(
  id: string,
  visibleTo: string[],
  editableBy: string[]
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/acl/contest/${id}`, {
    method: 'PUT',
    body: JSON.stringify({ visible_to: visibleTo, editable_by: editableBy }),
  })
}

export async function updateProblemAcl(
  id: string,
  editableBy: string[]
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/acl/problem/${id}`, {
    method: 'PUT',
    body: JSON.stringify({ editable_by: editableBy }),
  })
}

export async function updatePostAcl(
  id: string,
  visibleTo: string[],
  editableBy: string[]
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/admin/acl/post/${id}`, {
    method: 'PUT',
    body: JSON.stringify({ visible_to: visibleTo, editable_by: editableBy }),
  })
}
