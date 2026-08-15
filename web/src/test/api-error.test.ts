import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  apiFetch,
  ApiError,
  parseErrorBody,
  getToken,
  setToken,
} from "../services/api";
import { reportError } from "../errors/reporter";

vi.mock("../errors/reporter", () => ({
  reportError: vi.fn(),
  initErrorCapture: vi.fn(),
  reportNormalizedError: vi.fn(),
}));

function mockFetch(status: number, body: unknown) {
  return vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(body),
    json: async () => body,
  });
}

describe("apiFetch 统一错误处理", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    window.location.hash = "";
  });

  it("解析统一错误体并抛出 ApiError，5xx 自动上报", async () => {
    vi.stubGlobal(
      "fetch",
      mockFetch(500, {
        success: false,
        code: "DATABASE_ERROR",
        message: "数据库错误",
        hint: "请稍后重试",
        request_id: "rid-1",
      }),
    );
    await expect(apiFetch("/x")).rejects.toMatchObject({
      status: 500,
      code: "DATABASE_ERROR",
      hint: "请稍后重试",
      requestId: "rid-1",
    });
    expect(reportError).toHaveBeenCalledTimes(1);
  });

  it("401 AUTH_UNAUTHORIZED 触发会话过期（清 token + 跳登录）", async () => {
    setToken("t");
    vi.stubGlobal(
      "fetch",
      mockFetch(401, {
        success: false,
        code: "AUTH_UNAUTHORIZED",
        message: "未登录或会话已过期",
      }),
    );
    await expect(apiFetch("/user/me")).rejects.toBeInstanceOf(ApiError);
    expect(getToken()).toBeNull();
    expect(window.location.hash).toContain("/login");
  });

  it("AUTH_LOGIN_FAILED 不触发会话过期", async () => {
    setToken("t");
    vi.stubGlobal(
      "fetch",
      mockFetch(401, {
        success: false,
        code: "AUTH_LOGIN_FAILED",
        message: "用户名或密码错误",
      }),
    );
    await expect(apiFetch("/auth/login")).rejects.toBeInstanceOf(ApiError);
    expect(getToken()).toBe("t");
    expect(window.location.hash).not.toContain("/login");
  });

  it("网络失败 → ApiError(NETWORK_ERROR) 并上报", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));
    await expect(apiFetch("/x")).rejects.toMatchObject({
      status: 0,
      code: "NETWORK_ERROR",
    });
    expect(reportError).toHaveBeenCalled();
  });

  it("parseErrorBody 正常解析与容错", () => {
    expect(
      parseErrorBody(
        '{"success":false,"code":"X","message":"m","hint":"h","request_id":"r"}',
      ),
    ).toEqual({ message: "m", code: "X", hint: "h", requestId: "r" });
    expect(parseErrorBody("not json")).toEqual({});
  });
});
