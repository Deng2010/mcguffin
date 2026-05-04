import { Navigate, useLocation } from 'react-router-dom'
import { useAuth } from '../AuthContext'
import type { Permission } from '../types'
import type { ReactNode } from 'react'

export default function ProtectedRoute({
  children,
  requiredPermission,
}: {
  children: ReactNode
  requiredPermission?: Permission
}) {
  const { hasPermission, isAuthenticated, loading } = useAuth()
  const location = useLocation()

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-100">
        <p className="text-gray-500">加载中...</p>
      </div>
    )
  }

  // Public permissions don't need auth
  if (requiredPermission === 'view_showcase') {
    return <>{children}</>
  }

  // All other permissions require login
  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  if (!hasPermission(requiredPermission)) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-100">
        <div className="text-center">
          <h1 className="text-2xl font-bold text-gray-800 mb-4">权限不足</h1>
          <p className="text-gray-600 mb-6">您没有权限访问此页面</p>
        </div>
      </div>
    )
  }

  return <>{children}</>
}
