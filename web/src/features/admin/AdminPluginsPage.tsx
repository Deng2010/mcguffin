import { useState, useEffect, useRef } from "react";
import { apiFetch } from "../../services/api";

interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  permissions_needed: string[];
  source?: string;
}

interface PluginsListResponse {
  plugins: PluginManifest[];
}

export default function AdminPluginsPage() {
  const [plugins, setPlugins] = useState<PluginManifest[]>([]);
  const [loading, setLoading] = useState(true);
  const [reloading, setReloading] = useState(false);
  const [msg, setMsg] = useState("");
  const [installUrl, setInstallUrl] = useState("");
  const [installUrlId, setInstallUrlId] = useState("");
  const [installingUrl, setInstallingUrl] = useState(false);
  const uploadRef = useRef<HTMLInputElement>(null);

  const load = async () => {
    setLoading(true);
    try {
      const res = await apiFetch<PluginsListResponse>("/plugins");
      setPlugins(res.plugins);
    } catch (err) {
      setMsg(`加载插件列表失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleReload = async () => {
    setReloading(true);
    setMsg("");
    try {
      const res = await apiFetch<{
        reloaded: boolean;
        plugins_loaded: number;
        plugins_removed: number;
      }>("/plugins/reload", { method: "POST" });
      if (res.reloaded) {
        setMsg(
          `✅ 已刷新：加载 ${res.plugins_loaded} 个，移除 ${res.plugins_removed} 个`,
        );
        load();
        setTimeout(() => setMsg(""), 5000);
      }
    } catch (err) {
      setMsg(`刷新失败: ${err}`);
    } finally {
      setReloading(false);
    }
  };

  const handleUploadInstall = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.name.endsWith(".wasm")) {
      setMsg("请选择 .wasm 文件");
      e.target.value = "";
      return;
    }
    // Use file stem as plugin id (strip .wasm)
    const pluginId = file.name.replace(/\.wasm$/i, "");
    setMsg(`正在安装 ${pluginId}...`);
    try {
      const buffer = await file.arrayBuffer();
      const res = await fetch(`/api/admin/plugins/install?id=${encodeURIComponent(pluginId)}`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${localStorage.getItem("auth_token")}`,
        },
        body: buffer,
      });
      const data = await res.json();
      if (!res.ok) {
        setMsg(`安装失败: ${data.message}`);
        return;
      }
      setMsg(`✅ 插件「${data.plugin.name}」安装成功`);
      load();
      setTimeout(() => setMsg(""), 5000);
    } catch (err) {
      setMsg(`安装失败: ${err}`);
    }
    e.target.value = "";
  };

  const handleUrlInstall = async () => {
    const id = installUrlId.trim();
    const url = installUrl.trim();
    if (!id || !url) {
      setMsg("请填写插件 ID 和 URL");
      return;
    }
    setInstallingUrl(true);
    setMsg(`正在从 URL 安装 ${id}...`);
    try {
      const res = await apiFetch<{
        success: boolean;
        message?: string;
        plugin?: PluginManifest;
      }>(`/admin/plugins/install-url?id=${encodeURIComponent(id)}&url=${encodeURIComponent(url)}`, {
        method: "POST",
      });
      if (!res.success) {
        setMsg(`安装失败: ${res.message}`);
        return;
      }
      setMsg(`✅ 插件「${res.plugin?.name}」安装成功`);
      setInstallUrl("");
      setInstallUrlId("");
      load();
      setTimeout(() => setMsg(""), 5000);
    } catch (err) {
      setMsg(`安装失败: ${err}`);
    } finally {
      setInstallingUrl(false);
    }
  };

  const handleUninstall = async (pluginId: string, pluginName: string) => {
    if (!confirm(`确定要卸载插件「${pluginName}」吗？`)) return;
    setMsg(`正在卸载 ${pluginName}...`);
    try {
      const res = await apiFetch<{ success: boolean; message?: string }>(
        `/admin/plugins/${encodeURIComponent(pluginId)}`,
        { method: "DELETE" },
      );
      if (!res.success) {
        setMsg(`卸载失败: ${res.message}`);
        return;
      }
      setMsg(`✅ 已卸载「${pluginName}」`);
      load();
      setTimeout(() => setMsg(""), 5000);
    } catch (err) {
      setMsg(`卸载失败: ${err}`);
    }
  };

  return (
    <div>
      {msg && (
        <div
          className={`mb-4 p-3 text-sm border ${
            msg.startsWith("✅")
              ? "bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300"
              : (msg.startsWith("正在") || msg.startsWith("正在从"))
                ? "bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-300"
                : msg.includes("失败")
                  ? "bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300"
                  : "bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-700"
          }`}
        >
          {msg}
        </div>
      )}

      {/* 安装区域 */}
      <div className="mg-box-shadow p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-4 pb-2 border-b border-gray-200 dark:border-gray-700">
          安装插件
        </h2>

        {/* 上传安装 */}
        <div className="mb-4">
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
            上传 .wasm 文件安装（文件名将作为插件 ID）
          </p>
          <div className="flex items-center gap-3">
            <button
              onClick={() => uploadRef.current?.click()}
              className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800"
            >
              <span className="flex items-center gap-2">
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v12m0 0l-3-3m3 3l3-3m-6 8h6" />
                </svg>
                上传 .wasm 文件
              </span>
            </button>
            <input
              ref={uploadRef}
              type="file"
              accept=".wasm"
              onChange={handleUploadInstall}
              className="hidden"
            />
          </div>
        </div>

        {/* URL 安装 */}
        <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
            从 URL 下载安装
          </p>
          <div className="flex items-end gap-3 flex-wrap">
            <div className="flex-1 min-w-[180px]">
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                插件 ID
              </label>
              <input
                type="text"
                value={installUrlId}
                onChange={(e) => setInstallUrlId(e.target.value)}
                placeholder="my-plugin"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
            </div>
            <div className="flex-[2] min-w-[240px]">
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                WASM 文件 URL
              </label>
              <input
                type="text"
                value={installUrl}
                onChange={(e) => setInstallUrl(e.target.value)}
                placeholder="https://example.com/plugins/my-plugin.wasm"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
            </div>
            <button
              onClick={handleUrlInstall}
              disabled={installingUrl}
              className="px-5 py-2 bg-gray-800 dark:bg-gray-700 text-white text-sm font-medium hover:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50 whitespace-nowrap"
            >
              {installingUrl ? "安装中..." : "安装"}
            </button>
          </div>
        </div>
      </div>

      {/* 已安装插件列表 */}
      <div className="mg-box-shadow p-5">
        <div className="flex items-center justify-between mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100">
            已安装的插件
          </h2>
          <button
            onClick={handleReload}
            disabled={reloading}
            className="px-4 py-1.5 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-xs hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50"
          >
            <span className="flex items-center gap-1.5">
              <svg className={`w-3.5 h-3.5 ${reloading ? "animate-spin" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              {reloading ? "扫描中..." : "扫描目录刷新"}
            </span>
          </button>
        </div>

        {loading ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500">
            加载中...
          </div>
        ) : plugins.length === 0 ? (
          <div className="text-center py-8 border border-dashed border-gray-300 dark:border-gray-700">
            <p className="text-gray-400 dark:text-gray-500">暂无已安装的插件</p>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              通过上方区域上传 .wasm 文件或输入 URL 安装插件
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {plugins.map((p) => (
              <div
                key={p.id}
                className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <svg className="w-5 h-5 text-purple-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z" />
                    </svg>
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
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 ml-7">
                    {p.description || "暂无描述"}
                    <span className="ml-2 text-gray-400 dark:text-gray-500">
                      id: {p.id}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-2 ml-4 shrink-0">
                  <button
                    onClick={() => handleUninstall(p.id, p.name)}
                    className="px-3 py-1.5 text-xs border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    卸载
                  </button>
                </div>
              </div>
            ))}
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">
              共 {plugins.length} 个插件 · 删除后 .wasm 文件将被移除
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
