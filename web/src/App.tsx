import { useEffect } from "react";
import { NotificationProvider } from "./NotificationContext";
import { initAuth } from "./stores/authStore";
import { initSite } from "./stores/siteStore";
import { PluginRegistry } from "./plugins/registry";
import AppRoutes from "./app/routes";
import ErrorBoundary from "./errors/ErrorBoundary";
import { ToastProvider } from "./errors/ToastContext";
import { initErrorCapture } from "./errors/reporter";

export default function App() {
  useEffect(() => {
    initErrorCapture();
    initAuth();
    initSite();
    PluginRegistry.getInstance().discover();
    PluginRegistry.getInstance().fetchPluginStatus();
  }, []);

  return (
    <ToastProvider>
      <ErrorBoundary>
        <NotificationProvider>
          <AppRoutes />
        </NotificationProvider>
      </ErrorBoundary>
    </ToastProvider>
  );
}
