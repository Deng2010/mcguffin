import { useState, useEffect, createContext, useContext } from "react";
import { useAuth } from "../AuthContext";
import { apiFetch } from "../api";

// ============== Types ==============

interface ConfigData {
  server: { site_url: string; port: number };
  admin: { password: string; display_name: string };
  site: { name: string; title?: string | null; difficulty_order: string[] };
  oauth: { cp_client_id: string; cp_client_secret: string };
  backup: {
    interval_minutes: number;
    retention_count: number;
    backup_directory?: string | null;
  };
  difficulty: Record<string, { label: string; color: string }>;
  discussion_tags?: Record<string, { color: string; description: string }>;
  discussion_emojis?: Record<string, { char: string }>;
}

interface DifficultyEntry {
  name: string;
  label: string;
  color: string;
}

type TabId = "server" | "admin" | "site" | "oauth" | "difficulty" | "backup";

// ============== Component ==============

export default function AdminConfigPage() {
  const { user } = useAuth();
  const isSuperadmin = user?.role === "superadmin";
  const [activeTab, setActiveTab] = useState<TabId>("server");

  if (!isSuperadmin) {
    return (
      <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">
        权限不足
      </div>
    );
  }

  const tabs: { id: TabId; label: string }[] = [
    { id: "server", label: "服务器" },
    { id: "admin", label: "管理员" },
    { id: "site", label: "站点" },
    { id: "oauth", label: "OAuth" },
    { id: "difficulty", label: "难度" },
    { id: "backup", label: "备份" },
  ];

  return (
    <div>
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6 flex-wrap">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? "border-gray-800 text-gray-900 dark:border-gray-100 dark:text-gray-100"
                : "border-transparent text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-100"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <ConfigWrapper tab={activeTab} />
    </div>
  );
}

// ====================================================================
//  Config wrapper
// ====================================================================

const inputClass =
  "w-full px-4 py-2 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 focus:outline-none focus:border-gray-500 text-sm";

interface ConfigCtx {
  siteUrl: string;
  setSiteUrl: (v: string) => void;
  port: string;
  setPort: (v: string) => void;
  adminPassword: string;
  setAdminPassword: (v: string) => void;
  displayName: string;
  setDisplayName: (v: string) => void;
  siteName: string;
  setSiteName: (v: string) => void;
  siteTitle: string;
  setSiteTitle: (v: string) => void;
  cpClientId: string;
  setCpClientId: (v: string) => void;
  cpClientSecret: string;
  setCpClientSecret: (v: string) => void;
  difficulties: DifficultyEntry[];
  difficultyOrder: string[];
  newDiffName: string;
  setNewDiffName: (v: string) => void;
  newDiffLabel: string;
  setNewDiffLabel: (v: string) => void;
  newDiffColor: string;
  setNewDiffColor: (v: string) => void;
  updateDiff: (
    idx: number,
    field: keyof DifficultyEntry,
    value: string,
  ) => void;
  moveDiff: (idx: number, direction: -1 | 1) => void;
  removeDiff: (idx: number) => void;
  addDiff: () => void;
  backupInterval: number;
  setBackupInterval: (v: number) => void;
  backupRetention: number;
  setBackupRetention: (v: number) => void;
  backupDirectory: string;
  setBackupDirectory: (v: string) => void;
}

const ConfigCtx = createContext<ConfigCtx>(null!);

function ConfigWrapper({ tab }: { tab: TabId }) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [msg, setMsg] = useState("");

  const [siteUrl, setSiteUrl] = useState("");
  const [port, setPort] = useState("");
  const [adminPassword, setAdminPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [siteName, setSiteName] = useState("");
  const [siteTitle, setSiteTitle] = useState("");
  const [cpClientId, setCpClientId] = useState("");
  const [cpClientSecret, setCpClientSecret] = useState("");
  const [difficulties, setDifficulties] = useState<DifficultyEntry[]>([]);
  const [difficultyOrder, setDifficultyOrder] = useState<string[]>([]);
  const [newDiffName, setNewDiffName] = useState("");
  const [newDiffLabel, setNewDiffLabel] = useState("");
  const [newDiffColor, setNewDiffColor] = useState("#888888");
  const [backupInterval, setBackupInterval] = useState(60);
  const [backupRetention, setBackupRetention] = useState(48);
  const [backupDirectory, setBackupDirectory] = useState("");
  // 以下字段在页面上不显示编辑控件，但保存时必须保留以免被清空
  const [savedTags, setSavedTags] = useState<
    Record<string, { color: string; description: string }>
  >({});
  const [savedEmojis, setSavedEmojis] = useState<
    Record<string, { char: string }>
  >({});

  const loadConfig = async () => {
    setLoading(true);
    try {
      const res = await apiFetch<{
        success: boolean;
        config?: ConfigData;
        message?: string;
      }>("/admin/config");
      if (!res.success || !res.config) {
        setMsg(`加载配置失败: ${res.message}`);
        return;
      }
      setSiteUrl(res.config.server.site_url);
      setPort(String(res.config.server.port));
      setAdminPassword(res.config.admin.password);
      setDisplayName(res.config.admin.display_name);
      setSiteName(res.config.site.name);
      setSiteTitle(res.config.site.title ?? "");
      setCpClientId(res.config.oauth.cp_client_id);
      setCpClientSecret(res.config.oauth.cp_client_secret);
      setBackupInterval(res.config.backup?.interval_minutes ?? 60);
      setBackupRetention(res.config.backup?.retention_count ?? 48);
      setBackupDirectory(res.config.backup?.backup_directory ?? "");
      const allDiffs = Object.entries(res.config.difficulty).map(
        ([name, fields]) => ({
          name,
          label: fields.label,
          color: fields.color,
        }),
      );
      const order = res.config.site.difficulty_order ?? [];
      allDiffs.sort((a, b) => {
        const ai = order.indexOf(a.name);
        const bi = order.indexOf(b.name);
        return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi);
      });
      setDifficulties(allDiffs);
      setDifficultyOrder(
        allDiffs.length > 0 && order.length === 0
          ? allDiffs.map((d) => d.name)
          : order,
      );
      // 保存讨论标签和表情，确保保存配置时不会清空这些字段
      if (res.config.discussion_tags) setSavedTags(res.config.discussion_tags);
      if (res.config.discussion_emojis)
        setSavedEmojis(res.config.discussion_emojis);
    } catch (err) {
      setMsg(`加载配置失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadConfig();
  }, []);

  const updateDiff = (
    idx: number,
    field: keyof DifficultyEntry,
    value: string,
  ) => {
    setDifficulties((p) => {
      const n = [...p];
      n[idx] = { ...n[idx], [field]: value };
      return n;
    });
    if (field === "name") {
      setDifficultyOrder((p) => {
        const n = [...p];
        n[idx] = value;
        return n;
      });
    }
  };

  const moveDiff = (idx: number, dir: -1 | 1) => {
    const t = idx + dir;
    if (t < 0 || t >= difficulties.length) return;
    setDifficulties((p) => {
      const n = [...p];
      [n[idx], n[t]] = [n[t], n[idx]];
      return n;
    });
    setDifficultyOrder((p) => {
      const n = [...p];
      [n[idx], n[t]] = [n[t], n[idx]];
      return n;
    });
  };

  const removeDiff = (idx: number) => {
    const name = difficulties[idx].name;
    setDifficulties((p) => p.filter((_, i) => i !== idx));
    setDifficultyOrder((p) => p.filter((n) => n !== name));
  };

  const addDiff = () => {
    const name = newDiffName.trim();
    if (!name) return;
    if (difficulties.some((d) => d.name === name)) {
      setMsg(`难度 "${name}" 已存在`);
      return;
    }
    setDifficulties((p) => {
      setDifficultyOrder((o) => [...o, name]);
      return [
        ...p,
        { name, label: newDiffLabel.trim() || name, color: newDiffColor },
      ];
    });
    setNewDiffName("");
    setNewDiffLabel("");
    setNewDiffColor("#888888");
  };

  const handleSave = async () => {
    setSaving(true);
    setMsg("");
    try {
      const diffObj: Record<string, { label: string; color: string }> = {};
      for (const d of difficulties)
        if (d.name.trim())
          diffObj[d.name.trim()] = {
            label: d.label.trim() || d.name,
            color: d.color,
          };
      const order =
        difficultyOrder.length > 0
          ? difficultyOrder
          : difficulties.filter((d) => d.name.trim()).map((d) => d.name.trim());
      const res = await apiFetch<{ success: boolean; message: string }>(
        "/admin/config",
        {
          method: "PUT",
          body: JSON.stringify({
            server: {
              site_url: siteUrl,
              port: parseInt(port) || 3000,
            },
            admin: { password: adminPassword, display_name: displayName },
            site: {
              name: siteName,
              title: siteTitle || undefined,
              difficulty_order: order,
            },
            oauth: {
              cp_client_id: cpClientId,
              cp_client_secret: cpClientSecret,
            },
            backup: {
              interval_minutes: backupInterval,
              retention_count: backupRetention,
              backup_directory: backupDirectory || null,
            },
            difficulty: diffObj,
            discussion_tags: savedTags,
            discussion_emojis: savedEmojis,
          }),
        },
      );
      if (!res.success) {
        setMsg(`保存失败: ${res.message}`);
        return;
      }
      setMsg(res.message);
      setTimeout(() => setMsg(""), 5000);
    } catch (err) {
      setMsg(`保存失败: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const handleRestart = async () => {
    if (!window.confirm("确定要重启服务吗？服务会短暂中断（约2-3秒）。"))
      return;
    setRestarting(true);
    setMsg("");
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        "/admin/restart",
        { method: "POST" },
      );
      if (!res.success) {
        setMsg(`重启失败: ${res.message}`);
        setRestarting(false);
        return;
      }
      setMsg("服务正在重启，页面将在几秒后重载...");
      setTimeout(() => window.location.reload(), 5000);
    } catch (err) {
      setMsg(`重启失败: ${err}`);
      setRestarting(false);
    }
  };

  const ctx: ConfigCtx = {
    siteUrl,
    setSiteUrl,
    port,
    setPort,
    adminPassword,
    setAdminPassword,
    displayName,
    setDisplayName,
    siteName,
    setSiteName,
    siteTitle,
    setSiteTitle,
    cpClientId,
    setCpClientId,
    cpClientSecret,
    setCpClientSecret,
    difficulties,
    difficultyOrder,
    newDiffName,
    setNewDiffName,
    newDiffLabel,
    setNewDiffLabel,
    newDiffColor,
    setNewDiffColor,
    updateDiff,
    moveDiff,
    removeDiff,
    addDiff,
    backupInterval,
    setBackupInterval,
    backupRetention,
    setBackupRetention,
    backupDirectory,
    setBackupDirectory,
  };

  if (loading)
    return (
      <div>
        <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 shadow p-5 mb-6">
          <div className="text-center py-8 text-gray-400 dark:text-gray-500">
            加载配置中...
          </div>
        </div>
      </div>
    );

  const tabContent = () => {
    switch (tab) {
      case "server":
        return <ServerForm />;
      case "admin":
        return <AdminForm />;
      case "site":
        return <SiteForm />;
      case "oauth":
        return <OAuthForm />;
      case "difficulty":
        return <DifficultyForm />;
      case "backup":
        return <BackupForm />;
    }
  };

  return (
    <ConfigCtx.Provider value={ctx}>
      {msg && (
        <div
          className={`mb-4 p-3 text-sm border ${
            msg.includes("失败")
              ? "bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300"
              : "bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300"
          }`}
        >
          {msg}
        </div>
      )}

      <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 shadow p-5 mb-6">
        {tabContent()}
      </div>

      <div className="flex gap-3 items-center">
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-6 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
        >
          {saving ? "保存中..." : "保存配置"}
        </button>
        <button
          onClick={handleRestart}
          disabled={restarting}
          className="px-6 py-2 border border-yellow-500 text-yellow-700 text-sm hover:bg-yellow-50 disabled:opacity-50 dark:border-yellow-800 dark:text-yellow-400 dark:hover:bg-yellow-900/20"
        >
          {restarting ? "重启中..." : "重启服务"}
        </button>
        <p className="text-xs text-gray-400 dark:text-gray-500 ml-2">
          服务器/OAuth/管理员密码修改需重启服务才能生效。难度配置保存后立即生效。
        </p>
      </div>
    </ConfigCtx.Provider>
  );
}

// ============== Config Form Sub-Components ==============

function ServerForm() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          站点 URL
        </label>
        <input
          type="text"
          value={c.siteUrl}
          onChange={(e) => c.setSiteUrl(e.target.value)}
          className={inputClass}
          placeholder="https://lba-oi.team"
        />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          端口
        </label>
        <input
          type="number"
          value={c.port}
          onChange={(e) => c.setPort(e.target.value)}
          className={inputClass}
          placeholder="3000"
        />
      </div>
    </div>
  );
}

function AdminForm() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          登录密码
        </label>
        <input
          type="text"
          value={c.adminPassword}
          onChange={(e) => c.setAdminPassword(e.target.value)}
          className={inputClass}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          修改后需重启服务生效
        </p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          显示名称
        </label>
        <input
          type="text"
          value={c.displayName}
          onChange={(e) => c.setDisplayName(e.target.value)}
          className={inputClass}
        />
      </div>
    </div>
  );
}

function SiteForm() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          站点名称
        </label>
        <input
          type="text"
          value={c.siteName}
          onChange={(e) => c.setSiteName(e.target.value)}
          className={inputClass}
          placeholder="McGuffin"
        />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          网页标题
        </label>
        <input
          type="text"
          value={c.siteTitle}
          onChange={(e) => c.setSiteTitle(e.target.value)}
          className={inputClass}
          placeholder="与站点名称相同"
        />
      </div>
    </div>
  );
}

function OAuthForm() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          Client ID
        </label>
        <input
          type="text"
          value={c.cpClientId}
          onChange={(e) => c.setCpClientId(e.target.value)}
          className={inputClass}
        />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          Client Secret
        </label>
        <input
          type="text"
          value={c.cpClientSecret}
          onChange={(e) => c.setCpClientSecret(e.target.value)}
          className={inputClass}
        />
      </div>
    </div>
  );
}

function BackupForm() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          自动备份间隔（分钟）
        </label>
        <input
          type="number"
          min={10}
          max={1440}
          value={c.backupInterval}
          onChange={(e) => c.setBackupInterval(parseInt(e.target.value) || 60)}
          className={inputClass}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          每隔多少分钟自动备份一次。最小值 10 分钟，最大值 1440 分钟（24
          小时）。
        </p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          最大备份保留数量
        </label>
        <input
          type="number"
          min={1}
          max={999}
          value={c.backupRetention}
          onChange={(e) => c.setBackupRetention(parseInt(e.target.value) || 48)}
          className={inputClass}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          最多保留多少个自动备份文件。超出数量的旧备份会被自动清理。
        </p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          备份目录（留空使用默认位置）
        </label>
        <input
          type="text"
          value={c.backupDirectory}
          onChange={(e) => c.setBackupDirectory(e.target.value)}
          className={inputClass}
          placeholder="留空则使用 data 目录下的 backups/"
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          自定义备份文件、导出数据、导入数据的存放目录。修改后立即生效。
        </p>
      </div>
      <p className="text-xs text-gray-400 dark:text-gray-500">
        备份间隔和保留数量修改后需重启服务生效。
      </p>
    </div>
  );
}

function DifficultyForm() {
  const c = useContext(ConfigCtx);
  return (
    <div>
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
        添加、编辑或删除难度等级。名称用作内部标识（如
        Easy），标签显示给用户（如 简单），颜色用于 UI 展示。使用 ↑↓
        按钮调整显示顺序。
      </p>
      <div className="space-y-3">
        {c.difficulties.map((d, i) => (
          <div
            key={i}
            className="flex items-center gap-2 bg-gray-50 dark:bg-gray-800/50 p-2"
          >
            <div className="flex flex-col gap-0.5">
              <button
                onClick={() => c.moveDiff(i, -1)}
                disabled={i === 0}
                className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1"
              >
                ↑
              </button>
              <button
                onClick={() => c.moveDiff(i, 1)}
                disabled={i === c.difficulties.length - 1}
                className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1"
              >
                ↓
              </button>
            </div>
            <span className="text-xs text-gray-400 w-5 text-right">
              {i + 1}
            </span>
            <input
              type="text"
              value={d.name}
              onChange={(e) => c.updateDiff(i, "name", e.target.value)}
              className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
              placeholder="名称"
            />
            <input
              type="text"
              value={d.label}
              onChange={(e) => c.updateDiff(i, "label", e.target.value)}
              className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
              placeholder="标签"
            />
            <input
              type="color"
              value={d.color}
              onChange={(e) => c.updateDiff(i, "color", e.target.value)}
              className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer"
            />
            <span className="text-xs text-gray-500 dark:text-gray-400 w-20">
              {d.color}
            </span>
            <button
              onClick={() => c.removeDiff(i)}
              className="px-2 py-1 text-red-600 text-sm hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
            >
              删除
            </button>
          </div>
        ))}
        <div className="flex items-center gap-2 bg-blue-50 dark:bg-blue-900/30 p-2 border border-dashed border-blue-300 dark:border-blue-800">
          <input
            type="text"
            value={c.newDiffName}
            onChange={(e) => c.setNewDiffName(e.target.value)}
            className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
            placeholder="新难度名称"
          />
          <input
            type="text"
            value={c.newDiffLabel}
            onChange={(e) => c.setNewDiffLabel(e.target.value)}
            className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
            placeholder="显示标签"
          />
          <input
            type="color"
            value={c.newDiffColor}
            onChange={(e) => c.setNewDiffColor(e.target.value)}
            className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer"
          />
          <button
            onClick={c.addDiff}
            className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600"
          >
            添加
          </button>
        </div>
      </div>
    </div>
  );
}
