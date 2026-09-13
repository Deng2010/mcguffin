import { useState, useEffect, useRef } from "react";
import {
  getAdminPlugins,
  installPluginFromUrl,
  installPluginZip,
  setPluginEnabled,
  setPluginPermissions,
  setPluginsGloballyEnabled,
  uninstallPlugin,
  updatePluginFromUrl,
  updatePluginZip,
  type BackendPlugin,
  type PluginInstallResult,
} from "../../services/plugin.service";
import { useToast } from "../../errors/ToastContext";
import { PluginRegistry } from "../../plugins/registry";

// ── Component ──

export default function AdminPluginsPage() {
  const [backendPlugins, setBackendPlugins] = useState<BackendPlugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [globallyDisabled, setGloballyDisabled] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [knownPermissions, setKnownPermissions] = useState<string[]>([]);
  /** 正在编辑权限的插件 id */
  const [editingId, setEditingId] = useState<string | null>(null);
  /** 权限编辑草稿 */
  const [draftPerms, setDraftPerms] = useState<string[]>([]);
  const [savingPerms, setSavingPerms] = useState(false);
  /** 「从 URL 安装」输入框 */
  const [installUrl, setInstallUrl] = useState("");
  /** 正在更新的插件 id（更新是逐插件的操作） */
  const [updatingId, setUpdatingId] = useState<string | null>(null);
  const toast = useToast();
  const uploadRef = useRef<HTMLInputElement>(null);
  /** 「更新 .zip」的文件选择器与目标插件 id */
  const updateZipRef = useRef<HTMLInputElement>(null);
  const updateTargetRef = useRef<{ id: string; name: string } | null>(null);

  // ── Load backend plugins ──

  const loadBackendPlugins = async () => {
    setLoading(true);
    try {
      const res = await getAdminPlugins();
      setBackendPlugins(res.plugins);
      setGloballyDisabled(res.plugins_disabled === true);
      setKnownPermissions(res.known_permissions ?? []);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadBackendPlugins();
  }, []);

  // ── Plugin list ──

  const registry = PluginRegistry.getInstance();
  const displayPlugins = [...backendPlugins].sort((a, b) =>
    a.name.localeCompare(b.name),
  );

  // ── Install / update ──

  const showMsg = (text: string, type: "success" | "error" | "info") => {
    if (type === "success") toast.success(text);
    else if (type === "error") toast.error(text);
    else toast.info(text);
  };

  /** 安装 / 更新成功后的统一提示与刷新。 */
  const reportInstallResult = async (
    data: PluginInstallResult,
    fallbackName: string,
  ) => {
    const name = data.plugin?.name ?? fallbackName;
    const version = data.plugin?.version ? ` v${data.plugin.version}` : "";
    showMsg(`✅ ${data.message ?? `插件「${name}${version}」已就绪`}`, "success");
    if (data.new_permissions && data.new_permissions.length > 0) {
      showMsg(
        `⚠️ 新版本申请了新权限：${data.new_permissions.join(", ")}，` +
          `如需启用请在「权限」中勾选`,
        "info",
      );
    }
    // 动态加载 / 重新加载远程插件（无需刷新页面）
    await PluginRegistry.getInstance().refreshRemotePlugins();
    loadBackendPlugins();
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
      const data = await installPluginZip(buffer);
      await reportInstallResult(data, file.name);
    } catch (err) {
      showMsg(`安装失败: ${err}`, "error");
    } finally {
      setInstalling(false);
    }
    e.target.value = "";
  };

  /** 从 URL 安装 / 更新（服务端下载并记录来源 URL）。 */
  const handleInstallFromUrl = async () => {
    const url = installUrl.trim();
    if (!url) {
      showMsg("请填写插件包 URL", "error");
      return;
    }
    setInstalling(true);
    showMsg("正在从 URL 安装...", "info");
    try {
      const data = await installPluginFromUrl(url);
      await reportInstallResult(data, url);
      setInstallUrl("");
    } catch (err) {
      showMsg(`安装失败: ${err}`, "error");
    } finally {
      setInstalling(false);
    }
  };

  /** 选择一个 .zip 更新指定插件（包内 id 必须与插件一致）。 */
  const handleUpdateZip = (pluginId: string, pluginName: string) => {
    updateTargetRef.current = { id: pluginId, name: pluginName };
    updateZipRef.current?.click();
  };

  const handleUpdateZipSelected = async (
    e: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const file = e.target.files?.[0];
    const target = updateTargetRef.current;
    e.target.value = "";
    if (!file || !target) return;
    if (!file.name.endsWith(".zip")) {
      showMsg("请选择 .zip 文件", "error");
      return;
    }

    setUpdatingId(target.id);
    showMsg(`正在更新「${target.name}」...`, "info");
    try {
      const buffer = await file.arrayBuffer();
      const data = await updatePluginZip(target.id, buffer);
      await reportInstallResult(data, target.name);
    } catch (err) {
      showMsg(`更新失败: ${err}`, "error");
    } finally {
      setUpdatingId(null);
      updateTargetRef.current = null;
    }
  };

  /** 从安装时记录的来源 URL 更新（保留插件数据与已授权权限）。 */
  const handleUpdateFromUrl = async (pluginId: string, pluginName: string) => {
    setUpdatingId(pluginId);
    showMsg(`正在从原 URL 更新「${pluginName}」...`, "info");
    try {
      const data = await updatePluginFromUrl(pluginId);
      await reportInstallResult(data, pluginName);
    } catch (err) {
      showMsg(`更新失败: ${err}`, "error");
    } finally {
      setUpdatingId(null);
    }
  };

  // ── Uninstall ──

  const handleUninstall = async (pluginId: string, pluginName: string) => {
    if (!confirm(`确定要卸载插件「${pluginName}」吗？\n相关数据也会被删除。`))
      return;
    showMsg(`正在卸载 ${pluginName}...`, "info");
    try {
      const res = await uninstallPlugin(pluginId);
      if (!res.success) {
        showMsg(`卸载失败: ${res.message}`, "error");
        return;
      }
      showMsg(`✅ 已卸载「${pluginName}」`, "success");
      // 从本地注册中心移除（路由 / 导航 / 插槽立即失效）
      PluginRegistry.getInstance().remove(pluginId);
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
      const res = await setPluginsGloballyEnabled(!next);
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
      const res = await setPluginEnabled(pluginId, !currentEnabled);
      if (!res.success) {
        showMsg(`${actionLabel}失败: ${res.message}`, "error");
        return;
      }
      showMsg(`✅ 已${actionLabel}「${pluginName}」`, "success");
      // 启用 zip 插件后立即尝试加载；禁用后刷新状态（导航/路由同步隐藏）
      await PluginRegistry.getInstance().refreshRemotePlugins();
      loadBackendPlugins();
    } catch (err) {
      showMsg(`${actionLabel}失败: ${err}`, "error");
    }
  };

  // ── Permissions editing ──

  const startEditPerms = (p: BackendPlugin) => {
    setEditingId(p.id);
    setDraftPerms([...p.permissions_needed]);
  };

  const toggleDraftPerm = (perm: string) => {
    setDraftPerms((prev) =>
      prev.includes(perm) ? prev.filter((x) => x !== perm) : [...prev, perm],
    );
  };

  const handleSavePerms = async (pluginId: string, pluginName: string) => {
    setSavingPerms(true);
    try {
      const res = await setPluginPermissions(pluginId, draftPerms);
      if (!res.success) {
        showMsg(`权限保存失败: ${res.message}`, "error");
        return;
      }
      showMsg(`✅ 已更新「${pluginName}」的权限`, "success");
      setEditingId(null);
      loadBackendPlugins();
    } catch (err) {
      showMsg(`权限保存失败: ${err}`, "error");
    } finally {
      setSavingPerms(false);
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
            上传 .zip 格式的插件包（包含 plugin.json 和资源文件）；
            若插件 id 已安装，则按<strong>更新</strong>处理，保留插件数据与已授权权限
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

          <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
              从 URL 安装：填写可直接下载 .zip 的地址（例如插件仓库 Release 的
              <code className="mx-1">releases/latest/download/&lt;id&gt;.zip</code>）；
              来源会被记录，之后可一键「从原 URL 更新」
            </p>
            <div className="flex items-center gap-3">
              <input
                type="url"
                value={installUrl}
                onChange={(e) => setInstallUrl(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleInstallFromUrl();
                }}
                placeholder="https://github.com/<owner>/<repo>/releases/latest/download/my-plugin.zip"
                className="flex-1 min-w-0 px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-gray-800 dark:text-gray-200"
              />
              <button
                onClick={handleInstallFromUrl}
                disabled={installing}
                className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50 shrink-0"
              >
                {installing ? "处理中..." : "从 URL 安装"}
              </button>
            </div>
          </div>

          {/* 更新用的文件选择器（由插件列表中的「更新」按钮触发） */}
          <input
            ref={updateZipRef}
            type="file"
            accept=".zip"
            onChange={handleUpdateZipSelected}
            className="hidden"
          />
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
              通过上方区域上传 .zip 文件安装
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

                    {/* Remote load error badge */}
                    {registry.getPluginLoadError(p.id) && (
                      <span
                        className="text-xs px-1.5 py-0.5 bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                        title={registry.getPluginLoadError(p.id)}
                      >
                        加载失败
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
                    {p.source_url && (
                      <span
                        className="ml-2 text-gray-400 dark:text-gray-500"
                        title={p.source_url}
                      >
                        来源: {p.source_url}
                      </span>
                    )}
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2 ml-4 shrink-0">
                  {/* Update: upload a new zip for this plugin */}
                  <button
                    onClick={() => handleUpdateZip(p.id, p.name)}
                    disabled={updatingId === p.id}
                    title="上传新的 .zip 更新该插件（保留插件数据与已授权权限）"
                    className="px-3 py-1.5 text-xs border border-blue-300 dark:border-blue-800 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 disabled:opacity-50"
                  >
                    {updatingId === p.id ? "更新中..." : "更新"}
                  </button>
                  {/* Update from the recorded source URL */}
                  {p.source_url && (
                    <button
                      onClick={() => handleUpdateFromUrl(p.id, p.name)}
                      disabled={updatingId === p.id}
                      title={`从 ${p.source_url} 重新下载并更新`}
                      className="px-3 py-1.5 text-xs border border-blue-300 dark:border-blue-800 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 disabled:opacity-50"
                    >
                      从原 URL 更新
                    </button>
                  )}
                  {/* Permissions editor toggle */}
                  <button
                    onClick={() =>
                      editingId === p.id
                        ? setEditingId(null)
                        : startEditPerms(p)
                    }
                    className="px-3 py-1.5 text-xs border border-gray-300 dark:border-gray-600 text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    {editingId === p.id ? "收起权限" : "权限"}
                  </button>
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
                  <button
                    onClick={() => handleUninstall(p.id, p.name)}
                    className="px-3 py-1.5 text-xs border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    卸载
                  </button>
                </div>
              </div>
            ))}
            {/* 权限编辑面板（贴在对应插件条目下方） */}
            {displayPlugins
              .filter((p) => p.id === editingId)
              .map((p) => (
                <div
                  key={`${p.id}-perms`}
                  className="border border-t-0 border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/60 p-4 -mt-2"
                >
                  <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
                    勾选「{p.name}」被授予的插件权限（仅超级管理员可改；注册接口
                    不会修改已注册插件的权限）。
                  </p>
                  {knownPermissions.length === 0 ? (
                    <p className="text-xs text-gray-400">未获取到权限清单</p>
                  ) : (
                    <div className="flex flex-wrap gap-x-4 gap-y-2 mb-3">
                      {knownPermissions.map((perm) => (
                        <label
                          key={perm}
                          className="flex items-center gap-1.5 text-xs text-gray-700 dark:text-gray-300"
                        >
                          <input
                            type="checkbox"
                            checked={draftPerms.includes(perm)}
                            onChange={() => toggleDraftPerm(perm)}
                          />
                          {perm}
                        </label>
                      ))}
                    </div>
                  )}
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleSavePerms(p.id, p.name)}
                      disabled={savingPerms}
                      className="px-4 py-1.5 text-xs border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50"
                    >
                      {savingPerms ? "保存中..." : "保存权限"}
                    </button>
                    <button
                      onClick={() => setEditingId(null)}
                      className="px-4 py-1.5 text-xs border border-gray-300 dark:border-gray-600 text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800"
                    >
                      取消
                    </button>
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
