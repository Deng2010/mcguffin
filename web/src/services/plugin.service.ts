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
  /**
   * 安装来源：`zip` = 后台上传 .zip 安装（当前唯一安装方式）；
   * `code` = 早期的前端代码注册，机制已移除，仅历史数据可能残留。
   */
  source?: "code" | "zip";
  /** zip 插件入口文件 */
  entry?: string;
  /** 安装来源 URL（从 URL 安装时记录；本地上传安装为空） */
  source_url?: string;
}

/** 安装 / 更新插件的统一返回（install-zip / install-url / update / update-zip 共用）。 */
export interface PluginInstallResult {
  success: boolean;
  /** true = 覆盖已有插件（更新），false = 全新安装 */
  updated?: boolean;
  message?: string;
  /** 更新前的版本号（全新安装为 null） */
  previous_version?: string | null;
  /** 新版本声明、但尚未授予的权限（需管理员在「权限」中勾选后才生效） */
  new_permissions?: string[];
  plugin?: BackendPlugin & { file_count?: number };
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
): Promise<PluginInstallResult> {
  return uploadPluginZip("/api/admin/plugins/install-zip", buffer);
}

/** 上传 .zip 更新指定插件（包内 plugin.json 的 id 必须与 pluginId 一致）。 */
export async function updatePluginZip(
  pluginId: string,
  buffer: ArrayBuffer,
): Promise<PluginInstallResult> {
  return uploadPluginZip(
    `/api/admin/plugins/${encodeURIComponent(pluginId)}/update-zip`,
    buffer,
  );
}

/** 从 URL 安装 / 更新插件（服务端下载并记录来源 URL，供后续「从原 URL 更新」）。 */
export async function installPluginFromUrl(
  url: string,
): Promise<PluginInstallResult> {
  return apiFetch<PluginInstallResult>("/admin/plugins/install-url", {
    method: "POST",
    body: JSON.stringify({ url }),
  });
}

/** 从安装时记录的来源 URL 重新下载并更新指定插件。 */
export async function updatePluginFromUrl(
  pluginId: string,
): Promise<PluginInstallResult> {
  return apiFetch<PluginInstallResult>(
    `/admin/plugins/${encodeURIComponent(pluginId)}/update`,
    { method: "POST" },
  );
}

/** 上传 raw zip 字节（install-zip / update-zip 共用同一响应结构）。 */
async function uploadPluginZip(
  path: string,
  buffer: ArrayBuffer,
): Promise<PluginInstallResult> {
  const res = await fetch(path, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${getToken()}`,
      "Content-Type": "application/octet-stream",
    },
    body: buffer,
  });
  const data = await res.json();
  if (!res.ok) {
    throw Object.assign(new Error(data.message || "安装失败"), { data });
  }
  return data as PluginInstallResult;
}
