import { useState, useEffect } from "react";
import { useAuthStore } from "../../stores/authStore";
import { apiFetch } from "../../services/api";
import Tabs from "../../components/ui/Tabs";
import type { TabId, ConfigData, DifficultyEntry } from "./config-context";
import { ConfigCtx } from "./config-context";
import ServerSection from "./sections/ServerSection";
import AdminSection from "./sections/AdminSection";
import SiteSection from "./sections/SiteSection";
import OAuthSection from "./sections/OAuthSection";
import DifficultySection from "./sections/DifficultySection";
import BackupSection from "./sections/BackupSection";
import DiscussionsSection from "./sections/DiscussionsSection";
import GroupsSection from "./sections/GroupsSection";

const tabs: { id: TabId; label: string }[] = [
  { id: "server", label: "服务器" },
  { id: "admin", label: "管理员" },
  { id: "site", label: "站点" },
  { id: "oauth", label: "CP OAuth" },
  { id: "difficulty", label: "难度" },
  { id: "backup", label: "备份" },
  { id: "discussions", label: "讨论区" },
  { id: "groups", label: "成员组" },
];

export default function AdminConfigPage() {
  const { user } = useAuthStore();
  const isSuperadmin = user?.role === "superadmin";
  const [activeTab, setActiveTab] = useState<TabId>("server");

  if (!isSuperadmin) {
    return (
      <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">
        权限不足
      </div>
    );
  }

  return (
    <div>
      <Tabs
        tabs={tabs}
        activeTab={activeTab}
        onChange={(id) => setActiveTab(id as TabId)}
        wrap
      />
      <ConfigWrapper tab={activeTab} />
    </div>
  );
}

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
    if (!adminPassword.trim()) {
      setMsg("管理员密码不能为空");
      return;
    }
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
          : difficulties
              .filter((d) => d.name.trim())
              .map((d) => d.name.trim());
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

  const exportConfig = async () => {
    try {
      const res = await apiFetch<any>("/admin/export/config");
      if (!res.success) {
        setMsg(`导出失败: ${res.message}`);
        return;
      }
      const blob = new Blob([res.content], { type: res.mime });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = res.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      setMsg(`导出失败: ${err}`);
    }
  };

  const ctx = {
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
    discussionTags: savedTags,
    setDiscussionTags: setSavedTags,
    discussionEmojis: savedEmojis,
    setDiscussionEmojis: setSavedEmojis,
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
        return <ServerSection />;
      case "admin":
        return <AdminSection />;
      case "site":
        return <SiteSection />;
      case "oauth":
        return <OAuthSection />;
      case "difficulty":
        return <DifficultySection />;
      case "backup":
        return <BackupSection />;
      case "discussions":
        return <DiscussionsSection />;
      case "groups":
        return <GroupsSection />;
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
          onClick={exportConfig}
          className="px-6 py-2 border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-200 text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
        >
          导出配置文件
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
