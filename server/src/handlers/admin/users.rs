use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::state::{AppState, ADMIN_USER_ID};
use crate::types::{ChangeRolePayload, PERM_WILDCARD, SetUserGroupsPayload, SetUserPermissionsPayload};
use crate::utils::AuthUser;

// ============== User Management ==============

/// GET /api/admin/users
/// List all users (superadmin only)
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct AdminUserRow {
    id: String,
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    email: Option<String>,
    role: String,
    team_status: String,
    created_at: String,
    bio: String,
    group_ids: String,
    user_permissions: String,
    is_team_member: bool,
}

pub async fn admin_list_users(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    // Try SQLite first, then fallback to HashMap
    let sql_result = sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.username, u.display_name, u.avatar_url, u.email, u.role, \
         u.team_status, u.created_at, u.bio, u.group_ids, u.user_permissions, \
         CASE WHEN tm.user_id IS NOT NULL THEN 1 ELSE 0 END as is_team_member \
         FROM users u \
         LEFT JOIN team_members tm ON u.id = tm.user_id \
         ORDER BY u.created_at DESC",
    )
    .fetch_all(&state.db)
    .await;

    match sql_result {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "username": r.username,
                        "display_name": r.display_name,
                        "email": r.email,
                        "role": r.role,
                        "team_status": r.team_status,
                        "is_team_member": r.is_team_member,
                        "group_ids": serde_json::from_str::<Vec<String>>(&r.group_ids).unwrap_or_default(),
                        "user_permissions": serde_json::from_str::<Vec<String>>(&r.user_permissions).unwrap_or_default(),
                        "created_at": r.created_at,
                    })
                })
                .collect();
            Ok(Json(serde_json::json!(result)))
        }
        Err(_) => {
            // Fallback to HashMap
            let users = state.users.lock().await;
            let members = state.team_members.read().await;
            let result: Vec<serde_json::Value> = users
                .values()
                .map(|u| {
                    let is_team_member = members.values().any(|m| m.user_id == u.id);
                    serde_json::json!({
                        "id": u.id,
                        "username": u.username,
                        "display_name": u.display_name,
                        "email": u.email,
                        "role": u.role,
                        "team_status": u.team_status,
                        "is_team_member": is_team_member,
                        "group_ids": u.group_ids,
                        "user_permissions": u.user_permissions,
                        "created_at": u.created_at,
                    })
                })
                .collect();
            Ok(Json(serde_json::json!(result)))
        }
    }
}

/// POST /api/admin/users/{user_id}/role
/// Change a user's role (superadmin only)
pub async fn admin_change_user_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<ChangeRolePayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    if user_id == ADMIN_USER_ID {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "不能修改系统管理员的角色"}),
        ));
    }
    if payload.role != "admin" && payload.role != "member" && payload.role != "guest" {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效角色"}),
        ));
    }
    if state.users.lock().await.contains_key(&user_id) {
        state
            .update_user_field(&user_id, "role", payload.role.clone())
            .await;
        Ok(Json(
            serde_json::json!({"success": true, "message": "角色已更新"}),
        ))
    } else {
        Ok(Json(
            serde_json::json!({"success": false, "message": "用户不存在"}),
        ))
    }
}

/// POST /api/admin/users/{user_id}/remove
/// Remove (delete) a user (superadmin only)
pub async fn admin_remove_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    if user_id == ADMIN_USER_ID {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "不能删除系统管理员"}),
        ));
    }
    if user_id == auth.user_id {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "不能删除自己"}),
        ));
    }
    state.delete_user(&user_id).await;
    state.remove_team_member_by_user(&user_id).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "用户已删除"}),
    ))
}

// ============== User-Group Membership ==============

/// PUT /api/admin/users/{user_id}/groups — set user's group membership
pub async fn set_user_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<SetUserGroupsPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let user = match state.users.lock().await.get(&user_id) {
        Some(u) => u.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "用户不存在"}),
            ))
        }
    };
    let mut user = user;
    user.group_ids = payload.group_ids;
    state.upsert_user(&user).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "用户组已更新"}),
    ))
}

// ============== User Individual Permissions ==============

/// PUT /api/admin/users/{user_id}/permissions — set user's individual permissions
pub async fn set_user_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<SetUserPermissionsPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let user = match state.users.lock().await.get(&user_id) {
        Some(u) => u.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "用户不存在"}),
            ))
        }
    };
    let mut user = user;
    user.user_permissions = payload.permissions;
    state.upsert_user(&user).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "用户权限已更新"}),
    ))
}
