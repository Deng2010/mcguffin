// ============== 异常归一化 ==============
// 把任意异常（ApiError / 网络错误 / 未知异常 / 字符串）归一化为统一结构，
// 供 toast 展示与错误上报使用。

import { lookupHint } from "./registry";

export interface NormalizedError {
  code: string;
  message: string;
  hint: string;
  status?: number;
  stack?: string;
  route: string;
  pluginId?: string;
  source: "frontend" | "api";
}

export function getRoute(): string {
  try {
    return window.location.hash || window.location.pathname || "/";
  } catch {
    return "/";
  }
}

export function detectPluginId(route: string): string | undefined {
  const m = route.match(/\/plugins\/([^/]+)/);
  return m ? m[1] : undefined;
}

/** ApiError 形态判断（鸭子类型，避免与 services/api 形成循环依赖）。 */
export function isApiErrorLike(err: unknown): err is {
  status: number;
  message: string;
  code?: string;
  hint?: string;
  stack?: string;
} {
  return (
    err instanceof Error &&
    typeof (err as { status?: unknown }).status === "number"
  );
}

/** 判断是否为 fetch 网络层失败（后端不可达等）。 */
export function isNetworkError(err: unknown): boolean {
  return (
    err instanceof TypeError &&
    /fetch|network|load failed|failed to fetch/i.test(err.message)
  );
}

export function normalizeError(
  err: unknown,
  context?: { route?: string; source?: "frontend" | "api" },
): NormalizedError {
  const route = context?.route ?? getRoute();
  const pluginId = detectPluginId(route);
  const source = context?.source ?? "frontend";

  if (isApiErrorLike(err)) {
    const code = err.code ?? (err.status >= 500 ? "INTERNAL_ERROR" : "UNKNOWN_ERROR");
    return {
      code,
      message: err.message || "请求失败",
      hint: err.hint ?? lookupHint(code),
      status: err.status,
      stack: err.stack,
      route,
      pluginId,
      source,
    };
  }

  if (isNetworkError(err)) {
    const e = err as TypeError;
    return {
      code: "NETWORK_ERROR",
      message: "网络请求失败",
      hint: lookupHint("NETWORK_ERROR"),
      stack: e.stack,
      route,
      pluginId,
      source,
    };
  }

  if (err instanceof Error) {
    return {
      code: "UNKNOWN_ERROR",
      message: err.message || "未知错误",
      hint: lookupHint("UNKNOWN_ERROR"),
      stack: err.stack,
      route,
      pluginId,
      source,
    };
  }

  if (typeof err === "string" && err.trim()) {
    return {
      code: "UNKNOWN_ERROR",
      message: err,
      hint: lookupHint("UNKNOWN_ERROR"),
      route,
      pluginId,
      source,
    };
  }

  return {
    code: "UNKNOWN_ERROR",
    message: "未知错误",
    hint: lookupHint("UNKNOWN_ERROR"),
    route,
    pluginId,
    source,
  };
}

/** 取异常中最适合展示给用户的信息（供替换 alert 使用）。 */
export function errorMessage(err: unknown, fallback = "操作失败"): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return fallback;
}
