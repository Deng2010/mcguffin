import { useState, useEffect, useContext, createContext, useCallback, type ReactNode } from 'react'
import { getToken, setToken, clearToken, apiFetch } from './api'
import type { User, Permission } from './types'
import { defaultRolePermissions } from './types'

// ============== Context Type ==============

interface AuthContextType {
  user: User | null
  isAuthenticated: boolean
  loading: boolean
  login: () => void
  accountLogin: (identifier: string, password: string) => Promise<{ success: boolean; message: string }>
  logout: () => void
  hasPermission: (permission: Permission) => boolean
  refreshUser: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | null>(null)

// ============== Hook ==============

export function useAuth(): AuthContextType {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')
  return ctx
}

// ============== Provider ==============

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)
  const [permMap, setPermMap] = useState<Record<string, string[]> | null>(null)
  const [permMapLoading, setPermMapLoading] = useState(true)

  // Fetch role→permissions mapping from backend (no auth needed)
  useEffect(() => {
    apiFetch<Record<string, string[]>>('/auth/permissions')
      .then((map) => { setPermMap(map); setPermMapLoading(false) })
      .catch(() => { setPermMapLoading(false) /* fallback to defaults */ })
  }, [])

  const refreshUser = useCallback(async () => {
    const token = getToken()
    if (!token) {
      setUser(null)
      setLoading(false)
      return
    }
    try {
      const me = await apiFetch<User>('/user/me')
      setUser(me)
    } catch {
      clearToken()
      setUser(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { refreshUser() }, [refreshUser])

  const hasPermission = useCallback(
    (permission: Permission): boolean => {
      if (!user) {
        // Not logged in: only view_showcase is allowed
        return permission === 'view_showcase'
      }
      // While permMap is loading, be conservative: only public permissions
      if (permMapLoading) {
        return permission === 'view_showcase'
      }
      // Use backend-computed effective_role, or compute locally as fallback
      const effectiveRole = user.effective_role
        ?? (user.team_status === 'pending' ? 'pending' as any
          : user.team_status === 'none' && user.role === 'guest' ? 'guest'
          : user.role === 'member' && user.team_status === 'joined' ? 'member'
          : user.role)
      // Use backend-provided mapping, fall back to defaults
      const map = permMap ?? defaultRolePermissions as unknown as Record<string, Permission[]>
      const userPerms = map[effectiveRole]
      if (!userPerms) return false
      // Wildcard '*' means all permissions (superadmin)
      if (userPerms.includes('*' as any)) return true
      return userPerms.includes(permission)
    },
    [user, permMap, permMapLoading],
  )

  const login = () => {
    window.location.href = '/api/oauth/authorize'
  }

  const accountLogin = async (identifier: string, password: string): Promise<{ success: boolean; message: string }> => {
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
        await refreshUser()
      }
      return { success: res.success, message: res.message }
    } catch (err) {
      return { success: false, message: `请求失败: ${err}` }
    }
  }

  const logout = async () => {
    try { await apiFetch('/logout', { method: 'POST' }) } catch {}
    clearToken()
    setUser(null)
  }

  return (
    <AuthContext.Provider value={{
      user,
      isAuthenticated: !!user,
      loading,
      login,
      accountLogin,
      logout,
      hasPermission,
      refreshUser,
    }}>
      {children}
    </AuthContext.Provider>
  )
}
