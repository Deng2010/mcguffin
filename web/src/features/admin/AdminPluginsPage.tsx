import { useState, useEffect, useRef } from "react";
import { apiFetch } from "../../services/api";
import { useToast } from "../../errors/ToastContext";
import { PluginRegistry } from "../../plugins/registry";

// ── Types ──

interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  permissions_needed: string[];
  enabled: boolean;
}

interface PluginsListResponse {
  plugins: PluginManifest[];
  plugins_disabled?: boolean;
}

interface DisplayPlugin extends PluginManifest {
  /** Registered by definePlugin() in frontend code */
  isLocal: boolean;
  /** Installed via .zip upload (has source_dir on backend) */
  isUploaded: boolean;
}

// ── Component ──

export default function AdminPluginsPage() {
  const [backendPlugins, setBackendPlugins] = useState<PluginManifest[]>([]);
  const [loading, setLoading] = useState(true);
  const [globallyDisabled, setGloballyDisabled] = useState(false);
  const [installing, setInstalling] = useState(false);
  const toast = useToast();
  const uploadRef = useRef<HTMLInputElement>(null);

  // ── Load backend plugins ──

  const loadBackendPlugins = async () => {
    setLoading(true);
    try {
      const res = await apiFetch<PluginsListResponse>("/admin/plugins");
      setBackendPlugins(res.plugins);
      setGloballyDisabled(res.plugins_disabled === true);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadBackendPlugins();
  }, []);

  // ── Merge local + backend into a single list ──

  const registry = PluginRegistry.getInstance();
  // Use ALL locally-registered plugin IDs so disabled code-registered plugins
  // are still correctly labeled (instead of being misclassified as ZIP).
  const localIds = registry.getRegisteredPluginIds();

  const displayPlugins: DisplayPlugin[] = [
    // Plugins from backend (includes both local-registered and zip-installed)
    ...backendPlugins.map((p) => ({
      ...p,
      isLocal: localIds.has(p.id),
      isUploaded: !localIds.has(p.id),
    })),
    // Plugins only registered locally (not yet synced to backend)
    ...Array.from(localIds)
      .filter((id) => !backendPlugins.some((p) => p.id === id))
      .map((id) => ({
        id,
        name: id,
        version: "0.1.0",
        description: "",
        author: undefined,
        homepage: undefined,
        permissions_needed: [] as string[],
        enabled: true,
        isLocal: true,
        isUploaded: false,
      })),
  ];

  // Sort: local-first, then alphabetical
  displayPlugins.sort((a, b) => {
    if (a.isLocal !== b.isLocal) return a.isLocal ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  // ── Zip upload ──

  const showMsg = (text: string, type: "success" | "error" | "info") => {
    if (type === "success") toast.success(text);
    else if (type === "error") toast.error(text);
    else toast.info(text);
  };

  const handleInstallZip = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.name.endsWith(".zip")) {
      showMsg("请选择 .zip 文件", "error");
      e.target.value = "";
      return;
    }

    setInstalling(true);
    showMsg(`正在安装 ${file.name}...`, "info");

    try {
      const buffer = await file.arrayBuffer();
      const res = await fetch("/api/admin/plugins/install-zip", {
        method: "POST",
        headers: {
          Authorization: `Bearer ${localStorage.getItem("auth_token")}`,
          "Content-Type": "application/octet-stream",
        },
        body: buffer,
      });
      const data = await res.json();
      if (!res.ok) {
        showMsg(`安装失败: ${data.message}`, "error");
        return;
      }
      showMsg(
        `✅ 插件「${data.plugin.name} v${data.plugin.version}」安装成功`,
        "success",
      );
      loadBackendPlugins();
    } catch (err) {
      showMsg(`安装失败: ${err}`, "error");
    } finally {
      setInstalling(false);
    }
    e.target.value = "";
  };

  // ── Uninstall ──

  const handleUninstall = async (pluginId: string, pluginName: string) => {
    if (!confirm(`确定要卸载插件「${pluginName}」吗？\n相关数据也会被删除。`))
      return;
    showMsg(`正在卸载 ${pluginName}...`, "info");
    try {
      const res = await apiFetch<{ success: boolean; message?: string }>(
        `/admin/plugins/${encodeURIComponent(pluginId)}`,
        { method: "DELETE" },
      );
      if (!res.success) {
        showMsg(`卸载失败: ${res.message}`, "error");
        return;
      }
      showMsg(`✅ 已卸载「${pluginName}」`, "success");
      loadBackendPlugins();
    } catch (err) {
      showMsg(`卸载失败: ${err}`, "error");
    }
  };

  // ── Global enable / disable ──

  const handleGlobalToggle = async () => {
    const next = !globallyDisabled;
    const actionLabel = next ? "禁用" : "启用";
    if (!confirm(`确定要全局${actionLabel}所有插件功能吗？`)) return;
    showMsg(`正在全局${actionLabel}插件...`, "info");
    try {
      const res = await apiFetch<{
        success: boolean;
        plugins_disabled?: boolean;
        message?: string;
      }>("/admin/plugins/global", {
        method: "POST",
        body: JSON.stringify({ enabled: !next }),
      });
      if (!res.success) {
        showMsg(`全局${actionLabel}失败: ${res.message}`, "error");
        return;
      }
      setGloballyDisabled(res.plugins_disabled === true);
      PluginRegistry.getInstance().setGloballyDisabled(
        res.plugins_disabled === true,
      );
      showMsg(`✅ 已全局${actionLabel}插件功能`, "success");
    } catch (err) {
      showMsg(`全局${actionLabel}失败: ${err}`, "error");
    }
  };

  // ── Enable / Disable ──

  const handleToggle = async (
    pluginId: string,
    pluginName: string,
    currentEnabled: boolean,
  ) => {
    const action = currentEnabled ? "disable" : "enable";
    const actionLabel = currentEnabled ? "禁用" : "启用";
    if (!confirm(`确定要${actionLabel}插件「${pluginName}」吗？`)) return;
    showMsg(`正在${actionLabel} ${pluginName}...`, "info");
    try {
      const res = await apiFetch<{ success: boolean; message?: string }>(
        `/admin/plugins/${encodeURIComponent(pluginId)}/${action}`,
        { method: "POST" },
      );
      if (!res.success) {
        showMsg(`${actionLabel}失败: ${res.message}`, "error");
        return;
      }
      showMsg(`✅ 已${actionLabel}「${pluginName}」`, "success");
      loadBackendPlugins();
    } catch (err) {
      showMsg(`${actionLabel}失败: ${err}`, "error");
    }
  };

  // ── Render ──

  return (
    <div>
      {/* ── Global Toggle Section ── */}
      <div className="mg-box-shadow p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-4 pb-2 border-b border-gray-200 dark:border-gray-700">
          全局插件开关
        </h2>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-gray-700 dark:text-gray-200 font-medium">
              {globallyDisabled
                ? "插件功能当前已全局禁用"
                : "插件功能当前已全局启用"}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              全局禁用后，所有插件（含路由、页面与数据接口）将对所有用户隐藏并不可用。
            </p>
          </div>
          <button
            onClick={handleGlobalToggle}
            className={`px-5 py-2 text-sm font-medium border ${
              globallyDisabled
                ? "border-green-300 text-green-600 hover:bg-green-50 dark:border-green-800 dark:text-green-400 dark:hover:bg-green-900/20"
                : "border-red-300 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20"
            }`}
          >
            {globallyDisabled ? "启用插件功能" : "禁用插件功能"}
          </button>
        </div>
      </div>

      {/* ── Upload Section ── */}
      <div className="mg-box-shadow p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-4 pb-2 border-b border-gray-200 dark:border-gray-700">
          安装插件
        </h2>
        <div>
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
            上传 .zip 格式的插件包（包含 plugin.json 和资源文件）
          </p>
          <div className="flex items-center gap-3">
            <button
              onClick={() => uploadRef.current?.click()}
              disabled={installing}
              className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50"
            >
              <span className="flex items-center gap-2">
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 4v12m0 0l-3-3m3 3l3-3m-6 8h6"
                  />
                </svg>
                {installing ? "安装中..." : "上传 .zip 文件"}
              </span>
            </button>
            <input
              ref={uploadRef}
              type="file"
              accept=".zip"
              onChange={handleInstallZip}
              className="hidden"
            />
          </div>
        </div>
      </div>

      {/* ── Plugin List ── */}
      <div className="mg-box-shadow p-5">
        <div className="flex items-center justify-between mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100">
            插件列表
          </h2>
        </div>

        {loading && displayPlugins.length === 0 ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500">
            加载中...
          </div>
        ) : displayPlugins.length === 0 ? (
          <div className="text-center py-8 border border-dashed border-gray-300 dark:border-gray-700">
            <p className="text-gray-400 dark:text-gray-500">暂无已安装的插件</p>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              在 web/src/plugins/ 下创建插件目录使用 definePlugin() 注册，
              或通过上方区域上传 .zip 文件安装
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {displayPlugins.map((p) => (
              <div
                key={p.id}
                className={`flex items-center justify-between p-4 border ${
                  p.enabled === false
                    ? "border-gray-200 dark:border-gray-700 bg-gray-100 dark:bg-gray-800/30 opacity-70"
                    : "border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50"
                }`}
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-medium text-gray-800 dark:text-gray-100 truncate text-sm">
                      {p.name}
                    </span>
                    <span className="text-xs text-gray-400 dark:text-gray-500 bg-gray-200 dark:bg-gray-700 px-1.5 py-0.5">
                      v{p.version}
                    </span>
                    {p.author && (
                      <span className="text-xs text-gray-400 dark:text-gray-500 hidden sm:inline">
                        @{p.author}
                      </span>
                    )}

                    {/* Enabled/Disabled badge */}
                    {p.enabled === false ? (
                      <span className="text-xs px-1.5 py-0.5 bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400">
                        已禁用
                      </span>
                    ) : (
                      <span className="text-xs px-1.5 py-0.5 bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400">
                        已启用
                      </span>
                    )}

                    {/* Source badge */}
                    {p.isLocal && p.isUploaded && (
                      <span className="text-xs px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
                        双向注册
                      </span>
                    )}
                    {p.isLocal && !p.isUploaded && (
                      <span className="text-xs px-1.5 py-0.5 bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400">
                        代码注册
                      </span>
                    )}
                    {!p.isLocal && p.isUploaded && (
                      <span className="text-xs px-1.5 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400">
                        ZIP 安装
                      </span>
                    )}
                  </div>

                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                    {p.description || "暂无描述"}
                    <span className="ml-2 text-gray-400 dark:text-gray-500">
                      id: {p.id}
                    </span>
                    {p.permissions_needed.length > 0 && (
                      <span className="ml-2 text-gray-400 dark:text-gray-500">
                        权限: {p.permissions_needed.join(", ")}
                      </span>
                    )}
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2 ml-4 shrink-0">
                  {/* Enable/Disable toggle */}
                  <button
                    onClick={() =>
                      handleToggle(p.id, p.name, p.enabled !== false)
                    }
                    className={`px-3 py-1.5 text-xs border ${
                      p.enabled === false
                        ? "border-green-300 dark:border-green-800 text-green-600 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20"
                        : "border-yellow-300 dark:border-yellow-800 text-yellow-600 dark:text-yellow-400 hover:bg-yellow-50 dark:hover:bg-yellow-900/20"
                    }`}
                  >
                    {p.enabled === false ? "启用" : "禁用"}
                  </button>
                  {p.isUploaded && !p.isLocal && (
                    <button
                      onClick={() => handleUninstall(p.id, p.name)}
                      className="px-3 py-1.5 text-xs border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                    >
                      卸载
                    </button>
                  )}
                </div>
              </div>
            ))}
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">
              共 {displayPlugins.length} 个插件
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
