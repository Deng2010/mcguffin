use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::state::AppState;
use crate::types::ShowcaseConfigPayload;
use crate::utils::AuthUser;

// ============== Showcase Configuration ==============

/// GET /api/admin/showcase
/// edit_showcase permission required — returns current showcase selections
pub async fn get_showcase_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::EDIT_SHOWCASE)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "problem_ids": state.showcase_problem_ids.read().await.clone(),
        "contest_ids": state.showcase_contest_ids.read().await.clone(),
    })))
}

/// PUT /api/admin/showcase
/// edit_showcase permission required — updates which problems/contests appear on the showcase
pub async fn update_showcase_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<ShowcaseConfigPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::EDIT_SHOWCASE)
        .await?;

    let problem_ids_json = serde_json::to_string(&payload.problem_ids).unwrap_or_default();
    let contest_ids_json = serde_json::to_string(&payload.contest_ids).unwrap_or_default();
    *state.showcase_problem_ids.write().await = payload.problem_ids;
    *state.showcase_contest_ids.write().await = payload.contest_ids;

    // 同步写入 SQLite meta 表
    let _ =
        sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_problem_ids', ?)")
            .bind(&problem_ids_json)
            .execute(&state.db)
            .await;
    let _ =
        sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_contest_ids', ?)")
            .bind(&contest_ids_json)
            .execute(&state.db)
            .await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "展板配置已保存"}),
    ))
}
