import { apiFetch } from './api'

export interface LoginPayload {
  identifier: string
  password: string
}

export interface InitStatus {
  initialized: boolean
}

export interface InitAdminPayload {
  username: string
  password: string
}

export async function fetchPermissions(): Promise<Record<string, string[]>> {
  return apiFetch<Record<string, string[]>>('/auth/permissions')
}

export async function login(identifier: string, password: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ identifier, password }),
  })
}

export async function logout(): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/logout', { method: 'POST' })
}

export async function getInitStatus(): Promise<InitStatus> {
  return apiFetch<InitStatus>('/admin/init-status')
}

export async function initAdmin(body: InitAdminPayload): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>('/admin/init', {
    method: 'POST',
    body: JSON.stringify(body),
  })
}
