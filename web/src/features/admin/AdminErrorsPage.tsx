// ============== 错误中心 ==============
// 统一查看前后端上报的错误：错误码、消息、次数、来源、修改建议、堆栈；
// 支持按状态/错误码/来源过滤，状态流转与删除/清空。

import { useCallback, useEffect, useState } from "react";
import {
  clearErrors,
  deleteError,
  fetchErrorReports,
  updateErrorStatus,
  type ErrorReportRow,
  type ErrorReportStatus,
} from "../../services/error.service";
import { lookupSuggestion } from "../../errors/registry";
import { useToast } from "../../errors/ToastContext";

const STATUS_OPTIONS: { value: ErrorReportStatus | ""; label: string }[] = [
  { value: "", label: "全部状态" },
  { value: "open", label: "待处理" },
  { value: "investigating", label: "排查中" },
  { value: "resolved", label: "已解决" },
  { value: "ignored", label: "已忽略" },
];

const STATUS_BADGE: Record<ErrorReportStatus, string> = {
  open: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
  investigating: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/40 dark:text-yellow-300",
  resolved: "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-300",
  ignored: "bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400",
};

const STATUS_LABEL: Record<ErrorReportStatus, string> = {
  open: "待处理",
  investigating: "排查中",
  resolved: "已解决",
  ignored: "已忽略",
};

function formatTime(ts: string): string {
  try {
    return new Date(ts).toLocaleString("zh-CN", { hour12: false });
  } catch {
    return ts;
  }
}

export default function AdminErrorsPage() {
  const toast = useToast();
  const [rows, setRows] = useState<ErrorReportRow[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState<ErrorReportStatus | "">("");
  const [code, setCode] = useState("");
  const [source, setSource] = useState("");
  const [selected, setSelected] = useState<ErrorReportRow | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetchErrorReports({
        status: status || undefined,
        code: code.trim() || undefined,
        source: source || undefined,
      });
      if (res.success) {
        setRows(res.errors);
        setTotal(res.total);
      }
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "加载错误报告失败",
        err instanceof Error ? err.stack : undefined,
      );
    } finally {
      setLoading(false);
    }
  }, [status, code, source]);

  useEffect(() => {
    load();
  }, [load]);

  const handleStatus = async (row: ErrorReportRow, next: ErrorReportStatus) => {
    try {
      await updateErrorStatus(row.id, next);
      toast.success(`已标记为「${STATUS_LABEL[next]}」`);
      setSelected((cur) => (cur && cur.id === row.id ? { ...cur, status: next } : cur));
      load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "更新状态失败");
    }
  };

  const handleDelete = async (row: ErrorReportRow) => {
    if (!window.confirm("确定删除这条错误报告吗？")) return;
    try {
      await deleteError(row.id);
      toast.success("错误报告已删除");
      if (selected?.id === row.id) setSelected(null);
      load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "删除失败");
    }
  };

  const handleClear = async () => {
    if (!window.confirm(`确定清空全部 ${total} 条错误报告吗？此操作不可恢复。`)) return;
    try {
      await clearErrors();
      toast.success("错误报告已清空");
      setSelected(null);
      load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "清空失败");
    }
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-gray-800 dark:text-gray-100">错误中心</h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            前后端错误统一收集（共 {total} 条）
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={load}
            className="border border-gray-300 dark:border-gray-600 px-4 py-2 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
          >
            刷新
          </button>
          <button
            type="button"
            onClick={handleClear}
            disabled={total === 0}
            className="border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 px-4 py-2 text-sm disabled:opacity-40 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
          >
            清空
          </button>
        </div>
      </div>

      {/* 过滤器 */}
      <div className="flex flex-wrap items-center gap-3 mb-4">
        <select
          value={status}
          onChange={(e) => setStatus(e.target.value as ErrorReportStatus | "")}
          className="border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-700 dark:text-gray-200 focus:outline-none"
        >
          {STATUS_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
        <input
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder="错误码，如 PROBLEM_NOT_FOUND"
          className="border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-700 dark:text-gray-200 focus:outline-none w-72"
        />
        <select
          value={source}
          onChange={(e) => setSource(e.target.value)}
          className="border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-700 dark:text-gray-200 focus:outline-none"
        >
          <option value="">全部来源</option>
          <option value="frontend">前端</option>
          <option value="backend">后端</option>
        </select>
      </div>

      {loading ? (
        <p className="text-sm text-gray-400 dark:text-gray-500 py-8 text-center">加载中…</p>
      ) : rows.length === 0 ? (
        <p className="text-sm text-gray-400 dark:text-gray-500 py-8 text-center">
          暂无错误报告 🎉
        </p>
      ) : (
        <div className="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-gray-400 dark:text-gray-500 border-b border-gray-200 dark:border-gray-700">
                <th className="px-3 py-2 font-medium">错误码</th>
                <th className="px-3 py-2 font-medium">消息</th>
                <th className="px-3 py-2 font-medium">次数</th>
                <th className="px-3 py-2 font-medium">来源</th>
                <th className="px-3 py-2 font-medium">状态</th>
                <th className="px-3 py-2 font-medium">最近出现</th>
                <th className="px-3 py-2 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  key={row.id}
                  className="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/40 cursor-pointer"
                  onClick={() => setSelected(row)}
                >
                  <td className="px-3 py-2 font-mono text-xs text-blue-600 dark:text-blue-400">
                    {row.code}
                  </td>
                  <td className="px-3 py-2 text-gray-700 dark:text-gray-200 max-w-xs truncate">
                    {row.message}
                  </td>
                  <td className="px-3 py-2 text-gray-600 dark:text-gray-300">{row.count}</td>
                  <td className="px-3 py-2 text-gray-600 dark:text-gray-300">
                    {row.source === "frontend" ? "前端" : "后端"}
                  </td>
                  <td className="px-3 py-2">
                    <span
                      className={`inline-block px-2 py-0.5 text-xs ${STATUS_BADGE[row.status]}`}
                    >
                      {STATUS_LABEL[row.status]}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-gray-500 dark:text-gray-400 text-xs">
                    {formatTime(row.last_seen)}
                  </td>
                  <td className="px-3 py-2" onClick={(e) => e.stopPropagation()}>
                    <button
                      type="button"
                      onClick={() => handleDelete(row)}
                      className="text-xs text-red-500 hover:text-red-700 dark:hover:text-red-400"
                    >
                      删除
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* 详情抽屉 */}
      {selected && (
        <div className="fixed inset-0 z-50 bg-black/30" onClick={() => setSelected(null)}>
          <div
            className="absolute right-0 top-0 h-full w-full max-w-lg bg-white dark:bg-gray-900 shadow-xl overflow-y-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="p-5 border-b border-gray-200 dark:border-gray-700 flex items-start justify-between">
              <div>
                <h2 className="text-lg font-bold text-gray-800 dark:text-gray-100">
                  错误详情
                </h2>
                <p className="font-mono text-xs text-blue-600 dark:text-blue-400 mt-1">
                  {selected.code} · 出现 {selected.count} 次
                </p>
              </div>
              <button
                type="button"
                onClick={() => setSelected(null)}
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                aria-label="关闭"
              >
                ×
              </button>
            </div>

            <div className="p-5 space-y-4 text-sm">
              <Section label="消息">
                <p className="text-gray-800 dark:text-gray-100 break-words">{selected.message}</p>
              </Section>
              <Section label="用户提示（hint）">
                <p className="text-gray-700 dark:text-gray-300">
                  {selected.hint || lookupSuggestion(selected.code)}
                </p>
              </Section>
              <Section label="修改建议（suggestion）">
                <p className="text-gray-700 dark:text-gray-300">
                  {selected.suggestion || lookupSuggestion(selected.code)}
                </p>
              </Section>
              <Section label="状态">
                <div className="flex flex-wrap gap-2">
                  {(Object.keys(STATUS_LABEL) as ErrorReportStatus[]).map((s) => (
                    <button
                      key={s}
                      type="button"
                      onClick={() => handleStatus(selected, s)}
                      className={`px-3 py-1 text-xs border transition-colors ${
                        selected.status === s
                          ? "border-gray-800 dark:border-gray-100 bg-gray-800 dark:bg-gray-100 text-white dark:text-gray-900"
                          : "border-gray-300 dark:border-gray-600 text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
                      }`}
                    >
                      {STATUS_LABEL[s]}
                    </button>
                  ))}
                </div>
              </Section>
              <Section label="上下文">
                <dl className="space-y-1 text-xs text-gray-600 dark:text-gray-400">
                  <ContextRow k="路由" v={selected.route || "-"} />
                  <ContextRow k="来源" v={selected.source} />
                  <ContextRow k="HTTP" v={selected.http_status != null ? String(selected.http_status) : "-"} />
                  <ContextRow k="用户" v={selected.user_id || "-"} />
                  <ContextRow k="插件" v={selected.plugin_id || "-"} />
                  <ContextRow k="UA" v={selected.ua || "-"} />
                  <ContextRow k="首次" v={formatTime(selected.first_seen)} />
                  <ContextRow k="最近" v={formatTime(selected.last_seen)} />
                  {selected.resolved_by && (
                    <ContextRow k="处理人" v={selected.resolved_by} />
                  )}
                  {selected.resolved_at && (
                    <ContextRow k="处理时间" v={formatTime(selected.resolved_at)} />
                  )}
                </dl>
              </Section>
              {selected.stack && (
                <Section label="堆栈">
                  <pre className="text-xs bg-gray-100 dark:bg-gray-800 p-3 rounded overflow-x-auto whitespace-pre-wrap break-words text-gray-700 dark:text-gray-300 max-h-64 overflow-y-auto">
                    {selected.stack}
                  </pre>
                </Section>
              )}
              <div className="flex gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => handleDelete(selected)}
                  className="flex-1 border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 px-4 py-2 text-sm hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                >
                  删除
                </button>
                <button
                  type="button"
                  onClick={() => setSelected(null)}
                  className="flex-1 border border-gray-300 dark:border-gray-600 text-gray-600 dark:text-gray-300 px-4 py-2 text-sm hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                >
                  关闭
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-gray-400 dark:text-gray-500 mb-1">{label}</h3>
      {children}
    </div>
  );
}

function ContextRow({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex gap-2">
      <dt className="w-14 text-gray-400 dark:text-gray-500 flex-shrink-0">{k}</dt>
      <dd className="break-words">{v}</dd>
    </div>
  );
}
