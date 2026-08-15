// ============== Token Management ==============

import { normalizeError } from "../errors/normalize";
import { reportError } from "../errors/reporter";
import { toastError } from "../errors/ToastContext";

const TOKEN_KEY = "***";

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
}

// ============== API Error ==============

export class ApiError extends Error {
  status: number;
  responseText: string;
  code?: string;
  hint?: string;
  requestId?: string;

  constructor(
    status: number,
    responseText: string,
    message?: string,
    code?: string,
    hint?: string,
    requestId?: string,
  ) {
    super(message ?? `请求失败 (${status})`);
    this.status = status;
    this.responseText = responseText;
    this.code = code;
    this.hint = hint;
    this.requestId = requestId;
  }
}

/** 解析统一错误响应体（success/code/message/hint/request_id）。 */
export function parseErrorBody(
  text: string,
): { message?: string; code?: string; hint?: string; requestId?: string } {
  try {
    const j = JSON.parse(text);
    return {
      message: typeof j?.message === "string" ? j.message : undefined,
      code: typeof j?.code === "string" ? j.code : undefined,
      hint: typeof j?.hint === "string" ? j.hint : undefined,
      requestId: typeof j?.request_id === "string" ? j.request_id : undefined,
    };
  } catch {
    return {};
  }
}

// ============== 全局会话过期处理 ==============

let sessionExpiredNotified = false;

function handleSessionExpired() {
  clearToken();
  if (!sessionExpiredNotified) {
    sessionExpiredNotified = true;
    toastError("登录已过期，请重新登录");
    setTimeout(() => {
      sessionExpiredNotified = false;
    }, 5000);
  }
  try {
    if (!window.location.hash.includes("/login")) {
      window.location.hash = "#/login";
    }
  } catch {
    /* ignore */
  }
}

// ============== API Client ==============

export async function apiFetch<T>(
  path: string,
  options?: RequestInit,
): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...((options?.headers as Record<string, string>) || {}),
  };
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  let res: Response;
  try {
    res = await fetch(`/api${path}`, { ...options, headers });
  } catch (err) {
    // 网络层失败（后端不可达等）→ 归一化上报 + toast 提示
    const normalized = normalizeError(err, { source: "api" });
    reportError(err, { source: "api" });
    toastError(normalized.hint || normalized.message);
    throw new ApiError(0, "", normalized.message, normalized.code, normalized.hint);
  }
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    const parsed = parseErrorBody(text);
    const err = new ApiError(
      res.status,
      text,
      parsed.message ?? `请求失败 (${res.status})`,
      parsed.code,
      parsed.hint,
      parsed.requestId,
    );
    if (
      res.status === 401 &&
      (parsed.code === "AUTH_UNAUTHORIZED" ||
        parsed.code === "AUTH_TOKEN_INVALID" ||
        parsed.code === "USER_INVALID_SESSION" ||
        !parsed.code)
    ) {
      handleSessionExpired();
    } else if (res.status >= 500) {
      reportError(err, { source: "api" });
    }
    throw err;
  }
  return res.json();
}
