use axum::{
    extract::{Path, State},
    Json,
};
use axum::http::HeaderMap;

use crate::state::AppState;
use crate::types::*;
use crate::utils::{get_token_from_headers, resolve_user};

// ============== Get Current User ==============

/// GET /api/user/me
/// Returns current authenticated user, respecting session expiry
pub async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Option<User>> {
    match resolve_user(&state, &headers).await {
        Some((_uid, user)) => Json(Some(user)),
        None => Json(None),
    }
}

// ============== Get Public Profile ==============

/// GET /api/user/profile/{username}
/// Returns public user info (no email, no password_hash)
pub async fn get_public_profile(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Json<serde_json::Value> {
    let users = state.users.read().await;
    let user = users.values().find(|u| u.username == username).cloned();
    drop(users);

    match user {
        Some(u) => {
            // Also check if user is a team member
            let members = state.team_members.read().await;
            let is_team_member = members.values().any(|m| m.user_id == u.id);
            drop(members);
            Json(serde_json::json!({
                "exists": true,
                "username": u.username,
                "display_name": u.display_name,
                "avatar_url": u.avatar_url,
                "bio": u.bio,
                "role": u.role,
                "is_team_member": is_team_member,
                "team_role": u.role,
                "created_at": u.created_at,
            }))
        }
        None => Json(serde_json::json!({
            "exists": false,
            "message": "用户不存在",
        })),
    }
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
            Some(entry) => entry.user_id.clone(),
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

    // Handle password change
    if let Some(password) = payload.password {
        if !password.trim().is_empty() {
            match bcrypt::hash(password.trim(), 10) {
                Ok(hash) => {
                    user.password_hash = Some(hash);
                }
                Err(e) => {
                    return Json(serde_json::json!({
                        "success": false,
                        "message": format!("密码加密失败: {}", e),
                    }));
                }
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
        if let Some(entry) = sessions.get(&token) {
            return Json(VerifyResponse { valid: true, user_id: entry.user_id.clone() });
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
