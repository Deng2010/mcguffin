//! 统一错误码与错误响应助手
//!
//! - `ErrorCode` 枚举集中注册所有错误码（编译期保证无拼写错误）
//! - `json_error` / `http_error` 生成统一的错误响应格式
//! - 全局中间件为错误响应注入 `request_id` 并统一打日志
//! - 路由兜底（404 / 405）与 panic 兜底（500）返回统一 JSON

use axum::body::{to_bytes, Body, Bytes};
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// 统一错误码注册表：代码 → HTTP 状态、默认 message、用户提示 hint、开发者建议 suggestion。
/// 模块级粒度（约 40 个），handler 可覆盖 message，hint/suggestion 优先取注册表默认值。
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // 认证 / 会话
    AUTH_UNAUTHORIZED,
    AUTH_LOGIN_FAILED,
    AUTH_TOKEN_INVALID,
    AUTH_OAUTH_ERROR,
    // 权限
    PERMISSION_DENIED,
    // 校验
    VALIDATION_INVALID,
    VALIDATION_NAME_TAKEN,
    // 用户
    USER_NOT_FOUND,
    USER_INVALID_SESSION,
    // 团队
    TEAM_APPLICATION_EXISTS,
    TEAM_APPLICATION_INVALID,
    // 题目
    PROBLEM_NOT_FOUND,
    PROBLEM_FORBIDDEN,
    PROBLEM_INVALID_STATE,
    PROBLEM_CONTEST_INVALID,
    // 赛事
    CONTEST_NOT_FOUND,
    CONTEST_INVALID_STATUS,
    CONTEST_INVALID_LINK,
    // 帖子
    POST_NOT_FOUND,
    POST_FORBIDDEN,
    POST_INVALID_CONTENT,
    POST_REPLY_INVALID,
    // 插件
    PLUGIN_NOT_FOUND,
    PLUGIN_DISABLED,
    PLUGIN_PERMISSION_DENIED,
    PLUGIN_ALREADY_REGISTERED,
    PLUGIN_DATA_INVALID,
    // 通知
    NOTIFICATION_NOT_FOUND,
    // 站点 / 配置
    SITE_CONFIG_INVALID,
    SITE_DESCRIPTION_INVALID,
    // 备份 / 管理
    BACKUP_FAILED,
    BACKUP_RESTORE_FAILED,
    BACKUP_INTEGRITY_FAILED,
    EXPORT_FAILED,
    ADMIN_USER_PROTECTED,
    DATABASE_ERROR,
    // 全局
    NOT_FOUND,
    METHOD_NOT_ALLOWED,
    INTERNAL_ERROR,
    NETWORK_ERROR,
    RATE_LIMITED,
    UNKNOWN_ERROR,
}

impl ErrorCode {
    /// 稳定的机器可读错误码（写入日志 / 数据库 / 前端展示）。
    pub fn code(self) -> &'static str {
        match self {
            ErrorCode::AUTH_UNAUTHORIZED => "AUTH_UNAUTHORIZED",
            ErrorCode::AUTH_LOGIN_FAILED => "AUTH_LOGIN_FAILED",
            ErrorCode::AUTH_TOKEN_INVALID => "AUTH_TOKEN_INVALID",
            ErrorCode::AUTH_OAUTH_ERROR => "AUTH_OAUTH_ERROR",
            ErrorCode::PERMISSION_DENIED => "PERMISSION_DENIED",
            ErrorCode::VALIDATION_INVALID => "VALIDATION_INVALID",
            ErrorCode::VALIDATION_NAME_TAKEN => "VALIDATION_NAME_TAKEN",
            ErrorCode::USER_NOT_FOUND => "USER_NOT_FOUND",
            ErrorCode::USER_INVALID_SESSION => "USER_INVALID_SESSION",
            ErrorCode::TEAM_APPLICATION_EXISTS => "TEAM_APPLICATION_EXISTS",
            ErrorCode::TEAM_APPLICATION_INVALID => "TEAM_APPLICATION_INVALID",
            ErrorCode::PROBLEM_NOT_FOUND => "PROBLEM_NOT_FOUND",
            ErrorCode::PROBLEM_FORBIDDEN => "PROBLEM_FORBIDDEN",
            ErrorCode::PROBLEM_INVALID_STATE => "PROBLEM_INVALID_STATE",
            ErrorCode::PROBLEM_CONTEST_INVALID => "PROBLEM_CONTEST_INVALID",
            ErrorCode::CONTEST_NOT_FOUND => "CONTEST_NOT_FOUND",
            ErrorCode::CONTEST_INVALID_STATUS => "CONTEST_INVALID_STATUS",
            ErrorCode::CONTEST_INVALID_LINK => "CONTEST_INVALID_LINK",
            ErrorCode::POST_NOT_FOUND => "POST_NOT_FOUND",
            ErrorCode::POST_FORBIDDEN => "POST_FORBIDDEN",
            ErrorCode::POST_INVALID_CONTENT => "POST_INVALID_CONTENT",
            ErrorCode::POST_REPLY_INVALID => "POST_REPLY_INVALID",
            ErrorCode::PLUGIN_NOT_FOUND => "PLUGIN_NOT_FOUND",
            ErrorCode::PLUGIN_DISABLED => "PLUGIN_DISABLED",
            ErrorCode::PLUGIN_PERMISSION_DENIED => "PLUGIN_PERMISSION_DENIED",
            ErrorCode::PLUGIN_ALREADY_REGISTERED => "PLUGIN_ALREADY_REGISTERED",
            ErrorCode::PLUGIN_DATA_INVALID => "PLUGIN_DATA_INVALID",
            ErrorCode::NOTIFICATION_NOT_FOUND => "NOTIFICATION_NOT_FOUND",
            ErrorCode::SITE_CONFIG_INVALID => "SITE_CONFIG_INVALID",
            ErrorCode::SITE_DESCRIPTION_INVALID => "SITE_DESCRIPTION_INVALID",
            ErrorCode::BACKUP_FAILED => "BACKUP_FAILED",
            ErrorCode::BACKUP_RESTORE_FAILED => "BACKUP_RESTORE_FAILED",
            ErrorCode::BACKUP_INTEGRITY_FAILED => "BACKUP_INTEGRITY_FAILED",
            ErrorCode::EXPORT_FAILED => "EXPORT_FAILED",
            ErrorCode::ADMIN_USER_PROTECTED => "ADMIN_USER_PROTECTED",
            ErrorCode::DATABASE_ERROR => "DATABASE_ERROR",
            ErrorCode::NOT_FOUND => "NOT_FOUND",
            ErrorCode::METHOD_NOT_ALLOWED => "METHOD_NOT_ALLOWED",
            ErrorCode::INTERNAL_ERROR => "INTERNAL_ERROR",
            ErrorCode::NETWORK_ERROR => "NETWORK_ERROR",
            ErrorCode::RATE_LIMITED => "RATE_LIMITED",
            ErrorCode::UNKNOWN_ERROR => "UNKNOWN_ERROR",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            ErrorCode::AUTH_UNAUTHORIZED
            | ErrorCode::AUTH_LOGIN_FAILED
            | ErrorCode::AUTH_TOKEN_INVALID
            | ErrorCode::USER_INVALID_SESSION => StatusCode::UNAUTHORIZED,
            ErrorCode::PERMISSION_DENIED
            | ErrorCode::PROBLEM_FORBIDDEN
            | ErrorCode::POST_FORBIDDEN
            | ErrorCode::PLUGIN_DISABLED
            | ErrorCode::PLUGIN_PERMISSION_DENIED => StatusCode::FORBIDDEN,
            ErrorCode::VALIDATION_NAME_TAKEN
            | ErrorCode::TEAM_APPLICATION_EXISTS
            | ErrorCode::PLUGIN_ALREADY_REGISTERED => StatusCode::CONFLICT,
            ErrorCode::VALIDATION_INVALID
            | ErrorCode::TEAM_APPLICATION_INVALID
            | ErrorCode::PROBLEM_INVALID_STATE
            | ErrorCode::PROBLEM_CONTEST_INVALID
            | ErrorCode::CONTEST_INVALID_STATUS
            | ErrorCode::CONTEST_INVALID_LINK
            | ErrorCode::POST_INVALID_CONTENT
            | ErrorCode::POST_REPLY_INVALID
            | ErrorCode::PLUGIN_DATA_INVALID
            | ErrorCode::SITE_CONFIG_INVALID
            | ErrorCode::SITE_DESCRIPTION_INVALID
            | ErrorCode::ADMIN_USER_PROTECTED => StatusCode::BAD_REQUEST,
            ErrorCode::USER_NOT_FOUND
            | ErrorCode::PROBLEM_NOT_FOUND
            | ErrorCode::CONTEST_NOT_FOUND
            | ErrorCode::POST_NOT_FOUND
            | ErrorCode::PLUGIN_NOT_FOUND
            | ErrorCode::NOTIFICATION_NOT_FOUND
            | ErrorCode::NOT_FOUND => StatusCode::NOT_FOUND,
            ErrorCode::METHOD_NOT_ALLOWED => StatusCode::METHOD_NOT_ALLOWED,
            ErrorCode::RATE_LIMITED => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::NETWORK_ERROR => StatusCode::BAD_REQUEST,
            ErrorCode::AUTH_OAUTH_ERROR
            | ErrorCode::BACKUP_FAILED
            | ErrorCode::BACKUP_RESTORE_FAILED
            | ErrorCode::BACKUP_INTEGRITY_FAILED
            | ErrorCode::EXPORT_FAILED
            | ErrorCode::DATABASE_ERROR
            | ErrorCode::INTERNAL_ERROR
            | ErrorCode::UNKNOWN_ERROR => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 默认中文 message（handler 通常覆盖为更具体的提示）。
    pub fn default_message(self) -> &'static str {
        match self {
            ErrorCode::AUTH_UNAUTHORIZED => "未登录或会话已过期",
            ErrorCode::AUTH_LOGIN_FAILED => "用户名或密码错误",
            ErrorCode::AUTH_TOKEN_INVALID => "无效的会话",
            ErrorCode::AUTH_OAUTH_ERROR => "OAuth 认证服务暂不可用",
            ErrorCode::PERMISSION_DENIED => "权限不足",
            ErrorCode::VALIDATION_INVALID => "输入不合法",
            ErrorCode::VALIDATION_NAME_TAKEN => "该名称已被使用",
            ErrorCode::USER_NOT_FOUND => "用户不存在",
            ErrorCode::USER_INVALID_SESSION => "无效的会话",
            ErrorCode::TEAM_APPLICATION_EXISTS => "已提交过入队申请",
            ErrorCode::TEAM_APPLICATION_INVALID => "申请信息不合法",
            ErrorCode::PROBLEM_NOT_FOUND => "题目不存在",
            ErrorCode::PROBLEM_FORBIDDEN => "无权限查看",
            ErrorCode::PROBLEM_INVALID_STATE => "题目状态不允许此操作",
            ErrorCode::PROBLEM_CONTEST_INVALID => "题目与赛事关系无效",
            ErrorCode::CONTEST_NOT_FOUND => "比赛不存在",
            ErrorCode::CONTEST_INVALID_STATUS => "状态值无效，仅支持 draft 或 public",
            ErrorCode::CONTEST_INVALID_LINK => "设为公开前请先设置比赛链接",
            ErrorCode::POST_NOT_FOUND => "帖子不存在",
            ErrorCode::POST_FORBIDDEN => "无权操作此帖子",
            ErrorCode::POST_INVALID_CONTENT => "帖子内容不合法",
            ErrorCode::POST_REPLY_INVALID => "回复内容不合法",
            ErrorCode::PLUGIN_NOT_FOUND => "插件不存在",
            ErrorCode::PLUGIN_DISABLED => "插件已被禁用，请联系管理员启用",
            ErrorCode::PLUGIN_PERMISSION_DENIED => "插件未申请权限",
            ErrorCode::PLUGIN_ALREADY_REGISTERED => "插件已注册",
            ErrorCode::PLUGIN_DATA_INVALID => "插件数据不合法",
            ErrorCode::NOTIFICATION_NOT_FOUND => "通知不存在或无权操作",
            ErrorCode::SITE_CONFIG_INVALID => "站点配置不合法",
            ErrorCode::SITE_DESCRIPTION_INVALID => "站点简介不合法",
            ErrorCode::BACKUP_FAILED => "备份失败",
            ErrorCode::BACKUP_RESTORE_FAILED => "恢复失败",
            ErrorCode::BACKUP_INTEGRITY_FAILED => "数据完整性检查失败",
            ErrorCode::EXPORT_FAILED => "导出失败",
            ErrorCode::ADMIN_USER_PROTECTED => "超级管理员不可被删除或降级",
            ErrorCode::DATABASE_ERROR => "数据库错误",
            ErrorCode::NOT_FOUND => "请求的资源不存在",
            ErrorCode::METHOD_NOT_ALLOWED => "请求方法不被允许",
            ErrorCode::INTERNAL_ERROR => "服务器内部错误",
            ErrorCode::NETWORK_ERROR => "网络请求失败",
            ErrorCode::RATE_LIMITED => "请求过于频繁",
            ErrorCode::UNKNOWN_ERROR => "未知错误",
        }
    }

    /// 面向用户的可操作建议（展示在 toast / 错误提示条）。
    pub fn hint(self) -> &'static str {
        match self {
            ErrorCode::AUTH_UNAUTHORIZED
            | ErrorCode::AUTH_TOKEN_INVALID
            | ErrorCode::USER_INVALID_SESSION => "请重新登录",
            ErrorCode::PERMISSION_DENIED => "请联系管理员申请相应权限",
            ErrorCode::VALIDATION_INVALID => "请检查必填字段、格式与长度限制",
            ErrorCode::VALIDATION_NAME_TAKEN => "请换一个名称",
            ErrorCode::USER_NOT_FOUND => "请检查用户 ID 是否正确",
            ErrorCode::TEAM_APPLICATION_EXISTS => "请等待管理员审核，或联系管理员处理",
            ErrorCode::PROBLEM_NOT_FOUND => "检查题目 ID 是否正确，或确认该题目的可见范围",
            ErrorCode::PROBLEM_FORBIDDEN => "该题目不可见或权限不足",
            ErrorCode::POST_INVALID_CONTENT => "请检查标题与正文是否为空或超长",
            ErrorCode::POST_REPLY_INVALID => "请检查回复内容是否为空或超长",
            ErrorCode::PLUGIN_DISABLED => "请联系管理员启用该插件",
            ErrorCode::PLUGIN_PERMISSION_DENIED => "插件缺少所需权限，请联系管理员",
            ErrorCode::PLUGIN_DATA_INVALID => "namespace 与 key 不能为空",
            ErrorCode::NOTIFICATION_NOT_FOUND => "该通知可能已被删除",
            ErrorCode::NOT_FOUND => "请检查地址是否正确",
            ErrorCode::INTERNAL_ERROR => "请稍后重试，如持续出现请联系管理员",
            ErrorCode::NETWORK_ERROR => "请检查网络连接，或确认后端服务是否已启动",
            ErrorCode::RATE_LIMITED => "请稍后再试",
            _ => "请检查输入后重试",
        }
    }

    /// 面向开发者的排查建议（展示在错误中心 / 服务端日志）。
    pub fn suggestion(self) -> &'static str {
        match self {
            ErrorCode::AUTH_UNAUTHORIZED => {
                "检查请求是否携带有效 Bearer token；会话 24 小时无操作会过期"
            }
            ErrorCode::AUTH_LOGIN_FAILED => "核对用户名/密码与管理员密码配置",
            ErrorCode::AUTH_OAUTH_ERROR => "检查 CP OAuth 服务可用性与 client_id/secret 配置",
            ErrorCode::PERMISSION_DENIED => "核对用户角色、个人权限与成员组权限配置",
            ErrorCode::BACKUP_FAILED => "查看服务端日志确认具体原因（磁盘空间、目录权限等）",
            ErrorCode::BACKUP_RESTORE_FAILED => "检查备份文件完整性与数据库文件权限",
            ErrorCode::BACKUP_INTEGRITY_FAILED => {
                "建议从最近一次安全备份恢复，并检查备份文件完整性"
            }
            ErrorCode::EXPORT_FAILED => "检查数据库/配置文件可读性与权限",
            ErrorCode::DATABASE_ERROR => {
                "检查 SQLite 文件路径与磁盘空间，查看服务端日志确认具体 SQL 错误"
            }
            ErrorCode::INTERNAL_ERROR | ErrorCode::UNKNOWN_ERROR => {
                "查看服务端日志（含 request_id）定位具体异常"
            }
            ErrorCode::NETWORK_ERROR => {
                "开发环境检查 :3000 后端是否运行；生产环境检查反向代理与后端健康状态"
            }
            ErrorCode::RATE_LIMITED => "检查是否有脚本或插件高频调用触发限流",
            _ => "结合接口文档与前后端日志核对本次请求上下文",
        }
    }
}

/// 生成统一错误 JSON 值（不含 request_id，由全局中间件注入）。
pub fn error_value(
    code: ErrorCode,
    message: impl Into<String>,
    hint_override: Option<String>,
) -> Value {
    json!({
        "success": false,
        "code": code.code(),
        "message": message.into(),
        "hint": hint_override.unwrap_or_else(|| code.hint().to_string()),
    })
}

/// 供返回 `Json<Value>` 的 handler 使用。
pub fn json_error(code: ErrorCode, message: impl Into<String>) -> Json<Value> {
    Json(error_value(code, message, None))
}

pub fn json_error_with_hint(
    code: ErrorCode,
    message: impl Into<String>,
    hint: impl Into<String>,
) -> Json<Value> {
    Json(error_value(code, message, Some(hint.into())))
}

/// 供返回 `Result<_, (StatusCode, Json<Value>)>` 的 handler 使用。
pub fn http_error(code: ErrorCode, message: impl Into<String>) -> (StatusCode, Json<Value>) {
    (code.status(), Json(error_value(code, message, None)))
}

pub fn http_error_with_hint(
    code: ErrorCode,
    message: impl Into<String>,
    hint: impl Into<String>,
) -> (StatusCode, Json<Value>) {
    (
        code.status(),
        Json(error_value(code, message, Some(hint.into()))),
    )
}

// ============== 路由兜底 ==============

/// 未匹配路由 → 统一 404 JSON。
pub async fn api_not_found() -> (StatusCode, Json<Value>) {
    http_error(ErrorCode::NOT_FOUND, ErrorCode::NOT_FOUND.default_message())
}

/// 已知路径但方法不允许 → 统一 405 JSON。
pub async fn api_method_not_allowed() -> (StatusCode, Json<Value>) {
    http_error(
        ErrorCode::METHOD_NOT_ALLOWED,
        ErrorCode::METHOD_NOT_ALLOWED.default_message(),
    )
}

/// panic 兜底 → 统一 500 JSON（供 `CatchPanicLayer::custom` 使用）。
pub fn panic_to_response(_err: Box<dyn std::any::Any + Send>) -> Response {
    http_error(
        ErrorCode::INTERNAL_ERROR,
        ErrorCode::INTERNAL_ERROR.default_message(),
    )
    .into_response()
}

// ============== 全局错误中间件 ==============

/// 为 4xx/5xx 的 JSON 错误响应注入 `request_id` 并统一打日志。
///
/// 放在 `RequestIdLayer` 内侧、`CatchPanicLayer` 外侧：
/// panic 兜底产生的 500 也能被本中间件补上 request_id 并记录。
pub async fn log_errors_and_inject_request_id(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let res = next.run(req).await;
    let status = res.status();
    if !status.is_client_error() && !status.is_server_error() {
        return res;
    }

    let is_json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);
    if !is_json {
        return res;
    }

    let (parts, body) = res.into_parts();
    let bytes = match to_bytes(body, 1_000_000).await {
        Ok(b) => b,
        Err(_) => return Response::from_parts(parts, Body::from(Bytes::new())),
    };

    let mut value: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };

    let mut code = String::new();
    let mut message = String::new();
    if let Some(obj) = value.as_object_mut() {
        if let Some(rid) = &request_id {
            obj.insert("request_id".to_string(), Value::String(rid.clone()));
        }
        code = obj
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or(status.as_str())
            .to_string();
        message = obj
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
    }

    if status.is_server_error() {
        tracing::error!(
            request_id = request_id.as_deref().unwrap_or(""),
            method = %method,
            path = %path,
            status = status.as_u16(),
            code = %code,
            message = %message,
            "API 请求失败（服务端错误）"
        );
    } else {
        tracing::warn!(
            request_id = request_id.as_deref().unwrap_or(""),
            method = %method,
            path = %path,
            status = status.as_u16(),
            code = %code,
            message = %message,
            "API 请求失败"
        );
    }

    let body = Body::from(serde_json::to_vec(&value).unwrap_or_else(|_| bytes.to_vec()));
    let mut res = Response::from_parts(parts, body);
    if let Ok(len) = serde_json::to_vec(&value).map(|v| HeaderValue::from(v.len())) {
        res.headers_mut().insert(header::CONTENT_LENGTH, len);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_metadata_consistent() {
        for code in [
            ErrorCode::AUTH_UNAUTHORIZED,
            ErrorCode::AUTH_LOGIN_FAILED,
            ErrorCode::AUTH_TOKEN_INVALID,
            ErrorCode::AUTH_OAUTH_ERROR,
            ErrorCode::PERMISSION_DENIED,
            ErrorCode::VALIDATION_INVALID,
            ErrorCode::VALIDATION_NAME_TAKEN,
            ErrorCode::USER_NOT_FOUND,
            ErrorCode::USER_INVALID_SESSION,
            ErrorCode::TEAM_APPLICATION_EXISTS,
            ErrorCode::TEAM_APPLICATION_INVALID,
            ErrorCode::PROBLEM_NOT_FOUND,
            ErrorCode::PROBLEM_FORBIDDEN,
            ErrorCode::PROBLEM_INVALID_STATE,
            ErrorCode::PROBLEM_CONTEST_INVALID,
            ErrorCode::CONTEST_NOT_FOUND,
            ErrorCode::CONTEST_INVALID_STATUS,
            ErrorCode::CONTEST_INVALID_LINK,
            ErrorCode::POST_NOT_FOUND,
            ErrorCode::POST_FORBIDDEN,
            ErrorCode::POST_INVALID_CONTENT,
            ErrorCode::POST_REPLY_INVALID,
            ErrorCode::PLUGIN_NOT_FOUND,
            ErrorCode::PLUGIN_DISABLED,
            ErrorCode::PLUGIN_PERMISSION_DENIED,
            ErrorCode::PLUGIN_ALREADY_REGISTERED,
            ErrorCode::PLUGIN_DATA_INVALID,
            ErrorCode::NOTIFICATION_NOT_FOUND,
            ErrorCode::SITE_CONFIG_INVALID,
            ErrorCode::SITE_DESCRIPTION_INVALID,
            ErrorCode::BACKUP_FAILED,
            ErrorCode::BACKUP_RESTORE_FAILED,
            ErrorCode::BACKUP_INTEGRITY_FAILED,
            ErrorCode::EXPORT_FAILED,
            ErrorCode::ADMIN_USER_PROTECTED,
            ErrorCode::DATABASE_ERROR,
            ErrorCode::NOT_FOUND,
            ErrorCode::METHOD_NOT_ALLOWED,
            ErrorCode::INTERNAL_ERROR,
            ErrorCode::NETWORK_ERROR,
            ErrorCode::RATE_LIMITED,
            ErrorCode::UNKNOWN_ERROR,
        ] {
            assert!(!code.code().is_empty(), "code() must not be empty");
            assert!(
                !code.default_message().is_empty(),
                "{}: default_message must not be empty",
                code.code()
            );
            assert!(
                !code.hint().is_empty(),
                "{}: hint must not be empty",
                code.code()
            );
            assert!(
                !code.suggestion().is_empty(),
                "{}: suggestion must not be empty",
                code.code()
            );
        }
    }

    #[test]
    fn json_error_shape() {
        let Json(v) = json_error(ErrorCode::PROBLEM_NOT_FOUND, "题目不存在");
        assert_eq!(v["success"], false);
        assert_eq!(v["code"], "PROBLEM_NOT_FOUND");
        assert_eq!(v["message"], "题目不存在");
        assert_eq!(v["hint"], ErrorCode::PROBLEM_NOT_FOUND.hint());
    }

    #[test]
    fn http_error_status_mapping() {
        let (status, _) = http_error(ErrorCode::AUTH_UNAUTHORIZED, "未登录");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = http_error(ErrorCode::PROBLEM_NOT_FOUND, "题目不存在");
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = http_error(ErrorCode::BACKUP_FAILED, "备份失败");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn hint_override_works() {
        let Json(v) = json_error_with_hint(ErrorCode::VALIDATION_INVALID, "x", "自定义提示");
        assert_eq!(v["hint"], "自定义提示");
    }
}
