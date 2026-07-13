import { create } from 'zustand'
import { apiFetch, getToken, setToken, clearToken } from '../services/api'
import { fetchPermissions, logout as authServiceLogout } from '../services/auth.service'
import type { User, Permission } from '../types'
import { defaultRolePermissions } from '../types'

// ============== State & Actions ==============

interface AuthState {
  user: User | null
  isAuthenticated: boolean
  loading: boolean
  permMap: Record<string, string[]> | null
  permMapLoading: boolean
  login: () => void
  accountLogin: (identifier: string, password: string) => Promise<{ success: boolean; message: string }>
  logout: () => void
  hasPermission: (permission: Permission) => boolean
  refreshUser: () => Promise<void>
}

export const useAuthStore = create<AuthState>()((set, get) => ({
  user: null,
  isAuthenticated: false,
  loading: true,
  permMap: null,
  permMapLoading: true,

  login: () => {
    window.location.href = '/api/oauth/authorize'
  },

  accountLogin: async (identifier, password) => {
    try {
      const body: Record<string, any> = { password }
      if (identifier.trim()) {
        body.identifier = identifier.trim()
      }
      const res = await apiFetch<{ success: boolean; message: string; token?: string }>(
        '/auth/login',
        { method: 'POST', body: JSON.stringify(body) },
      )
      if (res.success && res.token) {
        setToken(res.token)
        await get().refreshUser()
      }
      return { success: res.success, message: res.message }
    } catch (err) {
      return { success: false, message: `请求失败: ${err}` }
    }
  },

  logout: async () => {
    try { await authServiceLogout() } catch { /* ignore — token might be invalid */ }
    clearToken()
    set({ user: null, isAuthenticated: false })
  },

  refreshUser: async () => {
    const token = getToken()
    if (!token) {
      set({ user: null, isAuthenticated: false, loading: false })
      return
    }
    try {
      const me = await apiFetch<User>('/user/me')
      set({ user: me, isAuthenticated: true, loading: false })
    } catch {
      clearToken()
      set({ user: null, isAuthenticated: false, loading: false })
    }
  },

  hasPermission: (permission) => {
    const { user, permMap, permMapLoading } = get()
    if (!user) {
      return permission === 'view_showcase'
    }
    if (permMapLoading) {
      return permission === 'view_showcase'
    }
    const effectiveRole = user.effective_role
      ?? (user.team_status === 'pending' ? 'guest' as any
        : user.team_status === 'none' && user.role === 'guest' ? 'guest'
        : user.role === 'member' && user.team_status === 'joined' ? 'member'
        : user.role)
    const map = permMap ?? defaultRolePermissions as unknown as Record<string, Permission[]>
    const userPerms = map[effectiveRole]
    if (!userPerms) return false
    if (userPerms.includes('*' as any)) return true
    return userPerms.includes(permission)
  },
}))

// ============== Initialization ==============
// Call once from App.tsx to kick off async bootstrapping.

export async function initAuth() {
  // Fetch role→permissions mapping (no auth needed)
  try {
    const map = await fetchPermissions()
    useAuthStore.setState({ permMap: map, permMapLoading: false })
  } catch {
    // Fallback to defaults
    useAuthStore.setState({ permMapLoading: false })
  }
  // Refresh user from stored token (if any)
  await useAuthStore.getState().refreshUser()
}
