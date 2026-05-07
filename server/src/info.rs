use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::state::AppState;
use crate::types::{SiteInfo, UpdateSiteDescriptionPayload};
use crate::utils::get_token_from_headers;

/// GET /api/site/info
/// Returns site info (no auth required)
pub async fn get_site_info(
    State(state): State<AppState>,
) -> Json<SiteInfo> {
    let difficulty_order = state.difficulty_order.read().await.clone();
    Json(SiteInfo {
        name: state.site_name.clone(),
        version: state.site_version.clone(),
        description: state.site_description.read().await.clone(),
        title: state.site_title.clone(),
        difficulty_order,
        showcase_problem_ids: state.showcase_problem_ids.read().await.clone(),
        showcase_contest_ids: state.showcase_contest_ids.read().await.clone(),
    })
}

/// PUT /api/site/description
/// Admin/superadmin only — updates team description
pub async fn update_site_description(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateSiteDescriptionPayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check admin auth
    let token = get_token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let user_id = state.sessions.read().await.get(&token).map(|e| e.user_id.clone()).ok_or(StatusCode::UNAUTHORIZED)?;
    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or(StatusCode::UNAUTHORIZED)?;
    if user.role != "admin" && user.role != "superadmin" {
        return Err(StatusCode::FORBIDDEN);
    }
    drop(users);

    *state.site_description.write().await = payload.description.clone();
    state.save().await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "团队简介已更新",
        "description": payload.description,
    })))
}

// ============== Difficulty Config (public read) ==============

/// GET /api/site/difficulties
/// Returns the custom difficulty levels (no auth required)
/// Ordered by the configured difficulty_order
pub async fn get_difficulties(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let diff = state.difficulty.read().await;
    let order = state.difficulty_order.read().await.clone();
    let mut levels: Vec<serde_json::Value> = order.iter().filter_map(|name| {
        diff.levels.get(name).map(|level| {
            serde_json::json!({
                "name": name,
                "label": level.label,
                "color": level.color,
            })
        })
    }).collect();
    // Also add any levels not in the order (shouldn't happen, but be safe)
    for (name, level) in &diff.levels {
        if !order.contains(name) {
            levels.push(serde_json::json!({
                "name": name,
                "label": level.label,
                "color": level.color,
            }));
        }
    }
    drop(diff);
    Json(serde_json::json!({ "levels": levels }))
}
