import { HashRouter, Routes, Route } from 'react-router-dom'
import { AuthProvider } from './AuthContext'
import { SiteProvider, useSite } from './SiteContext'
import Navbar from './components/Navbar'
import ProtectedRoute from './components/ProtectedRoute'
import LoginPage from './pages/LoginPage'
import AuthCallbackPage from './pages/AuthCallbackPage'
import ShowcasePage from './pages/ShowcasePage'
import ProblemsPage from './pages/ProblemsPage'
import ProblemDetailPage from './pages/ProblemDetailPage'
import TeamPage from './pages/TeamPage'
import ApplyPage from './pages/ApplyPage'
import ContestManagePage from './pages/ContestManagePage'
import ProfilePage from './pages/ProfilePage'
import AdminConfigPage from './pages/AdminConfigPage'
import AdminBackupPage from './pages/AdminBackupPage'
import NotFoundPage from './pages/NotFoundPage'

function Footer() {
  const { siteInfo } = useSite()
  const version = siteInfo?.version || '0.1.0'
  return (
    <footer className="border-t border-gray-200 bg-white mt-12 py-4 px-6">
      <div className="max-w-6xl mx-auto text-center text-xs text-gray-400">
        Powered by McGuffin v{version}
      </div>
    </footer>
  )
}

function AppContent() {
  return (
    <HashRouter>
      <div className="min-h-screen bg-gray-100 flex flex-col">
        <Navbar />
        <div className="flex-1">
          <Routes>
            <Route path="/" element={<ShowcasePage />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/auth/callback" element={<AuthCallbackPage />} />
            <Route
              path="/problems"
              element={<ProtectedRoute requiredPermission="view_problems"><ProblemsPage /></ProtectedRoute>}
            />
            <Route
              path="/problems/:id"
              element={<ProtectedRoute requiredPermission="view_problems"><ProblemDetailPage /></ProtectedRoute>}
            />
            <Route
              path="/team"
              element={<ProtectedRoute requiredPermission="view_team"><TeamPage /></ProtectedRoute>}
            />
            <Route
              path="/apply"
              element={<ProtectedRoute requiredPermission="apply_join"><ApplyPage /></ProtectedRoute>}
            />
            <Route
              path="/contests"
              element={<ProtectedRoute requiredPermission="approve_problem"><ContestManagePage /></ProtectedRoute>}
            />
            <Route
              path="/profile"
              element={<ProtectedRoute requiredPermission="view_showcase"><ProfilePage /></ProtectedRoute>}
            />
            <Route
              path="/admin/config"
              element={<ProtectedRoute requiredPermission="manage_site"><AdminConfigPage /></ProtectedRoute>}
            />
            <Route
              path="/admin/backup"
              element={<ProtectedRoute requiredPermission="manage_site"><AdminBackupPage /></ProtectedRoute>}
            />
            <Route path="*" element={<NotFoundPage />} />
          </Routes>
        </div>
        <Footer />
      </div>
    </HashRouter>
  )
}

export default function App() {
  return (
    <AuthProvider>
      <SiteProvider>
        <AppContent />
      </SiteProvider>
    </AuthProvider>
  )
}
