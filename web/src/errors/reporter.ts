// ============== 前端错误捕获与上报 ==============
//
// - 全局捕获：window error（含资源加载失败）、unhandledrejection、console.error（节流）
// - 队列：去重 + 批量（10 秒或 20 条），单会话上限 100 条
// - 离线：localStorage 待发队列（上限 100），上线后补发，页面卸载用 sendBeacon
// - 上报接口：POST /api/errors/report（免鉴权，服务端按 IP 限流，带 token 时附加 user_id）

import { getToken } from "../services/api";
import { normalizeError, NormalizedError } from "./normalize";

const QUEUE_KEY = "mcguffin.error.queue.v1";
const MAX_QUEUE = 100;
const MAX_SESSION = 100;
const BATCH_SIZE = 20;
const FLUSH_INTERVAL_MS = 10_000;
const REPORT_ENDPOINT = "/api/errors/report";

export interface ReportPayload {
  source: string;
  code: string;
  message: string;
  hint: string;
  suggestion?: string;
  stack?: string;
  url: string;
  route: string;
  method?: string;
  http_status?: number;
  ua?: string;
  plugin_id?: string;
}

let queue: ReportPayload[] = [];
let sessionCount = 0;
let flushTimer: number | null = null;
let initialized = false;

function fingerprint(p: ReportPayload): string {
  const stackHead = (p.stack || "").split("\n").slice(0, 3).join("\n").slice(0, 200);
  return [p.source, p.code, p.message, stackHead, p.route].join("|");
}

function restoreBacklog() {
  try {
    const raw = localStorage.getItem(QUEUE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      queue = parsed.slice(0, MAX_QUEUE);
    }
  } catch {
    /* 忽略损坏的待发队列 */
  } finally {
    try {
      localStorage.removeItem(QUEUE_KEY);
    } catch {
      /* ignore */
    }
  }
}

function persistBacklog() {
  try {
    if (queue.length > 0) {
      localStorage.setItem(QUEUE_KEY, JSON.stringify(queue.slice(0, MAX_QUEUE)));
    } else {
      localStorage.removeItem(QUEUE_KEY);
    }
  } catch {
    /* 存储不可用时静默丢弃 */
  }
}

function enqueue(payload: ReportPayload) {
  if (sessionCount >= MAX_SESSION) return;
  const fp = fingerprint(payload);
  // 队列内去重（服务端还会按 fingerprint 合并计数）
  if (queue.some((q) => fingerprint(q) === fp)) return;
  queue.push(payload);
  sessionCount += 1;
  if (queue.length >= BATCH_SIZE) {
    flush();
  }
}

async function postBatch(batch: ReportPayload[]): Promise<boolean> {
  const token = getToken();
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (token) headers["Authorization"] = `Bearer ${token}`;
  const results = await Promise.all(
    batch.map((p) =>
      fetch(REPORT_ENDPOINT, {
        method: "POST",
        headers,
        body: JSON.stringify(p),
        keepalive: true,
      })
        .then((r) => r.ok)
        .catch(() => false),
    ),
  );
  return results.every(Boolean);
}

export function flush() {
  if (queue.length === 0) return;
  const batch = queue.splice(0, BATCH_SIZE);
  postBatch(batch).then((ok) => {
    if (!ok) {
      // 失败（如离线）：合并回队列并写入待发 backlog，等待下次冲刷
      queue = [...batch, ...queue].slice(0, MAX_QUEUE);
      persistBacklog();
    } else if (queue.length === 0) {
      persistBacklog();
    }
  });
}

function scheduleFlush() {
  if (flushTimer !== null) return;
  flushTimer = window.setInterval(() => flush(), FLUSH_INTERVAL_MS);
}

function reportNormalized(n: NormalizedError, extra?: Partial<ReportPayload>) {
  enqueue({
    source: n.source,
    code: n.code,
    message: n.message,
    hint: n.hint,
    stack: n.stack,
    url: typeof window !== "undefined" ? window.location.href : "",
    route: n.route,
    http_status: n.status,
    ua: typeof navigator !== "undefined" ? navigator.userAgent : "",
    plugin_id: n.pluginId,
    ...extra,
  });
  scheduleFlush();
}

/** 显式上报一个异常（api.ts 5xx / ErrorBoundary 等调用）。 */
export function reportError(err: unknown, context?: { source?: "frontend" | "api" }) {
  reportNormalized(normalizeError(err, context));
}

/** 上报已归一化错误（可附带用户反馈等额外字段）。 */
export function reportNormalizedError(n: NormalizedError, extra?: Partial<ReportPayload>) {
  reportNormalized(n, extra);
}

function normalizeConsoleArgs(args: unknown[]): NormalizedError {
  const text = args
    .map((a) => {
      if (a instanceof Error) return a.stack || a.message;
      try {
        return typeof a === "string" ? a : JSON.stringify(a);
      } catch {
        return String(a);
      }
    })
    .join(" ");
  return {
    code: "UNKNOWN_ERROR",
    message: text.slice(0, 500) || "console.error",
    hint: "控制台输出错误，详见堆栈",
    route: typeof window !== "undefined" ? (window.location.hash || "/") : "/",
    source: "frontend",
  };
}

let lastConsoleReport = 0;
function captureConsole() {
  const originalError = console.error;
  console.error = (...args: unknown[]) => {
    originalError.apply(console, args);
    const now = Date.now();
    if (now - lastConsoleReport < 2_000) return; // 节流：2 秒内最多上报一次
    lastConsoleReport = now;
    reportNormalized(normalizeConsoleArgs(args));
  };
}

/** 初始化全局捕获（在应用入口调用一次）。 */
export function initErrorCapture() {
  if (initialized) return;
  initialized = true;
  restoreBacklog();
  scheduleFlush();

  // capture 阶段捕获所有错误事件（含资源加载失败）
  window.addEventListener(
    "error",
    (event) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "IMG" || target.tagName === "SCRIPT" || target.tagName === "LINK")) {
        const src = (target as HTMLImageElement).src || target.getAttribute("href") || "";
        reportNormalized({
          code: "NETWORK_ERROR",
          message: `资源加载失败: ${src.slice(0, 200)}`,
          hint: "请检查网络连接，或确认资源地址是否有效",
          route: window.location.hash || "/",
          source: "frontend",
        });
        return;
      }
      const err = event.error;
      if (err instanceof Error) {
        reportNormalized(normalizeError(err));
      } else {
        reportNormalized({
          code: "UNKNOWN_ERROR",
          message: event.message || "未捕获的脚本错误",
          hint: "发生未知脚本错误",
          route: window.location.hash || "/",
          source: "frontend",
        });
      }
    },
    true,
  );

  window.addEventListener("unhandledrejection", (event) => {
    reportNormalized(normalizeError(event.reason));
  });

  captureConsole();

  window.addEventListener("online", () => flush());
  window.addEventListener("beforeunload", () => {
    if (queue.length === 0) return;
    const token = getToken();
    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (token) headers["Authorization"] = `Bearer ${token}`;
    queue.splice(0).forEach((p) => {
      try {
        navigator.sendBeacon(REPORT_ENDPOINT, new Blob([JSON.stringify(p)], { type: "application/json" }));
      } catch {
        /* ignore */
      }
    });
    queue = [];
  });
}
