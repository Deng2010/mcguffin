import { describe, it, expect } from "vitest";
import { errorRegistry, lookupHint, lookupSuggestion } from "../errors/registry";

// 后端 `server/src/error.rs` 的 ErrorCode 完整清单（手工同步，与 registry.ts 注释一致）。
// 该清单用于防止前端错误码注册表遗漏后端已定义、但前端尚未镜像的错误码。
const BACKEND_ERROR_CODES = [
  // 认证 / 会话
  "AUTH_UNAUTHORIZED",
  "AUTH_LOGIN_FAILED",
  "AUTH_TOKEN_INVALID",
  "AUTH_OAUTH_ERROR",
  // 权限
  "PERMISSION_DENIED",
  // 校验
  "VALIDATION_INVALID",
  "VALIDATION_NAME_TAKEN",
  // 用户
  "USER_NOT_FOUND",
  "USER_INVALID_SESSION",
  // 团队
  "TEAM_APPLICATION_EXISTS",
  "TEAM_APPLICATION_INVALID",
  // 题目
  "PROBLEM_NOT_FOUND",
  "PROBLEM_FORBIDDEN",
  "PROBLEM_INVALID_STATE",
  "PROBLEM_CONTEST_INVALID",
  // 赛事
  "CONTEST_NOT_FOUND",
  "CONTEST_INVALID_STATUS",
  "CONTEST_INVALID_LINK",
  // 帖子
  "POST_NOT_FOUND",
  "POST_FORBIDDEN",
  "POST_INVALID_CONTENT",
  "POST_REPLY_INVALID",
  // 插件
  "PLUGIN_NOT_FOUND",
  "PLUGIN_DISABLED",
  "PLUGIN_PERMISSION_DENIED",
  "PLUGIN_ALREADY_REGISTERED",
  "PLUGIN_DATA_INVALID",
  // 通知
  "NOTIFICATION_NOT_FOUND",
  // 站点 / 配置
  "SITE_CONFIG_INVALID",
  "SITE_DESCRIPTION_INVALID",
  // 备份 / 管理
  "BACKUP_FAILED",
  "BACKUP_RESTORE_FAILED",
  "BACKUP_INTEGRITY_FAILED",
  "EXPORT_FAILED",
  "ADMIN_USER_PROTECTED",
  "DATABASE_ERROR",
  // 全局
  "NOT_FOUND",
  "METHOD_NOT_ALLOWED",
  "INTERNAL_ERROR",
  "NETWORK_ERROR",
  "RATE_LIMITED",
  "UNKNOWN_ERROR",
] as const;

describe("errorRegistry 与后端错误码一致", () => {
  it("前端注册表覆盖了后端全部错误码", () => {
    for (const code of BACKEND_ERROR_CODES) {
      expect(errorRegistry[code], `前端缺少错误码 ${code} 的 hint/suggestion`).toBeDefined();
      expect(errorRegistry[code].hint).toBeTruthy();
      expect(errorRegistry[code].suggestion).toBeTruthy();
    }
  });

  it("每个错误码都有非空 hint 与 suggestion", () => {
    for (const [code, meta] of Object.entries(errorRegistry)) {
      expect(meta.hint, `${code}.hint 不能为空`).toBeTruthy();
      expect(meta.suggestion, `${code}.suggestion 不能为空`).toBeTruthy();
    }
  });
});

describe("lookupHint / lookupSuggestion", () => {
  it("已知错误码返回对应 hint", () => {
    expect(lookupHint("PROBLEM_NOT_FOUND")).toContain("题目 ID");
  });

  it("未知错误码回退到默认提示", () => {
    expect(lookupHint("SOME_FUTURE_CODE")).toBe("请稍后重试");
    expect(lookupSuggestion("SOME_FUTURE_CODE")).toContain("前后端日志");
  });

  it("空 / undefined 错误码回退到默认提示", () => {
    expect(lookupHint()).toBe("请稍后重试");
    expect(lookupHint("")).toBe("请稍后重试");
  });
});
