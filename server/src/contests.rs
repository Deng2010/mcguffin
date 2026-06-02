use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::state::AppState;
use crate::types::*;
use crate::utils::{check_permission, get_token_from_headers, AuthUser};

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct ContestRow {
    id: String,
    name: String,
    start_time: String,
    end_time: String,
    description: String,
    created_by: String,
    created_at: String,
    status: String,
    link: Option<String>,
    problem_order: String,
    visible_to: String,
    editable_by: String,
}

/// Row type for reading problems from SQLite.
/// All fields mirror the `problems` table columns; some are unused in this
/// endpoint but required for `sqlx::FromRow` to match the SELECT list.
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct ProblemRow {
    id: String,
    title: String,
    author_id: String,
    author_name: String,
    contest: Option<String>,
    contest_id: Option<String>,
    difficulty: String,
    content: String,
    solution: Option<String>,
    status: String,
    created_at: String,
    public_at: Option<String>,
    claimed_by: Option<String>,
    verifier_solution: Option<String>,
    visible_to: String,
    link: Option<String>,
    remark: Option<String>,
    editable_by: String,
}

/// Resolve user from token; returns (user_id, user)
async fn resolve_user(state: &AppState, headers: &HeaderMap) -> Option<(String, User)> {
    let token = get_token_from_headers(headers)?;
    let entry = state.sessions.read().await.get(&token)?.clone();
    let user_id = entry.user_id;
    let user = state.users.read().await.get(&user_id)?.clone();
    Some((user_id, user))
}

pub(crate) fn to_list_item(c: &Contest) -> ContestListItem {
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
        visible_to: c.visible_to.clone(),
        editable_by: c.editable_by.clone(),
    }
}

// ============== List Contests ==============

/// GET /api/contests
/// Anyone can list contests.
/// - Non-admin: only "public" contests
/// - Admin: all contests
pub async fn get_contests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Vec<ContestListItem>> {
    let current_user = resolve_user(&state, &headers).await;

    let (can_view_all, can_view_public) = if let Some((_, user)) = &current_user {
        (
            check_permission(&state, user, crate::types::perms::VIEW_ALL_CONTESTS).await,
            check_permission(&state, user, crate::types::perms::VIEW_PUBLIC_CONTESTS).await,
        )
    } else {
        (false, true)
    };

    // Try SQLite first
    let status_filter = if can_view_all {
        None
    } else if can_view_public {
        Some("public")
    } else {
        None // will return empty
    };

    // Build query
    let sql_result = if let Some(status) = status_filter {
        sqlx::query_as::<_, ContestRow>(
            "SELECT id, name, start_time, end_time, description, created_by, \
             created_at, status, link, problem_order, visible_to, editable_by \
             FROM contests WHERE status = ? ORDER BY created_at DESC",
        )
        .bind(status)
        .fetch_all(&state.db)
        .await
    } else if can_view_all {
        sqlx::query_as::<_, ContestRow>(
            "SELECT id, name, start_time, end_time, description, created_by, \
             created_at, status, link, problem_order, visible_to, editable_by \
             FROM contests ORDER BY created_at DESC",
        )
        .fetch_all(&state.db)
        .await
    } else {
        // No permission — return empty
        return Json(Vec::new());
    };

    match sql_result {
        Ok(rows) => {
            let list: Vec<ContestListItem> = rows
                .into_iter()
                .map(|r| ContestListItem {
                    id: r.id,
                    name: r.name,
                    start_time: r.start_time,
                    end_time: r.end_time,
                    description: r.description,
                    created_by: r.created_by,
                    created_at: r.created_at,
                    status: r.status,
                    link: r.link,
                    problem_order: serde_json::from_str(&r.problem_order).unwrap_or_default(),
                    visible_to: serde_json::from_str(&r.visible_to).unwrap_or_default(),
                    editable_by: serde_json::from_str(&r.editable_by).unwrap_or_default(),
                })
                .collect();
            Json(list)
        }
        Err(_) => {
            // Fallback to HashMap
            let contests = state.contests.read().await;
            let mut list: Vec<ContestListItem> = contests
                .values()
                .filter(|c| {
                    if can_view_all {
                        true
                    } else if can_view_public {
                        c.status == "public"
                    } else {
                        false
                    }
                })
                .map(crate::contests::to_list_item)
                .collect();
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Json(list)
        }
    }
}

// ============== Create Contest ==============

/// POST /api/contests
/// Admin only — creates as "draft"
pub async fn create_contest(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateContestPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_CONTESTS)
        .await?;

    let contest = Contest {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        start_time: payload.start_time,
        end_time: payload.end_time,
        description: payload.description,
        created_by: auth.user_id,
        created_at: Utc::now(),
        status: "draft".to_string(),
        link: payload.link,
        problem_order: vec![],
        visible_to: vec![],
        editable_by: vec![],
    };

    let cid = contest.id.clone();
    state.insert_contest(&contest).await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "比赛已创建", "contest_id": cid}),
    ))
}

// ============== Delete Contest ==============

/// DELETE /api/contests/:id
/// Admin only — also clears contest_id from all Problems referencing it
pub async fn delete_contest(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(contest_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_CONTESTS)
        .await?;

    // Check existence first without write lock
    let exists = state.contests.read().await.contains_key(&contest_id);
    if exists {
        state.clear_contest_from_problems(&contest_id).await;
        state.delete_contest_by_id(&contest_id).await;
        Ok(Json(
            serde_json::json!({"success": true, "message": "比赛已删除"}),
        ))
    } else {
        Ok(Json(
            serde_json::json!({"success": false, "message": "比赛不存在"}),
        ))
    }
}

// ============== Update Contest ==============

/// PUT /api/contests/:id
/// Admin only — updates name, time, description
pub async fn update_contest(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(contest_id): Path<String>,
    Json(payload): Json<UpdateContestPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_CONTESTS)
        .await?;

    let mut contest = match state.contests.read().await.get(&contest_id) {
        Some(c) => c.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "比赛不存在"}),
            ))
        }
    };

    contest.name = payload.name;
    contest.start_time = payload.start_time;
    contest.end_time = payload.end_time;
    contest.description = payload.description;
    if let Some(link) = payload.link {
        contest.link = if link.is_empty() { None } else { Some(link) };
    }
    state.insert_contest(&contest).await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "比赛已更新"}),
    ))
}

// ============== Set Contest Status ==============

/// POST /api/contests/:contest_id/status
/// Admin only — toggles between "draft" and "public"
pub async fn set_contest_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(contest_id): Path<String>,
    Json(payload): Json<SetContestStatusPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_CONTESTS)
        .await?;

    if payload.status != "draft" && payload.status != "public" {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "状态值无效，仅支持 draft 或 public"}),
        ));
    }

    let mut contest = match state.contests.read().await.get(&contest_id) {
        Some(c) => c.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "比赛不存在"}),
            ))
        }
    };

    if payload.status == "public" && contest.status != "public" {
        // Require link when making public — check payload first, then existing contest link
        let link = payload
            .link
            .as_deref()
            .or(contest.link.as_deref())
            .unwrap_or("");
        if link.is_empty() {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "设为公开前请先设置比赛链接"}),
            ));
        }
        contest.link = Some(link.to_string());
    }

    contest.status = payload.status;
    state.insert_contest(&contest).await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "比赛状态已更新"}),
    ))
}

// ============== Set Problem Order ==============

/// POST /api/contests/:contest_id/problem-order
/// Admin only — sets the ordered list of problem IDs for a contest
pub async fn set_problem_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(contest_id): Path<String>,
    Json(payload): Json<ContestProblemOrderPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_CONTESTS)
        .await?;

    let mut contest = match state.contests.read().await.get(&contest_id) {
        Some(c) => c.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "比赛不存在"}),
            ))
        }
    };

    // Validate that all problem_ids exist and belong to this contest
    let problems = state.problems.read().await;
    for pid in &payload.problem_ids {
        match problems.get(pid) {
            Some(p) if p.contest_id.as_deref() == Some(&contest_id) => {}
            _ => {
                return Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("题目 {} 不存在或不属于此比赛", pid)
                })));
            }
        }
    }
    drop(problems);

    contest.problem_order = payload.problem_ids;
    state.insert_contest(&contest).await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "题目顺序已更新"}),
    ))
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
    let _is_admin_user = if let Some((_user_id, user)) = &current_user {
        check_permission(&state, user, crate::types::perms::MANAGE_CONTESTS).await
    } else {
        false
    };
    let _is_member_user = if let Some((_, user)) = &current_user {
        user.team_status == "joined"
    } else {
        false
    };

    // ── Try SQLite first ──
    let sql_result: Result<Vec<ProblemRow>, _> = sqlx::query_as::<_, ProblemRow>(
        "SELECT id, title, author_id, author_name, contest, contest_id, difficulty, \
         content, solution, status, created_at, public_at, claimed_by, \
         verifier_solution, visible_to, link, remark, editable_by \
         FROM problems WHERE contest_id = ?",
    )
    .bind(&contest_id)
    .fetch_all(&state.db)
    .await;

    let mut contest_problems: Vec<serde_json::Value> = if let Ok(rows) = sql_result {
        rows.iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "title": p.title,
                    "author_name": p.author_name,
                    "difficulty": p.difficulty,
                    "status": p.status,
                })
            })
            .collect()
    } else {
        // ── Fallback to HashMap ──
        let problems = state.problems.read().await;
        problems
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
            .collect()
    };

    // Contest info is still read from HashMap for sorting / metadata
    let contests = state.contests.read().await;
    let contest = match contests.get(&contest_id) {
        Some(c) => c.clone(),
        None => return Json(vec![]),
    };

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
