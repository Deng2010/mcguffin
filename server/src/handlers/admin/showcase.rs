use axum::{extract::State, http::StatusCode, Json};

use crate::state::AppState;
use crate::types::{ShowcaseConfigPayload, ShowcaseLayoutPayload};
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

// ============== Showcase Component Layout ==============

/// GET /api/admin/showcase/layout
/// edit_showcase permission required — returns the component layout (opaque JSON)
pub async fn get_showcase_layout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::EDIT_SHOWCASE)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "layout": state.showcase_layout.read().await.clone(),
    })))
}

/// PUT /api/admin/showcase/layout
/// edit_showcase permission required — persists the component layout.
/// 布局结构（schema_version / components[].settings / size / position）由前端
/// 定义并校验；后端将其视为 opaque JSON 做纯持久化，以便未来新增组件类型时
/// 无需改动后端。
pub async fn update_showcase_layout(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<ShowcaseLayoutPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::EDIT_SHOWCASE)
        .await?;

    if !payload.layout.is_object() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": "layout 必须是 JSON 对象（{ schema_version, components }）",
            })),
        ));
    }

    *state.showcase_layout.write().await = Some(payload.layout.clone());

    // 同步写入 SQLite meta 表
    let layout_json = serde_json::to_string(&payload.layout).unwrap_or_default();
    let _ = sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_layout', ?)")
        .bind(&layout_json)
        .execute(&state.db)
        .await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "展板布局已保存"}),
    ))
}
