use axum::http::HeaderMap;
use axum::{
    extract::{Path, State},
    Json,
};

use crate::state::AppState;
use crate::types::*;
use crate::utils::{get_token_from_headers, resolve_user};
use axum::extract::Query;
use std::collections::HashMap;

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

    // 显示名称不能为空
    if let Some(ref raw_name) = payload.display_name {
        let trimmed = raw_name.trim();
        if trimmed.is_empty() {
            return Json(serde_json::json!({
                "success": false,
                "message": "显示名称不能为空"
            }));
        }
    }

    // Pre-check display_name uniqueness before write lock
    let name_change = payload
        .display_name
        .as_ref()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    if let Some(ref name) = name_change {
        if name.chars().count() > 30 {
            return Json(serde_json::json!({
                "success": false,
                "message": "显示名称不能超过30个字符"
            }));
        }
        let current_display_name = {
            let users = state.users.read().await;
            match users.get(&user_id) {
                Some(u) => u.display_name.clone(),
                None => {
                    return Json(serde_json::json!({"success": false, "message": "用户不存在"}))
                }
            }
        };
        if *name != current_display_name {
            let is_taken = {
                let users = state.users.read().await;
                users
                    .values()
                    .any(|u| u.id != user_id && (u.display_name == *name || u.username == *name))
            };
            if is_taken {
                return Json(serde_json::json!({
                    "success": false,
                    "message": "该显示名称已被其他人使用"
                }));
            }
        }
    }

    let mut users = state.users.write().await;
    let user = match users.get_mut(&user_id) {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "用户不存在"})),
    };

    // Apply changes
    if let Some(name) = name_change {
        if !name.is_empty() {
            user.display_name = name;
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

// ============== Check Name Availability ==============

/// GET /api/user/check-name?name=xxx
/// Check if a display_name or username is already taken by another user
pub async fn check_name_available(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let name = match params.get("name") {
        Some(n) => n.trim().to_string(),
        None => return Json(serde_json::json!({"available": true})),
    };
    if name.is_empty() {
        return Json(serde_json::json!({"available": true}));
    }

    let token = match get_token_from_headers(&headers) {
        Some(t) => t,
        None => return Json(serde_json::json!({"available": false, "error": "未登录"})),
    };

    let user_id = {
        let sessions = state.sessions.read().await;
        match sessions.get(&token) {
            Some(entry) => entry.user_id.clone(),
            None => return Json(serde_json::json!({"available": false, "error": "无效的会话"})),
        }
    };

    let users = state.users.read().await;
    let taken = users
        .values()
        .any(|u| u.id != user_id && (u.display_name == name || u.username == name));

    Json(serde_json::json!({"available": !taken}))
}

// ============== Verify Token ==============

pub async fn verify_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<VerifyResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(entry) = sessions.get(&token) {
            return Json(VerifyResponse {
                valid: true,
                user_id: entry.user_id.clone(),
            });
        }
    }
    Json(VerifyResponse {
        valid: false,
        user_id: String::new(),
    })
}

// ============== Logout ==============

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Json<LogoutResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        state.sessions.write().await.remove(&token);
        state.save().await;
        Json(LogoutResponse { success: true })
    } else {
        Json(LogoutResponse { success: false })
    }
}
