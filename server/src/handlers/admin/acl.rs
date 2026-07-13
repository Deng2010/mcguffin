use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::state::AppState;
use crate::types::{SetAclPayload, SetProblemAclPayload, PERM_WILDCARD};
use crate::utils::AuthUser;

// ============== Problem Resource ACL ==============

/// PUT /api/admin/problems/{problem_id}/acl — set who can edit a problem
pub async fn set_problem_acl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(problem_id): Path<String>,
    Json(payload): Json<SetProblemAclPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let mut problem = match state.problems.read().await.get(&problem_id) {
        Some(p) => p.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "题目不存在"}),
            ))
        }
    };
    problem.editable_by = payload.editable_by;
    state.insert_problem(&problem).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "题目访问控制已更新"}),
    ))
}

// ============== Unified Resource ACL ==============

/// PUT /api/admin/acl/{resource_type}/{resource_id} — set ACL for any resource
/// resource_type: "problem" | "contest" | "post"
pub async fn set_resource_acl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Json(payload): Json<SetAclPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    {
        let mut found = false;
        match resource_type.as_str() {
            "problem" => {
                if let Some(mut p) = state.problems.read().await.get(&resource_id).cloned() {
                    p.visible_to = payload.visible_to.clone();
                    p.editable_by = payload.editable_by.clone();
                    state.insert_problem(&p).await;
                    found = true;
                }
            }
            "contest" => {
                if let Some(mut c) = state.contests.read().await.get(&resource_id).cloned() {
                    c.visible_to = payload.visible_to.clone();
                    c.editable_by = payload.editable_by.clone();
                    state.insert_contest(&c).await;
                    found = true;
                }
            }
            "post" | "discussion" => {
                if let Some(mut p) = state.posts.read().await.get(&resource_id).cloned() {
                    p.visible_to = payload.visible_to.clone();
                    p.editable_by = payload.editable_by.clone();
                    state.upsert_post(&p).await;
                    found = true;
                }
            }
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"success": false, "message": "无效的资源类型"})),
                ))
            }
        }
        if !found {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "资源不存在"})),
            ));
        }
    } // write locks dropped here

    Ok(Json(
        serde_json::json!({"success": true, "message": "访问控制已更新"}),
    ))
}
