import { HashRouter, Routes, Route, Navigate, Outlet } from "react-router-dom";
import { AuthProvider } from "./AuthContext";
import { SiteProvider, useSite } from "./SiteContext";
import { DarkModeProvider } from "./DarkModeContext";
import { NotificationProvider } from "./NotificationContext";
import Navbar from "./components/Navbar";
import AdminLayout from "./components/AdminLayout";
import ProtectedRoute from "./components/ProtectedRoute";
import LoginPage from "./pages/LoginPage";
import AuthCallbackPage from "./pages/AuthCallbackPage";
import ShowcasePage from "./pages/ShowcasePage";
import ProblemsPage from "./pages/ProblemsPage";
import ProblemDetailPage from "./pages/ProblemDetailPage";
import TeamPage from "./pages/TeamPage";
import ApplyPage from "./pages/ApplyPage";
import ContestManagePage from "./pages/ContestManagePage";
import ContestDetailPage from "./pages/ContestDetailPage";
import ProfilePage from "./pages/ProfilePage";
import AdminConfigPage from "./pages/AdminConfigPage";
import AdminUsersPage from "./pages/AdminUsersPage";
import AdminRolesPage from "./pages/AdminRolesPage";
import AdminBackupsPage from "./pages/AdminBackupsPage";
import CommunityPage from "./pages/CommunityPage";
import PostDetailPage from "./pages/PostDetailPage";
import NotFoundPage from "./pages/NotFoundPage";
import AdminInitPage from "./pages/AdminInitPage";

function Footer() {
  const { siteInfo } = useSite();
  const version = siteInfo?.version || "0.1.0";
  return (
    <footer className="border-t border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 mt-12 py-4 px-6">
      <div className="max-w-6xl mx-auto text-center text-xs text-gray-400 dark:text-gray-500">
        Powered by{" "}
        <a
          href="https://github.com/Deng2010/mcguffin"
          target="_blank"
          rel="noopener noreferrer"
          className="text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 underline"
        >
          McGuffin
        </a>{" "}
        v{version}
      </div>
    </footer>
  );
}

/** Main site layout: Navbar + content + Footer */
function MainLayout() {
  return (
    <div className="min-h-screen bg-gray-100 dark:bg-gray-950 flex flex-col">
      <Navbar />
      <div className="flex-1">
        <Outlet />
      </div>
      <Footer />
    </div>
  );
}

/** Admin guard layout */
function AdminGuardLayout() {
  return (
    <ProtectedRoute requiredPermission="manage_site">
      <AdminLayout />
    </ProtectedRoute>
  );
}

function AppContent() {
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
          <Route path="/admin/init" element={<AdminInitPage />} />
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
  );
}

export default function App() {
  return (
    <DarkModeProvider>
      <AuthProvider>
        <SiteProvider>
          <NotificationProvider>
            <AppContent />
          </NotificationProvider>
        </SiteProvider>
      </AuthProvider>
    </DarkModeProvider>
  );
}
