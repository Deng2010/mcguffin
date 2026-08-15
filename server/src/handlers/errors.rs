//! 错误报告与错误中心 API
//!
//! - `POST /errors/report`：公开上报（免鉴权），按 IP 限流，可附加 user_id
//! - `GET /errors`：错误中心列表（`access_admin`），支持状态/错误码/来源过滤
//! - `PATCH /errors/{id}`：状态流转（open/investigating/resolved/ignored）
//! - `DELETE /errors/{id}` / `DELETE /errors`：删除单条 / 清空

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{http_error, ErrorCode};
use crate::state::AppState;
use crate::utils::{check_permission, resolve_user};

const RATE_LIMIT_MAX: usize = 20;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_RETAINED: i64 = 2000;

// ============== 上报 ==============

#[derive(Debug, Deserialize)]
pub struct ErrorReportPayload {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub suggestion: Option<String>,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub http_status: Option<i64>,
    #[serde(default)]
    pub ua: Option<String>,
    #[serde(default)]
    pub plugin_id: Option<String>,
}

/// 生成去重指纹：code + message + 堆栈前几行 + 路由。
fn fingerprint(code: &str, message: &str, stack: &str, route: &str) -> String {
    let stack_head: String = stack.lines().take(3).collect::<Vec<_>>().join("\n");
    format!(
        "{}|{}|{}|{}",
        code,
        message,
        stack_head.chars().take(200).collect::<String>(),
        route
    )
}

/// 校验管理员权限：返回 (user_id, user)。失败时返回带正确 HTTP 状态码的错误响应。
async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, crate::types::User), (StatusCode, Json<serde_json::Value>)> {
    let (uid, user) = resolve_user(state, headers).await.ok_or_else(|| {
        http_error(
            ErrorCode::AUTH_UNAUTHORIZED,
            ErrorCode::AUTH_UNAUTHORIZED.default_message(),
        )
    })?;
    if !check_permission(state, &user, crate::types::perms::ACCESS_ADMIN).await {
        return Err(http_error(
            ErrorCode::PERMISSION_DENIED,
            ErrorCode::PERMISSION_DENIED.default_message(),
        ));
    }
    Ok((uid, user))
}

pub async fn report_error(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ErrorReportPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // ── 按 IP 限流 ──
    let ip = addr.ip().to_string();
    {
        let mut limits = state.error_rate_limits.lock().await;
        let now = Instant::now();
        let window_start = now - RATE_LIMIT_WINDOW;
        let hits = limits.entry(ip.clone()).or_default();
        hits.retain(|t| *t > window_start);
        if hits.len() >= RATE_LIMIT_MAX {
            return Err(http_error(
                ErrorCode::RATE_LIMITED,
                ErrorCode::RATE_LIMITED.default_message(),
            ));
        }
        hits.push(now);
    }

    // ── 附加 user_id（若带有效 token）──
    let user_id = resolve_user(&state, &headers).await.map(|(uid, _)| uid);

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let code = payload
        .code
        .unwrap_or_else(|| ErrorCode::UNKNOWN_ERROR.code().to_string());
    let message = payload.message.unwrap_or_default();
    let hint = payload.hint.unwrap_or_default();
    let suggestion = payload.suggestion.unwrap_or_default();
    let stack = payload.stack.unwrap_or_default();
    let url = payload.url.unwrap_or_default();
    let route = payload.route.unwrap_or_default();
    let method = payload.method.unwrap_or_default();
    let http_status = payload.http_status;
    let ua = payload.ua.unwrap_or_default();
    let plugin_id = payload.plugin_id.unwrap_or_default();
    let source = payload.source.unwrap_or_else(|| "frontend".to_string());
    let fp = fingerprint(&code, &message, &stack, &route);

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO error_reports
            (id, ts, user_id, source, code, message, hint, suggestion, stack, url,
             route, method, http_status, ua, plugin_id, fingerprint, count, status,
             resolved_by, resolved_at, first_seen, last_seen)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 'open', NULL, NULL, ?, ?)
        ON CONFLICT(fingerprint) DO UPDATE SET
            count = count + 1,
            ts = excluded.ts,
            last_seen = excluded.last_seen,
            status = 'open'
        "#,
    )
    .bind(&id)
    .bind(&now_str)
    .bind(&user_id)
    .bind(&source)
    .bind(&code)
    .bind(&message)
    .bind(&hint)
    .bind(&suggestion)
    .bind(&stack)
    .bind(&url)
    .bind(&route)
    .bind(&method)
    .bind(http_status)
    .bind(&ua)
    .bind(&plugin_id)
    .bind(&fp)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&state.db)
    .await
    .map_err(|_| {
        http_error(
            ErrorCode::DATABASE_ERROR,
            ErrorCode::DATABASE_ERROR.default_message(),
        )
    })?;

    prune_old_reports(&state.db).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "错误已上报",
        "id": id,
    })))
}

/// 保留策略：最多 MAX_RETAINED 条，且不超过 90 天。
async fn prune_old_reports(db: &SqlitePool) {
    let _ = sqlx::query(
        "DELETE FROM error_reports WHERE id NOT IN \
         (SELECT id FROM error_reports ORDER BY last_seen DESC LIMIT ?)",
    )
    .bind(MAX_RETAINED)
    .execute(db)
    .await;
    let _ = sqlx::query(
        "DELETE FROM error_reports WHERE julianday(last_seen) < julianday('now', '-90 days')",
    )
    .execute(db)
    .await;
}

// ============== 错误中心（管理后台） ==============

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct ErrorReportRow {
    pub id: String,
    pub ts: String,
    pub user_id: Option<String>,
    pub source: String,
    pub code: String,
    pub message: String,
    pub hint: String,
    pub suggestion: String,
    pub stack: String,
    pub url: String,
    pub route: String,
    pub method: String,
    pub http_status: Option<i64>,
    pub ua: String,
    pub plugin_id: String,
    pub count: i64,
    pub status: String,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Deserialize)]
pub struct ErrorListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

pub async fn list_errors(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ErrorListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;

    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);

    let mut where_clauses: Vec<String> = Vec::new();
    if let Some(s) = &q.status {
        where_clauses.push(format!("status = '{}'", s.replace('\'', "''")));
    }
    if let Some(c) = &q.code {
        where_clauses.push(format!("code = '{}'", c.replace('\'', "''")));
    }
    if let Some(s) = &q.source {
        where_clauses.push(format!("source = '{}'", s.replace('\'', "''")));
    }
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };

    let rows: Vec<ErrorReportRow> = sqlx::query_as::<_, ErrorReportRow>(&format!(
        "SELECT id, ts, user_id, source, code, message, hint, suggestion, stack, url, \
         route, method, http_status, ua, plugin_id, count, status, resolved_by, resolved_at, \
         first_seen, last_seen FROM error_reports{} ORDER BY last_seen DESC LIMIT ? OFFSET ?",
        where_sql
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|_| {
        http_error(
            ErrorCode::DATABASE_ERROR,
            ErrorCode::DATABASE_ERROR.default_message(),
        )
    })?;

    let total: i64 =
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM error_reports{}", where_sql))
            .fetch_one(&state.db)
            .await
            .map_err(|_| {
                http_error(
                    ErrorCode::DATABASE_ERROR,
                    ErrorCode::DATABASE_ERROR.default_message(),
                )
            })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "errors": rows,
        "total": total,
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateErrorStatusPayload {
    pub status: String,
}

pub async fn update_error_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateErrorStatusPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (uid, _) = require_admin(&state, &headers).await?;

    let status = payload.status;
    if !matches!(
        status.as_str(),
        "open" | "investigating" | "resolved" | "ignored"
    ) {
        return Err(http_error(
            ErrorCode::VALIDATION_INVALID,
            "状态值无效，仅支持 open / investigating / resolved / ignored",
        ));
    }

    let now = Utc::now().to_rfc3339();
    let result = match status.as_str() {
        "open" | "investigating" => {
            sqlx::query(
                "UPDATE error_reports SET status = ?, resolved_by = NULL, resolved_at = NULL WHERE id = ?",
            )
            .bind(&status)
            .bind(&id)
            .execute(&state.db)
            .await
        }
        _ => {
            sqlx::query(
                "UPDATE error_reports SET status = ?, resolved_by = ?, resolved_at = ? WHERE id = ?",
            )
            .bind(&status)
            .bind(&uid)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
        }
    };
    let rows = result.map_err(|_| {
        http_error(
            ErrorCode::DATABASE_ERROR,
            ErrorCode::DATABASE_ERROR.default_message(),
        )
    })?;
    if rows.rows_affected() == 0 {
        return Err(http_error(ErrorCode::NOT_FOUND, "错误报告不存在"));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "状态已更新",
    })))
}

pub async fn delete_error(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;

    let result = sqlx::query("DELETE FROM error_reports WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|_| {
            http_error(
                ErrorCode::DATABASE_ERROR,
                ErrorCode::DATABASE_ERROR.default_message(),
            )
        })?;
    if result.rows_affected() == 0 {
        return Err(http_error(ErrorCode::NOT_FOUND, "错误报告不存在"));
    }
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "错误报告已删除",
    })))
}

pub async fn clear_errors(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;

    sqlx::query("DELETE FROM error_reports")
        .execute(&state.db)
        .await
        .map_err(|_| {
            http_error(
                ErrorCode::DATABASE_ERROR,
                ErrorCode::DATABASE_ERROR.default_message(),
            )
        })?;
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "错误报告已清空",
    })))
}

#[cfg(test)]
mod tests {
    use super::fingerprint;

    #[test]
    fn fingerprint_uses_code_message_and_route() {
        let a = fingerprint("CODE_A", "msg", "l1\nl2", "/x");
        let b = fingerprint("CODE_B", "msg", "l1\nl2", "/x");
        let c = fingerprint("CODE_A", "other", "l1\nl2", "/x");
        let d = fingerprint("CODE_A", "msg", "l1\nl2", "/y");
        assert_ne!(a, b, "code 变化应改变指纹");
        assert_ne!(a, c, "message 变化应改变指纹");
        assert_ne!(a, d, "route 变化应改变指纹");
    }

    #[test]
    fn fingerprint_identical_inputs_match() {
        assert_eq!(
            fingerprint("E", "boom", "l1\nl2\nl3", "/p"),
            fingerprint("E", "boom", "l1\nl2\nl3", "/p"),
        );
    }

    #[test]
    fn fingerprint_takes_only_first_three_stack_lines() {
        let short = fingerprint("E", "m", "l1\nl2\nl3", "/p");
        let long = fingerprint("E", "m", "l1\nl2\nl3\nl4\nl5", "/p");
        // 额外堆栈行（第 4 行起）不影响指纹
        assert_eq!(short, long);
        // 但前三行之一改变则影响指纹
        let diff = fingerprint("E", "m", "l1\nl2\nCHANGED", "/p");
        assert_ne!(short, diff);
    }

    #[test]
    fn fingerprint_truncates_stack_head_to_200_chars() {
        let big = "x".repeat(500);
        let fp = fingerprint("E", "m", &big, "/p");
        // 堆栈前三行拼接后截断到 200 字符，故完整 500 字符序列不应出现在指纹中
        assert!(!fp.contains(&big));
    }

    #[test]
    fn fingerprint_handles_empty_inputs() {
        let fp = fingerprint("", "", "", "");
        assert!(fp.starts_with("|||"));
    }
}
