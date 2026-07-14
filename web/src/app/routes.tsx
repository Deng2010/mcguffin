import { HashRouter, Routes, Route, Navigate } from 'react-router-dom'
import { useSyncExternalStore } from 'react'
import ProtectedRoute from '../components/ProtectedRoute'
import MainLayout from './layouts/MainLayout'
import AdminLayout from './layouts/AdminLayout'
import { PluginRegistry } from '../plugins/registry'
import PluginPage from '../plugins/PluginPage'
import type { Permission } from '../types'
import LoginPage from '../features/auth/LoginPage'
import AuthCallbackPage from '../features/auth/AuthCallbackPage'
import ShowcasePage from '../features/showcase/ShowcasePage'
import ProblemsPage from '../features/problems/ProblemsPage'
import ProblemDetailPage from '../features/problems/ProblemDetailPage'
import TeamPage from '../features/team/TeamPage'
import ApplyPage from '../features/team/ApplyPage'
import ContestManagePage from '../features/contests/ContestManagePage'
import ContestDetailPage from '../features/contests/ContestDetailPage'
import ProfilePage from '../features/profile/ProfilePage'
import AdminConfigPage from '../features/admin/AdminConfigPage'
import AdminUsersPage from '../features/admin/AdminUsersPage'
import AdminRolesPage from '../features/admin/AdminRolesPage'
import AdminBackupsPage from '../features/admin/AdminBackupsPage'
import CommunityPage from '../features/community/CommunityPage'
import PostDetailPage from '../features/community/PostDetailPage'
import NotFoundPage from '../features/notfound/NotFoundPage'
import AdminInitPage from '../features/admin/AdminInitPage'

/** Admin guard layout */
function AdminGuardLayout() {
  return (
    <ProtectedRoute requiredPermission="manage_site">
      <AdminLayout />
    </ProtectedRoute>
  )
}

export default function AppRoutes() {
  const pluginRoutes = useSyncExternalStore(
    (cb) => PluginRegistry.getInstance().subscribe(cb),
    () => PluginRegistry.getInstance().getPluginRoutes(),
  )

  return (
    <HashRouter>
      <Routes>
        {/* Init page — outside MainLayout, no navbar/footer */}
        <Route path="/admin/init" element={<AdminInitPage />} />

        {/* Main site routes */}
        <Route element={<MainLayout />}>
          <Route path="/" element={<ShowcasePage />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/auth/callback" element={<AuthCallbackPage />} />
          <Route path="/problems" element={<ProblemsPage />} />
          <Route
            path="/problems/:id"
            element={
              <ProtectedRoute requiredPermission="view_problems">
                <ProblemDetailPage />
              </ProtectedRoute>
            }
          />
          <Route
            path="/team"
            element={
              <ProtectedRoute requiredPermission="view_team">
                <TeamPage />
              </ProtectedRoute>
            }
          />
          <Route
            path="/apply"
            element={
              <ProtectedRoute requiredPermission="apply_join">
                <ApplyPage />
              </ProtectedRoute>
            }
          />
          <Route path="/contests" element={<ContestManagePage />} />
          <Route path="/contests/:id" element={<ContestDetailPage />} />
          <Route path="/community" element={<CommunityPage />} />
          <Route path="/post/:id" element={<PostDetailPage />} />
          <Route
            path="/profile"
            element={
              <ProtectedRoute requiredPermission="view_showcase">
                <ProfilePage />
              </ProtectedRoute>
            }
          />
          <Route path="/profile/:username" element={<ProfilePage />} />
          {pluginRoutes.map(({ pluginId, route }) => {
            const element = route.required_permission ? (
              <ProtectedRoute requiredPermission={route.required_permission as Permission}>
                <PluginPage pluginId={pluginId} />
              </ProtectedRoute>
            ) : (
              <PluginPage pluginId={pluginId} />
            )
            return <Route key={pluginId} path={route.path.replace(/^\//, '')} element={element} />
          })}
          <Route path="*" element={<NotFoundPage />} />
        </Route>

        {/* Admin routes */}
        <Route path="/admin" element={<AdminGuardLayout />}>
          <Route index element={<Navigate to="/admin/config" replace />} />
          <Route path="config" element={<AdminConfigPage />} />
          <Route path="users" element={<AdminUsersPage />} />
          <Route path="roles" element={<AdminRolesPage />} />
          <Route path="backups" element={<AdminBackupsPage />} />
        </Route>
      </Routes>
    </HashRouter>
  )
}
