use axum::http::StatusCode;

#[derive(sqlx::FromRow)]
struct TeamMemberRow {
    id: String,
    user_id: String,
    joined_at: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    username: Option<String>,
    role: Option<String>,
}

#[derive(sqlx::FromRow)]
struct JoinRequestRow {
    id: String,
    user_id: String,
    user_name: String,
    user_email: Option<String>,
    reason: String,
    status: String,
    created_at: String,
}
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::state::{AppState, ADMIN_USER_ID};
use crate::types::*;
use crate::utils::{check_permission, get_token_from_headers, AuthUser};

// ============== List Team Members ==============

pub async fn get_team_members(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::VIEW_TEAM)
        .await?;
    let is_admin_user =
        check_permission(&state, &auth.user, crate::types::perms::MANAGE_TEAM).await;
    let is_superadmin_user = check_permission(&state, &auth.user, PERM_WILDCARD).await;

    // 尝试从 SQLite 查询（含 LEFT JOIN），失败时回退到 HashMap
    let sql_result = sqlx::query_as::<_, TeamMemberRow>(
        "SELECT tm.id, tm.user_id, tm.joined_at, \
         u.display_name, u.avatar_url, u.username, u.role \
         FROM team_members tm \
         LEFT JOIN users u ON tm.user_id = u.id",
    )
    .fetch_all(&state.db)
    .await;

    let result: Vec<serde_json::Value> = if let Ok(rows) = sql_result {
        rows.into_iter()
            .filter(|r| {
                let member_role = r.role.as_deref().unwrap_or("");
                if !is_superadmin_user && member_role == "superadmin" {
                    return false;
                }
                is_admin_user || r.user_id != ADMIN_USER_ID
            })
            .map(|r| {
                let current_name = r.display_name.unwrap_or_default();
                let current_avatar_url = r.avatar_url;
                let user_username = r.username.unwrap_or_default();
                let user_role = r.role.unwrap_or_default();
                let current_avatar = current_name
                    .chars()
                    .next()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "U".to_string());
                serde_json::json!({
                    "id": r.id,
                    "user_id": r.user_id,
                    "name": current_name,
                    "username": user_username,
                    "avatar": current_avatar,
                    "avatar_url": current_avatar_url,
                    "role": user_role,
                    "joined_at": r.joined_at,
                })
            })
            .collect()
    } else {
        // 回退：HashMap 模式
        let members = state.team_members.read().await;
        let users = state.users.lock().await;
        members
            .values()
            .filter(|m| {
                let member_role = users.get(&m.user_id).map(|u| u.role.as_str()).unwrap_or("");
                if !is_superadmin_user && member_role == "superadmin" {
                    return false;
                }
                is_admin_user || m.user_id != ADMIN_USER_ID
            })
            .map(|m| {
                let user_info = users.get(&m.user_id);
                let current_name = user_info
                    .map(|u| u.display_name.clone())
                    .unwrap_or_default();
                let current_avatar_url = user_info.and_then(|u| u.avatar_url.clone());
                let user_username = user_info.map(|u| u.username.clone()).unwrap_or_default();
                let user_role = user_info.map(|u| u.role.clone()).unwrap_or_default();
                let current_avatar = current_name
                    .chars()
                    .next()
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
            .collect()
    };
    Ok(Json(serde_json::json!(result)))
}

// ============== List Pending Join Requests ==============

pub async fn get_pending_requests(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_TEAM)
        .await?;

    let rows = sqlx::query_as::<_, JoinRequestRow>(
        "SELECT id, user_id, user_name, user_email, reason, status, created_at \
         FROM join_requests WHERE status = 'pending' ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "数据库查询失败"})),
        )
    })?;

    let requests: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "user_id": r.user_id,
                "user_name": r.user_name,
                "user_email": r.user_email,
                "reason": r.reason,
                "status": r.status,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(requests)))
}

// ============== Apply to Join ==============

pub async fn apply_to_join(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ApplyPayload>,
) -> Json<ApplyResponse> {
    if let Some(token) = get_token_from_headers(&headers) {
        let sessions = state.sessions.read().await;
        if let Some(entry) = sessions.get(&token) {
            if entry.user_id == ADMIN_USER_ID {
                return Json(ApplyResponse {
                    success: false,
                    message: "管理员无需申请".to_string(),
                });
            }
            let user_id = entry.user_id.clone();
            let users = state.users.lock().await;
            if let Some(user) = users.get(&user_id) {
                if user.team_status == "joined" {
                    return Json(ApplyResponse {
                        success: false,
                        message: "您已是团队成员".to_string(),
                    });
                }
                // Check for existing pending join request instead of using team_status
                let has_pending = state
                    .join_requests
                    .read()
                    .await
                    .values()
                    .any(|r| r.user_id == user_id && r.status == "pending");
                if has_pending {
                    return Json(ApplyResponse {
                        success: false,
                        message: "您已提交过申请，请等待审核".to_string(),
                    });
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

                state.insert_join_request(&request).await;

                // Don't change team_status — user stays as "guest"

                return Json(ApplyResponse {
                    success: true,
                    message: "申请已提交".to_string(),
                });
            }
        }
    }
    Json(ApplyResponse {
        success: false,
        message: "未登录".to_string(),
    })
}

// ============== Review Join Application ==============

pub async fn review_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((request_id, action)): Path<(String, String)>,
) -> Json<ReviewResponse> {
    if !check_permission(&state, &auth.user, crate::types::perms::MANAGE_TEAM).await {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }

    let action_clone = action.clone();
    let (target_user_id, new_status) = {
        // 先读取当前申请信息
        let pending_request = state.join_requests.read().await.get(&request_id).cloned();
        let Some(request) = pending_request else {
            return Json(ReviewResponse {
                success: false,
                message: "申请不存在".to_string(),
            });
        };
        let uid = request.user_id.clone();

        if action_clone == "approve" {
            state
                .update_join_request_status(&request_id, "approved")
                .await;
            // 检查是否已是团队成员
            if !state.is_team_member(&request.user_id).await {
                let member = TeamMember {
                    id: Uuid::new_v4().to_string(),
                    user_id: request.user_id.clone(),
                    joined_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                };
                state.insert_team_member(&member).await;
            }
            (uid, "joined".to_string())
        } else if action_clone == "reject" {
            state
                .update_join_request_status(&request_id, "rejected")
                .await;
            (uid, "none".to_string())
        } else {
            return Json(ReviewResponse {
                success: false,
                message: "无效操作".to_string(),
            });
        }
    };

    state
        .update_user_field(&target_user_id, "team_status", new_status.clone())
        .await;
    let role = if new_status == "joined" {
        "member"
    } else {
        "guest"
    };
    state
        .update_user_field(&target_user_id, "role", role.to_string())
        .await;

    match action_clone.as_str() {
        "approve" => Json(ReviewResponse {
            success: true,
            message: "已批准申请".to_string(),
        }),
        "reject" => Json(ReviewResponse {
            success: true,
            message: "已拒绝申请".to_string(),
        }),
        _ => Json(ReviewResponse {
            success: false,
            message: "无效操作".to_string(),
        }),
    }
}

// ============== Change Member Role ==============

pub async fn change_member_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<ChangeRolePayload>,
) -> Json<ReviewResponse> {
    if user_id == ADMIN_USER_ID {
        return Json(ReviewResponse {
            success: false,
            message: "不能修改系统管理员的角色".to_string(),
        });
    }
    if !check_permission(&state, &auth.user, crate::types::perms::MANAGE_MEMBERS).await {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }
    if payload.role != "admin" && payload.role != "member" {
        return Json(ReviewResponse {
            success: false,
            message: "无效角色".to_string(),
        });
    }
    // Only superadmin can demote an existing admin or promote to admin
    let is_super = check_permission(&state, &auth.user, PERM_WILDCARD).await;
    let users = state.users.lock().await;
    let target_role = users.get(&user_id).map(|u| u.role.as_str());
    if let Some("admin") = target_role {
        if !is_super {
            return Json(ReviewResponse {
                success: false,
                message: "权限不足：仅限系统管理员操作".to_string(),
            });
        }
    }
    if payload.role == "admin" && !is_super {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足：仅限系统管理员操作".to_string(),
        });
    }
    // Check if user is a team member
    let is_member = state
        .team_members
        .read()
        .await
        .values()
        .any(|m| m.user_id == user_id);
    if !is_member {
        return Json(ReviewResponse {
            success: false,
            message: "团队成员不存在".to_string(),
        });
    }
    drop(users);
    state
        .update_user_field(&user_id, "role", payload.role.clone())
        .await;
    Json(ReviewResponse {
        success: true,
        message: "角色已更新".to_string(),
    })
}

// ============== Remove Member ==============

pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
) -> Json<ReviewResponse> {
    if user_id == ADMIN_USER_ID {
        return Json(ReviewResponse {
            success: false,
            message: "不能移除系统管理员".to_string(),
        });
    }
    if !check_permission(&state, &auth.user, crate::types::perms::MANAGE_MEMBERS).await {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }
    if auth.user_id == user_id {
        return Json(ReviewResponse {
            success: false,
            message: "不能移除自己".to_string(),
        });
    }
    let users = state.users.lock().await;
    // Only superadmin can remove an admin
    let target_is_admin = users.get(&user_id).map(|u| u.role.as_str()) == Some("admin");
    if target_is_admin && !check_permission(&state, &auth.user, PERM_WILDCARD).await {
        drop(users);
        return Json(ReviewResponse {
            success: false,
            message: "权限不足：仅限系统管理员操作".to_string(),
        });
    }
    let is_member = state.is_team_member(&user_id).await;
    if !is_member {
        return Json(ReviewResponse {
            success: false,
            message: "团队成员不存在".to_string(),
        });
    }

    state.remove_team_member_by_user(&user_id).await;
    drop(users);
    state
        .update_user_field(&user_id, "team_status", "none".to_string())
        .await;
    state
        .update_user_field(&user_id, "role", "guest".to_string())
        .await;
    Json(ReviewResponse {
        success: true,
        message: "已移除团队成员".to_string(),
    })
}
