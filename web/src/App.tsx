import { useEffect, lazy } from "react";
import { NotificationProvider } from "./NotificationContext";
import { initAuth } from "./stores/authStore";
import { initSite } from "./stores/siteStore";
import AppRoutes from "./app/routes";
import { PluginRegistry } from "./plugins/registry";

const SystemInfoPage = lazy(() => import("./plugins/builtins/SystemInfoPage"));

export default function App() {
  useEffect(() => {
    initAuth();
    initSite();
    const init = async () => {
      const registry = PluginRegistry.getInstance();
      await registry.discover();
      registry.registerComponent("system-info", SystemInfoPage);
    };
    init();
  }, []);

  return (
    <NotificationProvider>
      <AppRoutes />
    </NotificationProvider>
  );
}
