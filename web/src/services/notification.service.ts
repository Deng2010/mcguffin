import type { NotificationResponse } from '../types'
import { apiFetch } from './api'

export async function fetchNotifications(): Promise<NotificationResponse> {
  return apiFetch<NotificationResponse>('/notifications')
}

export async function markNotificationRead(id: string): Promise<{ success: boolean; message: string }> {
  return apiFetch(`/notifications/read/${id}`, { method: 'POST' })
}

export async function markAllNotificationsRead(): Promise<{ success: boolean; message: string }> {
  return apiFetch('/notifications/read-all', { method: 'POST' })
}
