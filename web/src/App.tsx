import { useEffect } from "react";
import { NotificationProvider } from "./NotificationContext";
import { initAuth } from "./stores/authStore";
import { initSite } from "./stores/siteStore";
import { PluginRegistry } from "./plugins/registry";
import AppRoutes from "./app/routes";

export default function App() {
  useEffect(() => {
    initAuth();
    initSite();
    PluginRegistry.getInstance().discover();
  }, []);

  return (
    <NotificationProvider>
      <AppRoutes />
    </NotificationProvider>
  );
}
