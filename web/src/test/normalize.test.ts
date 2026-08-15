import { describe, it, expect } from "vitest";
import { normalizeError, errorMessage } from "../errors/normalize";
import { ApiError } from "../services/api";

describe("normalizeError", () => {
  it("把 ApiError 归一化为统一结构", () => {
    const err = new ApiError(
      500,
      "",
      "数据库错误",
      "DATABASE_ERROR",
      "请稍后重试",
      "rid-1",
    );
    const n = normalizeError(err, { route: "#/admin/errors" });
    expect(n.code).toBe("DATABASE_ERROR");
    expect(n.message).toBe("数据库错误");
    expect(n.hint).toBe("请稍后重试");
    expect(n.status).toBe(500);
    expect(n.source).toBe("frontend");
  });

  it("网络层错误 → NETWORK_ERROR", () => {
    const n = normalizeError(new TypeError("Failed to fetch"), {
      route: "#/problems",
    });
    expect(n.code).toBe("NETWORK_ERROR");
    expect(n.hint).toContain("网络连接");
  });

  it("未知异常 → UNKNOWN_ERROR", () => {
    const n = normalizeError(new Error("boom"), { route: "#/" });
    expect(n.code).toBe("UNKNOWN_ERROR");
    expect(n.message).toBe("boom");
  });

  it("字符串异常直接作为消息", () => {
    const n = normalizeError("自定义错误", { route: "#/" });
    expect(n.message).toBe("自定义错误");
    expect(n.code).toBe("UNKNOWN_ERROR");
  });

  it("错误码无 hint 时回退到注册表", () => {
    const err = new ApiError(404, "", "题目不存在", "PROBLEM_NOT_FOUND");
    const n = normalizeError(err);
    expect(n.hint).toContain("题目 ID");
  });
});

describe("errorMessage", () => {
  it("返回 Error 的 message", () => {
    expect(errorMessage(new Error("失败了"))).toBe("失败了");
  });
  it("返回 fallback", () => {
    expect(errorMessage(null, "操作失败")).toBe("操作失败");
  });
});
