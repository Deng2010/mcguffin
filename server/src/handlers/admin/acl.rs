use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::state::AppState;
use crate::types::{SetAclPayload, SetProblemAclPayload, PERM_WILDCARD};
use crate::utils::AuthUser;

// ============== List All Resources for ACL ==============

/// GET /api/admin/acl/resources — return all problems, contests, posts with ACL data
pub async fn get_acl_resources(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let problems = state.problems.read().await;
    let contests = state.contests.read().await;
    let posts = state.posts.read().await;
    let users = state.users.read().await;

    let problem_list: Vec<serde_json::Value> = problems
        .values()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "status": p.status,
                "visible_to": p.visible_to,
                "editable_by": p.editable_by,
            })
        })
        .collect();

    let contest_list: Vec<serde_json::Value> = contests
        .values()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "title": c.name,
                "status": c.status,
                "visible_to": c.visible_to,
                "editable_by": c.editable_by,
            })
        })
        .collect();

    let post_list: Vec<serde_json::Value> = posts
        .values()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "status": p.status,
                "team_only": p.team_only,
                "visible_to": p.visible_to,
                "editable_by": p.editable_by,
            })
        })
        .collect();

    let user_list: Vec<serde_json::Value> = users
        .values()
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "username": u.username,
                "display_name": u.display_name,
                "role": u.role,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "problems": problem_list,
        "contests": contest_list,
        "posts": post_list,
        "users": user_list,
    })))
}

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
