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
    Json(SiteInfo {
        name: state.site_name.clone(),
        version: state.site_version.clone(),
        description: state.site_description.read().await.clone(),
        title: state.site_title.clone(),
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
    let user_id = state.sessions.read().await.get(&token).cloned().ok_or(StatusCode::UNAUTHORIZED)?;
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
pub async fn get_difficulties(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let diff = state.difficulty.read().await;
    let levels: Vec<serde_json::Value> = diff.levels.iter().map(|(name, level)| {
        serde_json::json!({
            "name": name,
            "label": level.label,
            "color": level.color,
        })
    }).collect();
    drop(diff);
    Json(serde_json::json!({ "levels": levels }))
}
