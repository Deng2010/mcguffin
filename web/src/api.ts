// ============== Token Management ==============

import type { NotificationResponse } from './types'

const TOKEN_KEY='***'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token)
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY)
}

// ============== Notification API ==============

export async function fetchNotifications(): Promise<NotificationResponse> {
  return apiFetch<NotificationResponse>('/notifications')
}

export async function markNotificationRead(id: string): Promise<{ success: boolean; message: string }> {
  return apiFetch(`/notifications/read/${id}`, { method: 'POST' })
}

export async function markAllNotificationsRead(): Promise<{ success: boolean; message: string }> {
  return apiFetch('/notifications/read-all', { method: 'POST' })
}

// ============== API Client ==============

export async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const token = getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options?.headers as Record<string, string> || {}),
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }
  const res = await fetch(`/api${path}`, { ...options, headers })
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    console.error(`API error ${res.status}:`, text);
    throw new Error(`请求失败 (${res.status})`);
  }
  return res.json()
}
