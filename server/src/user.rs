use axum::{
    extract::State,
    Json,
};
use axum::http::HeaderMap;

use crate::state::AppState;
use crate::types::*;
use crate::utils::get_token_from_headers;

// ============== Get Current User ==============

pub async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Option<User>> {
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(user_id) = sessions.get(&token) {
            let users = state.users.read().await;
            return Json(users.get(user_id).cloned());
        }
    }
    Json(None)
}

// ============== Update Profile ==============

/// PUT /api/user/profile
/// Update display_name, avatar_url, bio for the current user
pub async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProfilePayload>,
) -> Json<serde_json::Value> {
    let token = match get_token_from_headers(&headers) {
        Some(t) => t,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };

    let user_id = {
        let sessions = state.sessions.read().await;
        match sessions.get(&token) {
            Some(uid) => uid.clone(),
            None => return Json(serde_json::json!({"success": false, "message": "无效的会话"})),
        }
    };

    let mut users = state.users.write().await;
    let user = match users.get_mut(&user_id) {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "用户不存在"})),
    };

    // Apply changes
    if let Some(name) = payload.display_name {
        if !name.trim().is_empty() {
            user.display_name = name.trim().to_string();
        }
    }
    if let Some(url) = payload.avatar_url {
        user.avatar_url = if url.trim().is_empty() {
            None
        } else {
            Some(url.trim().to_string())
        };
    }
    if let Some(bio) = payload.bio {
        user.bio = bio;
    }

    // Also sync to TeamMember record so member list stays consistent
    {
        let mut members = state.team_members.write().await;
        for member in members.values_mut() {
            if member.user_id == user_id {
                member.name = user.display_name.clone();
                member.avatar = user.display_name.chars().next()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "U".to_string());
                member.avatar_url = user.avatar_url.clone();
            }
        }
    }

    let updated_user = user.clone();
    drop(users);
    state.save().await;

    Json(serde_json::json!({
        "success": true,
        "message": "个人资料已更新",
        "user": updated_user,
    }))
}

// ============== Verify Token ==============

pub async fn verify_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<VerifyResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(user_id) = sessions.get(&token) {
            return Json(VerifyResponse { valid: true, user_id: user_id.clone() });
        }
    }
    Json(VerifyResponse { valid: false, user_id: String::new() })
}

// ============== Logout ==============

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<LogoutResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        state.sessions.write().await.remove(&token);
        state.save().await;
        Json(LogoutResponse { success: true })
    } else {
        Json(LogoutResponse { success: false })
    }
}
