import { useState, useEffect, useRef } from "react";
import { apiFetch } from "../api";

interface BackupEntry {
  name: string;
  size: number;
  modified: string;
  type?: string;
}

export default function AdminBackupsPage() {
  const [backups, setBackups] = useState<BackupEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [msg, setMsg] = useState("");
  const importFileRef = useRef<HTMLInputElement>(null);
  const importConfigRef = useRef<HTMLInputElement>(null);
  const importDbRef = useRef<HTMLInputElement>(null);

  const load = async () => {
    setLoading(true);
    try {
      const res = await apiFetch<{ success: boolean; backups: BackupEntry[] }>(
        "/admin/backups",
      );
      if (res.success) setBackups(res.backups);
    } catch (err) {
      setMsg(`加载备份列表失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    load();
  }, []);

  const handleCreate = async () => {
    setCreating(true);
    setMsg("");
    try {
      const res = await apiFetch<{
        success: boolean;
        message: string;
        backup?: string;
      }>("/admin/backup", { method: "POST" });
      if (!res.success) {
        setMsg(`备份失败: ${res.message}`);
        return;
      }
      setMsg(`备份已创建: ${res.backup}`);
      load();
      setTimeout(() => setMsg(""), 5000);
    } catch (err) {
      setMsg(`备份失败: ${err}`);
    } finally {
      setCreating(false);
    }
  };

  const handleRestore = async (name: string) => {
    if (!confirm(`确定要从备份「${name}」恢复吗？`)) return;
    try {
      const res = await apiFetch<any>(
        `/admin/backup/restore/${encodeURIComponent(name)}`,
        { method: "POST" },
      );
      setMsg(res.success ? `✅ ${res.message}` : `恢复失败: ${res.message}`);
      if (res.success) load();
    } catch (err) {
      setMsg(`恢复失败: ${err}`);
    }
  };

  const handleDelete = async (name: string) => {
    if (!confirm(`确定要删除备份「${name}」吗？`)) return;
    try {
      const res = await apiFetch<any>(
        `/admin/backup/${encodeURIComponent(name)}`,
        { method: "DELETE" },
      );
      setMsg(res.success ? `已删除: ${name}` : `删除失败: ${res.message}`);
      if (res.success) load();
    } catch (err) {
      setMsg(`删除失败: ${err}`);
    }
  };

  const fmtSize = (b: number) =>
    b > 1048576
      ? `${(b / 1048576).toFixed(1)} MB`
      : b > 1024
        ? `${(b / 1024).toFixed(1)} KB`
        : `${b} B`;

  const handleDownload = async (name: string) => {
    try {
      const res = await apiFetch<any>(
        `/admin/backup/download/${encodeURIComponent(name)}`,
      );
      if (!res.success) {
        setMsg(`下载失败: ${res.message}`);
        return;
      }
      const blob =
        res.encoding === "base64"
          ? new Blob(
              [Uint8Array.from(atob(res.content), (c) => c.charCodeAt(0))],
              { type: res.mime },
            )
          : new Blob([res.content], { type: res.mime });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = res.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setMsg(`已下载: ${res.filename}`);
    } catch (err) {
      setMsg(`下载失败: ${err}`);
    }
  };

  const doExport = async (type: string) => {
    try {
      const res = await apiFetch<any>(`/admin/export/${type}`);
      if (!res.success) {
        setMsg(`导出失败: ${res.message}`);
        return;
      }
      const blob = new Blob([res.content!], { type: res.mime });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = res.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setMsg(`已导出: ${res.filename}`);
    } catch (err) {
      setMsg(`导出失败: ${err}`);
    }
  };

  const handleImportFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const content = await file.text();
    if (
      !confirm(
        `确定要从「${file.name}」导入数据吗？\n此操作将替换所有现有数据！`,
      )
    )
      return;
    try {
      const res = await apiFetch<any>("/admin/import/data", {
        method: "POST",
        body: JSON.stringify({ content }),
        headers: { "Content-Type": "application/json" },
      });
      setMsg(res.success ? `✅ ${res.message}` : `导入失败: ${res.message}`);
      if (res.success) load();
    } catch (err) {
      setMsg(`导入失败: ${err}`);
    }
    e.target.value = "";
  };

  /** 从 .db 文件恢复（base64 上传到服务器再 restore） */
  const handleImportDb = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (
      !confirm(
        `确定要从 .db 备份文件「${file.name}」恢复吗？\n此操作将替换所有现有数据！`,
      )
    ) {
      e.target.value = "";
      return;
    }
    try {
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      let binary = "";
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      const content = btoa(binary);
      const res = await apiFetch<any>("/admin/backup/restore-upload", {
        method: "POST",
        body: JSON.stringify({ content, filename: file.name }),
        headers: { "Content-Type": "application/json" },
      });
      setMsg(res.success ? `✅ ${res.message}` : `恢复失败: ${res.message}`);
      if (res.success) load();
    } catch (err) {
      setMsg(`恢复失败: ${err}`);
    }
    e.target.value = "";
  };

  const handleImportConfig = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const content = await file.text();
    if (!confirm(`确定要从「${file.name}」导入配置吗？`)) return;
    try {
      const res = await apiFetch<any>("/admin/import/config", {
        method: "POST",
        body: JSON.stringify({ content }),
        headers: { "Content-Type": "application/json" },
      });
      setMsg(res.success ? `✅ ${res.message}` : `导入失败: ${res.message}`);
    } catch (err) {
      setMsg(`导入失败: ${err}`);
    }
    e.target.value = "";
  };

  return (
    <div>
      {msg && (
        <div
          className={`mb-4 p-3 text-sm border ${
            msg.startsWith("✅")
              ? "bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300"
              : msg.includes("失败")
                ? "bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300"
                : "bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-700"
          }`}
        >
          {msg}
        </div>
      )}

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow p-5 mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">
          导出
        </h2>
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">
          导出当前数据或配置文件，下载到本地。
        </p>
        <div className="flex items-center gap-3">
          <button
            onClick={() => doExport("data")}
            className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-800 dark:border-gray-600 text-gray-800 dark:text-gray-200 text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-800"
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
                  d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
              导出数据文件
            </span>
          </button>
          <button
            onClick={() => doExport("config")}
            className="px-5 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow text-gray-700 dark:text-gray-200 text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
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
                  d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
              导出配置文件
            </span>
          </button>
        </div>
        <div className="flex items-center gap-3 mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
          <p className="text-xs text-gray-500 dark:text-gray-400 mr-2">
            导入：
          </p>
          <input
            ref={importFileRef}
            type="file"
            accept=".json"
            onChange={handleImportFile}
            className="hidden"
          />
          <input
            ref={importConfigRef}
            type="file"
            accept=".toml"
            onChange={handleImportConfig}
            className="hidden"
          />
          <input
            ref={importDbRef}
            type="file"
            accept=".db"
            onChange={handleImportDb}
            className="hidden"
          />
          <button
            onClick={() => importFileRef.current?.click()}
            className="px-5 py-2 bg-white dark:bg-gray-900 border border-orange-600 dark:border-orange-700 text-orange-700 dark:text-orange-400 text-sm font-medium hover:bg-orange-50 dark:hover:bg-orange-900/20"
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
                  d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5 5-5M12 15V3"
                />
              </svg>
              导入数据
            </span>
          </button>
          <button
            onClick={() => importConfigRef.current?.click()}
            className="px-5 py-2 bg-white dark:bg-gray-900 border border-orange-300 dark:border-orange-800 text-orange-600 dark:text-orange-400 text-sm hover:bg-orange-50 dark:hover:bg-orange-900/20"
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
                  d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5 5-5M12 15V3"
                />
              </svg>
              导入配置
            </span>
          </button>
          <button
            onClick={() => importDbRef.current?.click()}
            className="px-5 py-2 bg-white dark:bg-gray-900 border border-purple-600 dark:border-purple-700 text-purple-700 dark:text-purple-400 text-sm font-medium hover:bg-purple-50 dark:hover:bg-purple-900/20"
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
                  d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5 5-5M12 15V3"
                />
              </svg>
              从 .db 恢复
            </span>
          </button>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow p-5">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3 pb-2 border-b border-gray-200 dark:border-gray-700">
          备份管理
        </h2>
        <div className="flex items-center gap-3 mb-4">
          <button
            onClick={handleCreate}
            disabled={creating}
            className="px-5 py-2 bg-gray-800 dark:bg-gray-700 text-white text-sm font-medium hover:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
          >
            {creating ? "创建中..." : "创建备份"}
          </button>
          <button
            onClick={load}
            disabled={loading}
            className="px-5 py-2 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50"
          >
            刷新
          </button>
        </div>
        {loading ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500">
            加载中...
          </div>
        ) : backups.length === 0 ? (
          <div className="text-center py-8 border border-dashed border-gray-300 dark:border-gray-700">
            <p className="text-gray-400 dark:text-gray-500">暂无备份</p>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              点击上方「创建备份」按钮创建第一个备份
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {backups.map((b) => (
              <div
                key={b.name}
                className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <svg
                      className="w-5 h-5 text-gray-400 dark:text-gray-500 shrink-0"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={1.5}
                        d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
                      />
                    </svg>
                    <span className="font-medium text-gray-800 dark:text-gray-100 truncate text-sm">
                      {b.name}
                    </span>
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 ml-7">
                    {fmtSize(b.size)} · {b.modified}
                  </div>
                </div>
                <div className="flex items-center gap-2 ml-4 shrink-0">
                  <button
                    onClick={() => handleDownload(b.name)}
                    className="px-3 py-1.5 text-xs border border-blue-500 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20"
                  >
                    下载
                  </button>
                  <button
                    onClick={() => handleRestore(b.name)}
                    className="px-3 py-1.5 text-xs border border-green-600 dark:border-green-800 text-green-700 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20"
                  >
                    恢复
                  </button>
                  <button
                    onClick={() => handleDelete(b.name)}
                    className="px-3 py-1.5 text-xs border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    删除
                  </button>
                </div>
              </div>
            ))}
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">
              共 {backups.length} 个备份 · 恢复操作会自动创建当前数据的
              pre_restore_ 安全快照
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
