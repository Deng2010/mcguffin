use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::state::AppState;
use crate::types::*;
use crate::utils::get_token_from_headers;

/// Resolve user from token; returns (user_id, user)
async fn resolve_user<'a>(state: &'a AppState, headers: &HeaderMap) -> Option<(String, User)> {
    let token = get_token_from_headers(headers)?;
    let entry = state.sessions.read().await.get(&token)?.clone();
    let user_id = entry.user_id;
    let user = state.users.read().await.get(&user_id)?.clone();
    Some((user_id, user))
}

/// Check if user has admin role (includes superadmin)
async fn is_admin(state: &AppState, user_id: &str) -> bool {
    let users = state.users.read().await;
    users.get(user_id).map(|u| u.role == "admin" || u.role == "superadmin").unwrap_or(false)
}

/// Check if user is a team member
async fn is_team_member(state: &AppState, user_id: &str) -> bool {
    let members = state.team_members.read().await;
    members.values().any(|m| m.user_id == user_id)
}

fn to_list_item(c: &Contest) -> ContestListItem {
    ContestListItem {
        id: c.id.clone(),
        name: c.name.clone(),
        start_time: c.start_time.clone(),
        end_time: c.end_time.clone(),
        description: c.description.clone(),
        created_by: c.created_by.clone(),
        created_at: c.created_at.format("%Y-%m-%d %H:%M").to_string(),
        status: c.status.clone(),
        link: c.link.clone(),
        problem_order: c.problem_order.clone(),
    }
}

// ============== List Contests ==============

/// GET /api/contests
/// Anyone can list contests.
/// - Non-admin: only "public" contests
/// - Admin: all contests (or with ?public=true to see only public)
pub async fn get_contests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Vec<ContestListItem>> {
    let current_user = resolve_user(&state, &headers).await;
    let is_admin_user = if let Some((user_id, _)) = &current_user {
        is_admin(&state, user_id).await
    } else {
        false
    };
    let is_member_user = if let Some((user_id, _)) = &current_user {
        is_team_member(&state, user_id).await
    } else {
        false
    };

    let contests = state.contests.read().await;
    let mut list: Vec<ContestListItem> = contests
        .values()
        .filter(|c| {
            if is_admin_user {
                true // admin sees all
            } else if is_member_user {
                true // members see all (including unpublished)
            } else {
                c.status == "public" // guests see only public
            }
        })
        .map(|c| to_list_item(c))
        .collect();
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(list)
}

// ============== Create Contest ==============

/// POST /api/contests
/// Admin only — creates as "draft"
pub async fn create_contest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateContestPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let contest = Contest {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        start_time: payload.start_time,
        end_time: payload.end_time,
        description: payload.description,
        created_by: user_id,
        created_at: Utc::now(),
        status: "draft".to_string(),
        link: payload.link,
        problem_order: vec![],
    };

    let cid = contest.id.clone();
    state.contests.write().await.insert(contest.id.clone(), contest);
    state.save().await;

    Json(serde_json::json!({"success": true, "message": "比赛已创建", "contest_id": cid}))
}

// ============== Delete Contest ==============

/// DELETE /api/contests/:id
/// Admin only — also clears contest_id from all Problems referencing it
pub async fn delete_contest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(contest_id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let mut contests = state.contests.write().await;
    if contests.remove(&contest_id).is_some() {
        drop(contests);
        // Also clear contest_id from any problems referencing this contest
        let mut problems = state.problems.write().await;
        for p in problems.values_mut() {
            if p.contest_id.as_deref() == Some(&contest_id) {
                p.contest_id = None;
            }
        }
        drop(problems);
        state.save().await;
        Json(serde_json::json!({"success": true, "message": "比赛已删除"}))
    } else {
        Json(serde_json::json!({"success": false, "message": "比赛不存在"}))
    }
}

// ============== Update Contest ==============

/// PUT /api/contests/:id
/// Admin only — updates name, time, description
pub async fn update_contest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(contest_id): Path<String>,
    Json(payload): Json<UpdateContestPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let mut contests = state.contests.write().await;
    let contest = match contests.get_mut(&contest_id) {
        Some(c) => c,
        None => return Json(serde_json::json!({"success": false, "message": "比赛不存在"})),
    };

    contest.name = payload.name;
    contest.start_time = payload.start_time;
    contest.end_time = payload.end_time;
    contest.description = payload.description;
    if let Some(link) = payload.link {
        contest.link = if link.is_empty() { None } else { Some(link) };
    }
    drop(contests);

    state.save().await;
    Json(serde_json::json!({"success": true, "message": "比赛已更新"}))
}

// ============== Set Contest Status ==============

/// POST /api/contests/:contest_id/status
/// Admin only — toggles between "draft" and "public"
pub async fn set_contest_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(contest_id): Path<String>,
    Json(payload): Json<SetContestStatusPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    if payload.status != "draft" && payload.status != "public" {
        return Json(serde_json::json!({"success": false, "message": "状态值无效，仅支持 draft 或 public"}));
    }

    let mut contests = state.contests.write().await;
    let contest = match contests.get_mut(&contest_id) {
        Some(c) => c,
        None => return Json(serde_json::json!({"success": false, "message": "比赛不存在"})),
    };

    if payload.status == "public" && contest.status != "public" {
        // Require link when making public — check payload first, then existing contest link
        let link = payload.link.as_deref().or(contest.link.as_deref()).unwrap_or("");
        if link.is_empty() {
            return Json(serde_json::json!({"success": false, "message": "设为公开前请先设置比赛链接"}));
        }
        contest.link = Some(link.to_string());
    }

    contest.status = payload.status;
    drop(contests);
    state.save().await;

    Json(serde_json::json!({"success": true, "message": "比赛状态已更新"}))
}

// ============== Set Problem Order ==============

/// POST /api/contests/:contest_id/problem-order
/// Admin only — sets the ordered list of problem IDs for a contest
pub async fn set_problem_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(contest_id): Path<String>,
    Json(payload): Json<ContestProblemOrderPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let mut contests = state.contests.write().await;
    let contest = match contests.get_mut(&contest_id) {
        Some(c) => c,
        None => return Json(serde_json::json!({"success": false, "message": "比赛不存在"})),
    };

    // Validate that all problem_ids exist and belong to this contest
    let problems = state.problems.read().await;
    for pid in &payload.problem_ids {
        match problems.get(pid) {
            Some(p) if p.contest_id.as_deref() == Some(&contest_id) => {}
            _ => {
                return Json(serde_json::json!({
                    "success": false,
                    "message": format!("题目 {} 不存在或不属于此比赛", pid)
                }));
            }
        }
    }
    drop(problems);

    contest.problem_order = payload.problem_ids;
    drop(contests);
    state.save().await;

    Json(serde_json::json!({"success": true, "message": "题目顺序已更新"}))
}

// ============== Get Contest Problems (Ordered) ==============

/// GET /api/contests/:contest_id/problems
/// Returns problems belonging to this contest, ordered by problem_order if set.
pub async fn get_contest_problems(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(contest_id): Path<String>,
) -> Json<Vec<serde_json::Value>> {
    let current_user = resolve_user(&state, &headers).await;
    // Check admin status for later filtering
    let _is_admin_user = if let Some((user_id, _)) = &current_user {
        is_admin(&state, user_id).await
    } else {
        false
    };
    let _is_member_user = if let Some((user_id, _)) = &current_user {
        is_team_member(&state, user_id).await
    } else {
        false
    };

    let problems = state.problems.read().await;
    let contests = state.contests.read().await;
    let contest = match contests.get(&contest_id) {
        Some(c) => c.clone(),
        None => return Json(vec![]),
    };

    // Collect problems that belong to this contest
    let mut contest_problems: Vec<serde_json::Value> = problems
        .values()
        .filter(|p| p.contest_id.as_deref() == Some(&contest_id))
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "author_name": p.author_name,
                "difficulty": p.difficulty,
                "status": p.status,
            })
        })
        .collect();

    // Sort by problem_order if set
    if !contest.problem_order.is_empty() {
        let order_map: std::collections::HashMap<&str, usize> = contest
            .problem_order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        contest_problems.sort_by_cached_key(|p| {
            p.get("id")
                .and_then(|v| v.as_str())
                .and_then(|id| order_map.get(id))
                .copied()
                .unwrap_or(usize::MAX)
        });
    } else {
        // Default sort: by creation time (oldest first)
        contest_problems.sort_by(|a, b| {
            let a_id = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let b_id = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
            a_id.cmp(b_id)
        });
    }

    Json(contest_problems)
}
