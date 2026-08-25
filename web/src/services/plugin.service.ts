// ============== 管理后台插件 API ==============

import { apiFetch } from "./api";

export interface BackendPlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  permissions_needed: string[];
  enabled: boolean;
}

export interface PluginsListResponse {
  plugins: BackendPlugin[];
  plugins_disabled?: boolean;
}

export async function getAdminPlugins(): Promise<PluginsListResponse> {
  return apiFetch<PluginsListResponse>("/admin/plugins");
}

export async function uninstallPlugin(
  pluginId: string,
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(
    `/admin/plugins/${encodeURIComponent(pluginId)}`,
    { method: "DELETE" },
  );
}

export async function setPluginsGloballyEnabled(
  enabled: boolean,
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>("/admin/plugins/global", {
    method: "POST",
    body: JSON.stringify({ enabled }),
  });
}

export async function setPluginEnabled(
  pluginId: string,
  enabled: boolean,
): Promise<Record<string, any>> {
  const action = enabled ? "enable" : "disable";
  return apiFetch<Record<string, any>>(
    `/admin/plugins/${encodeURIComponent(pluginId)}/${action}`,
    { method: "POST" },
  );
}

export async function installPluginZip(
  buffer: ArrayBuffer,
): Promise<Record<string, any>> {
  return fetch("/api/admin/plugins/install-zip", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${localStorage.getItem("auth_token")}`,
      "Content-Type": "application/octet-stream",
    },
    body: buffer,
  }).then(async (res) => {
    const data = await res.json();
    if (!res.ok) {
      throw Object.assign(new Error(data.message || "安装失败"), { data });
    }
    return data;
  });
}
