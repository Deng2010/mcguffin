// ============== Plugin Data API client ==============
// These functions are called by React hooks. They talk to the
// generic backend data API at /api/v1/plugins/{plugin_id}/data/*

import { apiFetch } from '../../services/api'

function pluginApi<T>(pluginId: string, path: string, options?: RequestInit): Promise<T> {
  return apiFetch<T>(`/plugins/${encodeURIComponent(pluginId)}${path}`, options)
}

// ── KV ──

export async function getPluginData(
  pluginId: string, namespace: string, key: string,
): Promise<string> {
  const res = await pluginApi<{ value: string }>(
    pluginId,
    `/data?namespace=${encodeURIComponent(namespace)}&key=${encodeURIComponent(key)}`,
  )
  return res.value
}

export async function setPluginData(
  pluginId: string, namespace: string, key: string, value: string,
): Promise<void> {
  await pluginApi(pluginId, '/data', {
    method: 'POST',
    body: JSON.stringify({ namespace, key, value }),
  })
}

// ── Counters ──

export async function pluginAdd(
  pluginId: string, namespace: string, key: string, delta: number,
): Promise<number> {
  const res = await pluginApi<{ value: number }>(pluginId, '/data/add', {
    method: 'POST',
    body: JSON.stringify({ namespace, key, delta }),
  })
  return res.value
}

export const pluginIncr = (pid: string, ns: string, k: string) => pluginAdd(pid, ns, k, 1)
export const pluginDecr = (pid: string, ns: string, k: string) => pluginAdd(pid, ns, k, -1)

// ── Sets ──

export async function pluginSetAdd(
  pluginId: string, namespace: string, key: string, member: string,
): Promise<boolean> {
  const res = await pluginApi<{ added: boolean }>(pluginId, '/data/set-add', {
    method: 'POST',
    body: JSON.stringify({ namespace, key, member }),
  })
  return res.added
}

export async function pluginSetRemove(
  pluginId: string, namespace: string, key: string, member: string,
): Promise<boolean> {
  const res = await pluginApi<{ removed: boolean }>(pluginId, '/data/set-remove', {
    method: 'POST',
    body: JSON.stringify({ namespace, key, member }),
  })
  return res.removed
}

export async function pluginSetMembers(
  pluginId: string, namespace: string, key: string,
): Promise<string[]> {
  const res = await pluginApi<{ members: string[]; count: number }>(
    pluginId,
    `/data/set-members?namespace=${encodeURIComponent(namespace)}&key=${encodeURIComponent(key)}`,
  )
  return res.members
}

export async function pluginSetIsMember(
  pluginId: string, namespace: string, key: string, member: string,
): Promise<boolean> {
  const res = await pluginApi<{ is_member: boolean }>(
    pluginId,
    `/data/set-is-member?namespace=${encodeURIComponent(namespace)}&key=${encodeURIComponent(key)}&member=${encodeURIComponent(member)}`,
  )
  return res.is_member
}

// ── Keys ──

export async function pluginKeys(
  pluginId: string, namespace: string, prefix?: string,
): Promise<string[]> {
  const params = new URLSearchParams({ namespace })
  if (prefix) params.set('prefix', prefix)
  const res = await pluginApi<{ keys: string[] }>(pluginId, `/data/keys?${params}`)
  return res.keys
}

// ── Notifications ──

export async function pluginCreateNotification(
  pluginId: string, userId: string, title: string, body: string, link?: string,
): Promise<void> {
  await pluginApi(pluginId, '/notify', {
    method: 'POST',
    body: JSON.stringify({ user_id: userId, title, body, link }),
  })
}

// ── Files ──

export async function pluginWriteFile(
  pluginId: string, filePath: string, data: Blob | ArrayBuffer,
): Promise<{ path: string; size: number }> {
  const token = localStorage.getItem('auth_token')
  const res = await fetch(`/api/plugins/${encodeURIComponent(pluginId)}/files/${encodeURIComponent(filePath)}`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
    },
    body: data,
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }))
    throw new Error(err.message ?? '写入文件失败')
  }
  return res.json()
}

export async function pluginReadFile(
  pluginId: string, filePath: string,
): Promise<Blob> {
  const token = localStorage.getItem('auth_token')
  const res = await fetch(`/api/plugins/${encodeURIComponent(pluginId)}/files/${encodeURIComponent(filePath)}`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }))
    throw new Error(err.message ?? '读取文件失败')
  }
  return res.blob()
}

export async function pluginDeleteFile(
  pluginId: string, filePath: string,
): Promise<void> {
  await pluginApi(pluginId, `/files/${encodeURIComponent(filePath)}`, {
    method: 'DELETE',
  })
}

export async function pluginListFiles(
  pluginId: string, prefix?: string,
): Promise<string[]> {
  const params = prefix ? `?prefix=${encodeURIComponent(prefix)}` : ''
  const res = await pluginApi<{ files: string[]; count: number }>(pluginId, `/files/list${params}`)
  return res.files
}

// ── Users ──

export interface PluginUserInfo {
  id: string
  username: string
  display_name: string
  avatar_url: string | null
  role: string
  effective_role: string
  team_status: string
  bio: string
  created_at: string
}

export interface PluginTeamMember extends PluginUserInfo {
  user_id: string
  joined_at: string
}

export async function pluginUserMe(pluginId: string): Promise<PluginUserInfo> {
  return pluginApi<PluginUserInfo>(pluginId, '/users/me')
}

export async function pluginUserGet(pluginId: string, userId: string): Promise<PluginUserInfo> {
  return pluginApi<PluginUserInfo>(pluginId, `/users/${encodeURIComponent(userId)}`)
}

export async function pluginUserList(pluginId: string): Promise<{ members: PluginTeamMember[]; count: number }> {
  return pluginApi<{ members: PluginTeamMember[]; count: number }>(pluginId, '/users')
}
