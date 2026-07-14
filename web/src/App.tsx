import { useEffect } from "react";
import { NotificationProvider } from "./NotificationContext";
import { initAuth } from "./stores/authStore";
import { initSite } from "./stores/siteStore";
import AppRoutes from "./app/routes";

export default function App() {
  useEffect(() => {
    initAuth();
    initSite();
  }, []);

  return (
    <NotificationProvider>
      <AppRoutes />
    </NotificationProvider>
  );
}
