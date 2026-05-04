import { useState, useEffect, useContext, createContext, useCallback, type ReactNode } from 'react'
import { getToken, setToken, clearToken, apiFetch } from './api'
import type { User, Permission } from './types'
import { rolePermissions } from './types'

// ============== Context Type ==============

interface AuthContextType {
  user: User | null
  isAuthenticated: boolean
  loading: boolean
  login: () => void
  adminLogin: (password: string) => Promise<{ success: boolean; message: string }>
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
      // Map user state to role for permission lookup
      let effectiveRole = user.role
      if (user.team_status === 'pending') {
        effectiveRole = 'pending' as any
      } else if (user.team_status === 'none' && user.role === 'guest') {
        effectiveRole = 'guest'
      } else if (user.role === 'member' && user.team_status === 'joined') {
        effectiveRole = 'member'
      }
      // 'superadmin' / 'admin' role gets all admin permissions regardless of team_status
      return rolePermissions[effectiveRole as User['role']]?.includes(permission) ?? false
    },
    [user],
  )

  const login = () => {
    window.location.href = '/api/oauth/authorize'
  }

  const adminLogin = async (password: string): Promise<{ success: boolean; message: string }> => {
    try {
      const res = await apiFetch<{ success: boolean; message: string; token?: string }>(
        '/auth/admin-login',
        { method: 'POST', body: JSON.stringify({ password }) },
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
      adminLogin,
      logout,
      hasPermission,
      refreshUser,
    }}>
      {children}
    </AuthContext.Provider>
  )
}
