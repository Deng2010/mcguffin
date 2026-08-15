// ============== 错误码注册表（前端镜像） ==============
//
// 与后端 `server/src/error.rs` 的 ErrorCode 手工同步。
// 每个错误码附带：
//   - hint：面向用户的可操作建议（toast / 错误提示条）
//   - suggestion：面向开发者的排查建议（错误中心展示）

export type ErrorCode =
  // 认证 / 会话
  | "AUTH_UNAUTHORIZED"
  | "AUTH_LOGIN_FAILED"
  | "AUTH_TOKEN_INVALID"
  | "AUTH_OAUTH_ERROR"
  // 权限
  | "PERMISSION_DENIED"
  // 校验
  | "VALIDATION_INVALID"
  | "VALIDATION_NAME_TAKEN"
  // 用户
  | "USER_NOT_FOUND"
  | "USER_INVALID_SESSION"
  // 团队
  | "TEAM_APPLICATION_EXISTS"
  | "TEAM_APPLICATION_INVALID"
  // 题目
  | "PROBLEM_NOT_FOUND"
  | "PROBLEM_FORBIDDEN"
  | "PROBLEM_INVALID_STATE"
  | "PROBLEM_CONTEST_INVALID"
  // 赛事
  | "CONTEST_NOT_FOUND"
  | "CONTEST_INVALID_STATUS"
  | "CONTEST_INVALID_LINK"
  // 帖子
  | "POST_NOT_FOUND"
  | "POST_FORBIDDEN"
  | "POST_INVALID_CONTENT"
  | "POST_REPLY_INVALID"
  // 插件
  | "PLUGIN_NOT_FOUND"
  | "PLUGIN_DISABLED"
  | "PLUGIN_PERMISSION_DENIED"
  | "PLUGIN_ALREADY_REGISTERED"
  | "PLUGIN_DATA_INVALID"
  // 通知
  | "NOTIFICATION_NOT_FOUND"
  // 站点 / 配置
  | "SITE_CONFIG_INVALID"
  | "SITE_DESCRIPTION_INVALID"
  // 备份 / 管理
  | "BACKUP_FAILED"
  | "BACKUP_RESTORE_FAILED"
  | "BACKUP_INTEGRITY_FAILED"
  | "EXPORT_FAILED"
  | "ADMIN_USER_PROTECTED"
  | "DATABASE_ERROR"
  // 全局
  | "NOT_FOUND"
  | "METHOD_NOT_ALLOWED"
  | "INTERNAL_ERROR"
  | "NETWORK_ERROR"
  | "RATE_LIMITED"
  | "UNKNOWN_ERROR"
  | (string & {});

export interface ErrorMeta {
  hint: string;
  suggestion: string;
}

/** 客户端已知错误码的默认 hint/suggestion（与后端 error.rs 保持一致）。 */
export const errorRegistry: Record<string, ErrorMeta> = {
  AUTH_UNAUTHORIZED: {
    hint: "请重新登录",
    suggestion: "检查请求是否携带有效 Bearer token；会话 24 小时无操作会过期",
  },
  AUTH_LOGIN_FAILED: { hint: "请核对用户名或密码", suggestion: "核对用户名/密码与管理员密码配置" },
  AUTH_TOKEN_INVALID: { hint: "请重新登录", suggestion: "token 无效或已过期，重新登录获取新 token" },
  AUTH_OAUTH_ERROR: {
    hint: "认证服务暂不可用，请稍后再试",
    suggestion: "检查 CP OAuth 服务可用性与 client_id/secret 配置",
  },
  PERMISSION_DENIED: {
    hint: "请联系管理员申请相应权限",
    suggestion: "核对用户角色、个人权限与成员组权限配置",
  },
  VALIDATION_INVALID: {
    hint: "请检查必填字段、格式与长度限制",
    suggestion: "结合接口文档核对请求参数",
  },
  VALIDATION_NAME_TAKEN: { hint: "请换一个名称", suggestion: "名称唯一性冲突，改用其它名称" },
  USER_NOT_FOUND: { hint: "请检查用户 ID 是否正确", suggestion: "用户不存在或已被删除" },
  USER_INVALID_SESSION: { hint: "请重新登录", suggestion: "会话无效，重新登录" },
  TEAM_APPLICATION_EXISTS: {
    hint: "请等待管理员审核，或联系管理员处理",
    suggestion: "同一用户只能存在一条待审核申请",
  },
  TEAM_APPLICATION_INVALID: { hint: "请检查申请信息", suggestion: "申请信息不合法" },
  PROBLEM_NOT_FOUND: {
    hint: "检查题目 ID 是否正确，或确认该题目的可见范围",
    suggestion: "题目不存在或已被删除",
  },
  PROBLEM_FORBIDDEN: { hint: "该题目不可见或权限不足", suggestion: "核对题目 ACL 与用户权限" },
  PROBLEM_INVALID_STATE: { hint: "当前状态不允许此操作", suggestion: "核对题目状态流转条件" },
  PROBLEM_CONTEST_INVALID: { hint: "请检查题目与赛事关系", suggestion: "题目/赛事关系不匹配" },
  CONTEST_NOT_FOUND: { hint: "请检查赛事 ID", suggestion: "比赛不存在或已被删除" },
  CONTEST_INVALID_STATUS: { hint: "状态值无效", suggestion: "仅支持 draft 或 public" },
  CONTEST_INVALID_LINK: {
    hint: "请先设置比赛链接",
    suggestion: "设为公开前必须填写比赛链接",
  },
  POST_NOT_FOUND: { hint: "该帖子可能已被删除", suggestion: "帖子不存在或已被删除" },
  POST_FORBIDDEN: { hint: "无权操作此帖子", suggestion: "核对帖子可见性 ACL 与用户权限" },
  POST_INVALID_CONTENT: {
    hint: "请检查标题与正文是否为空或超长",
    suggestion: "标题/正文为空或超过长度限制",
  },
  POST_REPLY_INVALID: { hint: "请检查回复内容是否为空或超长", suggestion: "回复为空或超过长度限制" },
  PLUGIN_NOT_FOUND: { hint: "插件不存在或已卸载", suggestion: "插件未注册或已被卸载" },
  PLUGIN_DISABLED: { hint: "请联系管理员启用该插件", suggestion: "插件处于禁用状态" },
  PLUGIN_PERMISSION_DENIED: {
    hint: "插件缺少所需权限，请联系管理员",
    suggestion: "插件未申请或未获授权限",
  },
  PLUGIN_ALREADY_REGISTERED: { hint: "插件已注册", suggestion: "重复注册同一插件" },
  PLUGIN_DATA_INVALID: { hint: "namespace 与 key 不能为空", suggestion: "插件 KV 参数不合法" },
  NOTIFICATION_NOT_FOUND: { hint: "该通知可能已被删除", suggestion: "通知不存在或无权操作" },
  SITE_CONFIG_INVALID: { hint: "请检查站点配置", suggestion: "配置项缺失或格式不合法" },
  SITE_DESCRIPTION_INVALID: { hint: "请检查站点简介", suggestion: "站点简介不合法" },
  BACKUP_FAILED: { hint: "备份失败，请稍后重试", suggestion: "查看服务端日志确认具体原因（磁盘空间、目录权限等）" },
  BACKUP_RESTORE_FAILED: {
    hint: "恢复失败，请检查备份文件",
    suggestion: "检查备份文件完整性与数据库文件权限",
  },
  BACKUP_INTEGRITY_FAILED: {
    hint: "数据完整性检查失败",
    suggestion: "建议从最近一次安全备份恢复，并检查备份文件完整性",
  },
  EXPORT_FAILED: { hint: "导出失败，请稍后重试", suggestion: "检查数据库/配置文件可读性与权限" },
  ADMIN_USER_PROTECTED: { hint: "系统管理员不可被修改", suggestion: "超级管理员受保护，不可删除/降级" },
  DATABASE_ERROR: {
    hint: "服务器数据异常，请稍后重试",
    suggestion: "检查 SQLite 文件路径与磁盘空间，查看服务端日志确认具体 SQL 错误",
  },
  NOT_FOUND: { hint: "请检查地址是否正确", suggestion: "资源或路由不存在" },
  METHOD_NOT_ALLOWED: { hint: "请求方法不被允许", suggestion: "核对接口的 HTTP 方法" },
  INTERNAL_ERROR: {
    hint: "请稍后重试，如持续出现请联系管理员",
    suggestion: "查看服务端日志（含 request_id）定位具体异常",
  },
  NETWORK_ERROR: {
    hint: "请检查网络连接，或确认后端服务是否已启动",
    suggestion: "开发环境检查 :3000 后端是否运行；生产环境检查反向代理与后端健康状态",
  },
  RATE_LIMITED: { hint: "请求过于频繁，请稍后再试", suggestion: "检查是否有脚本或插件高频调用触发限流" },
  UNKNOWN_ERROR: {
    hint: "发生未知错误，请稍后重试",
    suggestion: "结合接口文档与前后端日志核对本次请求上下文",
  },
};

export function lookupHint(code?: string): string {
  if (code && errorRegistry[code]) return errorRegistry[code].hint;
  return "请稍后重试";
}

export function lookupSuggestion(code?: string): string {
  if (code && errorRegistry[code]) return errorRegistry[code].suggestion;
  return "结合接口文档与前后端日志核对本次请求上下文";
}
