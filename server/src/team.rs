use axum::{
    extract::{Path, State},
    Json,
};
use axum::http::HeaderMap;
use chrono::Utc;
use uuid::Uuid;

use crate::state::{AppState, ADMIN_USER_ID};
use crate::types::*;
use crate::utils::{get_token_from_headers, is_admin, is_superadmin, resolve_user};

// ============== List Team Members ==============

pub async fn get_team_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let is_superadmin_user = is_superadmin(&state, &user_id).await;
    let members = state.team_members.read().await;
    let users = state.users.read().await;
    let result: Vec<serde_json::Value> = members
        .values()
        .filter(|m| {
            // Superadmin visibility: derive role from users
            let member_role = users.get(&m.user_id).map(|u| u.role.as_str()).unwrap_or("");
            // Non-superadmin cannot see superadmin in member list
            if !is_superadmin_user && member_role == "superadmin" {
                return false;
            }
            is_admin_user || m.user_id != ADMIN_USER_ID
        })
        .map(|m| {
            let user_info = users.get(&m.user_id);
            let current_name = user_info.map(|u| u.display_name.clone())
                .unwrap_or_default();
            let current_avatar_url = user_info.and_then(|u| u.avatar_url.clone());
            let user_username = user_info.map(|u| u.username.clone())
                .unwrap_or_default();
            let user_role = user_info.map(|u| u.role.clone())
                .unwrap_or_default();
            let current_avatar = current_name.chars().next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "U".to_string());
            serde_json::json!({
                "id": m.id,
                "user_id": m.user_id,
                "name": current_name,
                "username": user_username,
                "avatar": current_avatar,
                "avatar_url": current_avatar_url,
                "role": user_role,
                "joined_at": m.joined_at,
            })
        })
        .collect();
    Json(serde_json::json!(result))
}

// ============== List Pending Join Requests ==============

pub async fn get_pending_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }
    let requests = state.join_requests.read().await;
    Json(serde_json::json!(requests.values().filter(|r| r.status == "pending").cloned().collect::<Vec<JoinRequest>>()))
}

// ============== Apply to Join ==============

pub async fn apply_to_join(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ApplyPayload>,
) -> Json<ApplyResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(user_id) = sessions.get(&token) {
            if user_id == ADMIN_USER_ID {
                return Json(ApplyResponse { success: false, message: "管理员无需申请".to_string() });
            }
            let users = state.users.read().await;
            if let Some(user) = users.get(user_id) {
                if user.team_status == "joined" {
                    return Json(ApplyResponse { success: false, message: "您已是团队成员".to_string() });
                }
                if user.team_status == "pending" {
                    return Json(ApplyResponse { success: false, message: "您已提交过申请".to_string() });
                }
                
                let request = JoinRequest {
                    id: Uuid::new_v4().to_string(),
                    user_id: user.id.clone(),
                    user_name: user.display_name.clone(),
                    user_email: user.email.clone().unwrap_or_default(),
                    reason: payload.reason,
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                };
                
                state.join_requests.write().await.insert(request.id.clone(), request);
                
                drop(users);
                if let Some(u) = state.users.write().await.get_mut(user_id) {
                    u.team_status = "pending".to_string();
                    u.role = "pending".to_string();
                }
                
                state.save().await;
                
                return Json(ApplyResponse { success: true, message: "申请已提交".to_string() });
            }
        }
    }
    Json(ApplyResponse { success: false, message: "未登录".to_string() })
}

// ============== Review Join Application ==============

pub async fn review_application(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((request_id, action)): Path<(String, String)>,
) -> Json<ReviewResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(user_id) = sessions.get(&token) {
            let users = state.users.read().await;
            if users.contains_key(user_id) {
                if !is_admin(&state, user_id).await {
                    return Json(ReviewResponse { success: false, message: "权限不足".to_string() });
                }
                
                let action_clone = action.clone();
                let (target_user_id, new_status) = {
                    let mut requests = state.join_requests.write().await;
                    if let Some(request) = requests.get_mut(&request_id) {
                        let uid = request.user_id.clone();
                        let new_status = if action_clone == "approve" {
                            request.status = "approved".to_string();
                            // Prevent duplicate: check if already a team member
                            let already_member = state.team_members.read().await
                                .values().any(|m| m.user_id == request.user_id);
                            if !already_member {
                                let member = TeamMember {
                                    id: Uuid::new_v4().to_string(),
                                    user_id: request.user_id.clone(),
                                    joined_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                };
                                state.team_members.write().await.insert(member.id.clone(), member);
                            }
                            "joined"
                        } else if action_clone == "reject" {
                            request.status = "rejected".to_string();
                            "none"
                        } else {
                            return Json(ReviewResponse { success: false, message: "无效操作".to_string() });
                        };
                        (uid, new_status.to_string())
                    } else {
                        return Json(ReviewResponse { success: false, message: "申请不存在".to_string() });
                    }
                };
                
                // Drop read locks before acquiring write lock on users
                drop(users);
                
                if let Some(u) = state.users.write().await.get_mut(&target_user_id) {
                    u.team_status = new_status.clone();
                    u.role = if new_status == "joined" { "member".to_string() } else { "guest".to_string() };
                }
                
                state.save().await;
                
                match action_clone.as_str() {
                    "approve" => Json(ReviewResponse { success: true, message: "已批准申请".to_string() }),
                    "reject" => Json(ReviewResponse { success: true, message: "已拒绝申请".to_string() }),
                    _ => Json(ReviewResponse { success: false, message: "无效操作".to_string() })
                }
            } else {
                Json(ReviewResponse { success: false, message: "用户不存在".to_string() })
            }
        } else {
            Json(ReviewResponse { success: false, message: "未登录".to_string() })
        }
    } else {
        Json(ReviewResponse { success: false, message: "未登录".to_string() })
    }
}

// ============== Change Member Role ==============

pub async fn change_member_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<ChangeRolePayload>,
) -> Json<ReviewResponse> {
    if user_id == ADMIN_USER_ID {
        return Json(ReviewResponse { success: false, message: "不能修改系统管理员的角色".to_string() });
    }
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(admin_user_id) = sessions.get(&token) {
            let users = state.users.read().await;
            if let Some(_admin) = users.get(admin_user_id) {
                if !is_admin(&state, admin_user_id).await {
                    return Json(ReviewResponse { success: false, message: "权限不足".to_string() });
                }
                if payload.role != "admin" && payload.role != "member" {
                    return Json(ReviewResponse { success: false, message: "无效角色".to_string() });
                }
                // Only superadmin can demote an existing admin or promote to admin
                let is_super = is_superadmin(&state, admin_user_id).await;
                let target_role = users.get(&user_id).map(|u| u.role.as_str());
                if let Some("admin") = target_role {
                    if !is_super {
                        return Json(ReviewResponse { success: false, message: "权限不足：仅限系统管理员操作".to_string() });
                    }
                }
                if payload.role == "admin" && !is_super {
                    return Json(ReviewResponse { success: false, message: "权限不足：仅限系统管理员操作".to_string() });
                }
                // Check if user is a team member
                let is_member = state.team_members.read().await
                    .values().any(|m| m.user_id == user_id);
                if !is_member {
                    return Json(ReviewResponse { success: false, message: "团队成员不存在".to_string() });
                }
                drop(users);
                if let Some(u) = state.users.write().await.get_mut(&user_id) {
                    u.role = payload.role.clone();
                }
                state.save().await;
                Json(ReviewResponse { success: true, message: "角色已更新".to_string() })
            } else {
                Json(ReviewResponse { success: false, message: "用户不存在".to_string() })
            }
        } else {
            Json(ReviewResponse { success: false, message: "未登录".to_string() })
        }
    } else {
        Json(ReviewResponse { success: false, message: "未登录".to_string() })
    }
}

// ============== Remove Member ==============

pub async fn remove_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Json<ReviewResponse> {
    if user_id == ADMIN_USER_ID {
        return Json(ReviewResponse { success: false, message: "不能移除系统管理员".to_string() });
    }
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(admin_user_id) = sessions.get(&token) {
            let users = state.users.read().await;
            if users.contains_key(admin_user_id) {
                if !is_admin(&state, admin_user_id).await {
                    return Json(ReviewResponse { success: false, message: "权限不足".to_string() });
                }
                if admin_user_id == &user_id {
                    return Json(ReviewResponse { success: false, message: "不能移除自己".to_string() });
                }
                // Only superadmin can remove an admin
                let target_is_admin = users.get(&user_id).map(|u| u.role.as_str()) == Some("admin");
                if target_is_admin && !is_superadmin(&state, admin_user_id).await {
                    return Json(ReviewResponse { success: false, message: "权限不足：仅限系统管理员操作".to_string() });
                }
                let mut members = state.team_members.write().await;
                let member_id = members.values()
                    .find(|m| m.user_id == user_id)
                    .map(|m| m.id.clone());
                if let Some(mid) = member_id {
                    members.remove(&mid);
                    drop(members);
                    drop(users);
                    if let Some(u) = state.users.write().await.get_mut(&user_id) {
                        u.team_status = "none".to_string();
                        u.role = "guest".to_string();
                    }
                    state.save().await;
                    Json(ReviewResponse { success: true, message: "已移除团队成员".to_string() })
                } else {
                    Json(ReviewResponse { success: false, message: "团队成员不存在".to_string() })
                }
            } else {
                Json(ReviewResponse { success: false, message: "用户不存在".to_string() })
            }
        } else {
            Json(ReviewResponse { success: false, message: "未登录".to_string() })
        }
    } else {
        Json(ReviewResponse { success: false, message: "未登录".to_string() })
    }
}
