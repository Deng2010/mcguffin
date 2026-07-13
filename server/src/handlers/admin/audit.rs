use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use sqlx::FromRow;

use crate::state::AppState;
use crate::utils::AuthUser;

// ============== Audit Log ==============

/// GET /api/admin/audit-log
/// view_stats permission required — returns recent permission audit entries
/// Row type for sqlx query_as when reading from audit_log table
#[derive(FromRow)]
struct AuditLogRow {
    timestamp: String,
    user_id: String,
    user_name: String,
    action: String,
    resource: String,
    result: String,
    reason: Option<String>,
}

pub async fn get_audit_log(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::VIEW_STATS)
        .await?;

    let rows = sqlx::query_as::<_, AuditLogRow>(
        "SELECT timestamp, user_id, user_name, action, resource, result, reason \
         FROM audit_log ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "数据库查询失败"})),
        )
    })?;

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "timestamp": r.timestamp,
                "user_id": r.user_id,
                "user_name": r.user_name,
                "action": r.action,
                "resource": r.resource,
                "result": r.result,
                "reason": r.reason,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(entries)))
}
