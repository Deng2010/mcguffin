use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::state::AppState;
use crate::types::{CreateGroupPayload, UpdateGroupPayload, MemberGroup, PERM_WILDCARD};
use crate::utils::AuthUser;

use super::config::sync_groups_to_config;

// ============== Member Groups CRUD ==============

/// GET /api/admin/groups — list all member groups
pub async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let groups = state.member_groups.read().await;
    let result: Vec<serde_json::Value> = groups
        .values()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "name": g.name,
                "permissions": g.permissions,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(result)))
}

/// POST /api/admin/groups — create a member group
pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateGroupPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "组名不能为空"}),
        ));
    }
    let id = Uuid::new_v4().to_string();
    let group = MemberGroup {
        id: id.clone(),
        name,
        permissions: payload.permissions,
    };
    state.member_groups.write().await.insert(id.clone(), group);
    sync_groups_to_config(&state).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "成员组已创建", "id": id}),
    ))
}

/// PUT /api/admin/groups/{group_id} — update a member group
pub async fn update_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<String>,
    Json(payload): Json<UpdateGroupPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let mut groups = state.member_groups.write().await;
    let group = match groups.get_mut(&group_id) {
        Some(g) => g,
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "成员组不存在"}),
            ))
        }
    };
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "组名不能为空"}),
        ));
    }
    group.name = name;
    group.permissions = payload.permissions;
    drop(groups);
    sync_groups_to_config(&state).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "成员组已更新"}),
    ))
}

/// DELETE /api/admin/groups/{group_id} — delete a member group
pub async fn delete_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let mut groups = state.member_groups.write().await;
    if !groups.contains_key(&group_id) {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "成员组不存在"}),
        ));
    }
    groups.remove(&group_id);
    drop(groups);

    state.remove_group_from_all_users(&group_id).await;
    sync_groups_to_config(&state).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "成员组已删除"}),
    ))
}
