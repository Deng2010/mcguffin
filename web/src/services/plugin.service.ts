// ============== 管理后台插件 API ==============

import { apiFetch, getToken } from "./api";

export interface BackendPlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  permissions_needed: string[];
  enabled: boolean;
  /** 安装来源：code = 前端代码注册；zip = 后台上传安装 */
  source?: "code" | "zip";
  /** zip 插件入口文件 */
  entry?: string;
}

export interface PluginsListResponse {
  plugins: BackendPlugin[];
  plugins_disabled?: boolean;
  /** 后端已知的插件权限清单（用于权限编辑 UI） */
  known_permissions?: string[];
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

/** 调整插件已授予的权限（superadmin）。注册后权限冻结，只能经此接口变更。 */
export async function setPluginPermissions(
  pluginId: string,
  permissions: string[],
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(
    `/admin/plugins/${encodeURIComponent(pluginId)}/permissions`,
    { method: "PUT", body: JSON.stringify({ permissions }) },
  );
}

export async function installPluginZip(
  buffer: ArrayBuffer,
): Promise<Record<string, any>> {
  return fetch("/api/admin/plugins/install-zip", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${getToken()}`,
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
