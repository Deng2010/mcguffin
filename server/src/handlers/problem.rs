use axum::http::HeaderMap;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{json_error, ErrorCode};
use crate::handlers::notification::create_notification;
use crate::state::AppState;
use crate::types::*;
use crate::utils::{check_permission, resolve_user, AuthUser};

// ── ProblemRow for SQLite deserialization ──
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
    verifiers: String,
}

// ============== List Problems ==============

/// GET /api/problems?all=true
/// - Everyone (default): only "published"
/// - Admin with ?all=true: all except "rejected"
pub async fn get_problems(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ProblemListItem>> {
    let current_user = resolve_user(&state, &headers).await;
    let is_admin_user = if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::APPROVE_ALL_PROBLEMS).await
    } else {
        false
    };
    // 细分后的题目浏览权限：待审核 / 已通过 / 公开
    let can_view_pending = if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::VIEW_PENDING_PROBLEMS).await
    } else {
        false
    };
    let can_view_approved = if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::VIEW_APPROVED_PROBLEMS).await
    } else {
        false
    };
    // 公开题目沿用原有匿名可见行为：未登录也允许浏览公开题目。
    let can_view_public = if current_user.is_none() {
        true
    } else if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::VIEW_PUBLIC_PROBLEMS).await
    } else {
        false
    };
    let current_uid = current_user.as_ref().map(|(id, _)| id.clone());

    let show_all = params.get("all").map(|v| v == "true").unwrap_or(false);

    // Search/filter query params
    let search_q = params
        .get("search")
        .map(|v| v.to_lowercase())
        .filter(|v| !v.is_empty());
    let diff_filter = params.get("difficulty").filter(|v| !v.is_empty()).cloned();
    let status_filter = params.get("status").filter(|v| !v.is_empty()).cloned();
    let author_filter = params
        .get("author")
        .map(|v| v.to_lowercase())
        .filter(|v| !v.is_empty());

    let skip_all_filters = is_admin_user && show_all;

    // ── Pre-compute values needed for fallback ──
    let difficulty_order = state.difficulty_order.read().await.clone();

    // ── Build dynamic SQL query ──
    // Use a dynamic SQL string with ? placeholders and a Vec of bind values.
    // COALESCE(c.name, p.contest) resolves the contest name: if a contest record
    // exists by contest_id, use its current name; otherwise fall back to the
    // originally stored contest name (handles contest renames).
    let mut sql = String::from(
        "SELECT p.id, p.title, p.author_id, p.author_name, \
         COALESCE(c.name, p.contest) AS contest, p.contest_id, \
         p.difficulty, p.content, p.solution, p.status, \
         p.created_at, p.public_at, p.claimed_by, \
         p.verifier_solution, p.visible_to, p.link, p.remark, p.editable_by, p.verifiers \
         FROM problems p \
         LEFT JOIN contests c ON p.contest_id = c.id \
         WHERE p.status != 'rejected'",
    );
    let mut binds: Vec<String> = Vec::new();

    // Filter 1-4: 按细分后的浏览权限判断可见性
    if !skip_all_filters {
        let mut status_clauses: Vec<String> = Vec::new();
        if can_view_public {
            status_clauses.push("p.status = 'published'".to_string());
        }
        if can_view_approved {
            status_clauses.push("p.status = 'approved'".to_string());
        }
        // 待审核题目：有浏览待审核权限的用户可见；作者与可见名单成员始终可见。
        if can_view_pending {
            status_clauses.push("p.status = 'pending'".to_string());
        } else if let Some(uid) = &current_uid {
            status_clauses.push(
                "(p.status = 'pending' AND (p.author_id = ? OR EXISTS \
                 (SELECT 1 FROM json_each(p.visible_to) WHERE value = ?)))"
                    .to_string(),
            );
            binds.push(uid.clone());
            binds.push(uid.clone());
        }
        // 已退回题目：管理员/超级管理员可查看全部；其他用户仅作者可见。
        if is_admin_user {
            status_clauses.push("p.status = 'returned'".to_string());
        } else if let Some(uid) = &current_uid {
            status_clauses.push("(p.status = 'returned' AND p.author_id = ?)".to_string());
            binds.push(uid.clone());
        }

        if status_clauses.is_empty() {
            sql.push_str(" AND 1 = 0");
        } else {
            sql.push_str(" AND (");
            sql.push_str(&status_clauses.join(" OR "));
            sql.push(')');
        }
    }

    // Filter 5: search filter on title (case-insensitive contains)
    if !skip_all_filters {
        if let Some(q) = &search_q {
            sql.push_str(" AND LOWER(p.title) LIKE ?");
            binds.push(format!("%{}%", q));
        }
    }

    // Filter 6: difficulty filter (exact match)
    if !skip_all_filters {
        if let Some(d) = &diff_filter {
            sql.push_str(" AND p.difficulty = ?");
            binds.push(d.clone());
        }
    }

    // Filter 7: status filter (exact match)
    if !skip_all_filters {
        if let Some(s) = &status_filter {
            sql.push_str(" AND p.status = ?");
            binds.push(s.clone());
        }
    }

    // Filter 8: author filter (case-insensitive contains on author_name)
    if !skip_all_filters {
        if let Some(a) = &author_filter {
            sql.push_str(" AND LOWER(p.author_name) LIKE ?");
            binds.push(format!("%{}%", a));
        }
    }

    // ── Execute SQLite query ──
    let sql_result: Result<Vec<ProblemRow>, _> = {
        let mut query = sqlx::query_as::<_, ProblemRow>(&sql);
        for b in &binds {
            query = query.bind(b.as_str());
        }
        query.fetch_all(&state.db).await
    };

    match sql_result {
        Ok(rows) => {
            // Map rows to ProblemListItem (Filter 9: contest name already resolved via COALESCE)
            let mut problems: Vec<ProblemListItem> = rows
                .into_iter()
                .map(|row| {
                    let verifiers: Vec<VerifierEntry> =
                        serde_json::from_str(&row.verifiers).unwrap_or_default();
                    let verifier_summaries: Vec<VerifierSummary> = verifiers
                        .iter()
                        .map(|v| VerifierSummary {
                            user_id: v.user_id.clone(),
                            user_name: v.user_name.clone(),
                            has_solution: v.solution.is_some(),
                        })
                        .collect();
                    ProblemListItem {
                        id: row.id,
                        title: row.title,
                        author_id: row.author_id.clone(),
                        author_name: row.author_name.clone(),
                        contest: row.contest.unwrap_or_default(),
                        contest_id: row.contest_id,
                        difficulty: row.difficulty,
                        status: row.status.clone(),
                        created_at: DateTime::parse_from_rfc3339(&row.created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or(Utc::now()),
                        public_at: row.public_at.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                        claimed_by: row.claimed_by,
                        has_verifier_solution: row.verifier_solution.is_some(),
                        verifiers: verifier_summaries,
                        visible_to: serde_json::from_str(&row.visible_to).unwrap_or_default(),
                        link: row.link,
                        remark: if (row.status == "pending"
                            && (is_admin_user
                                || current_uid
                                    .as_ref()
                                    .is_some_and(|uid| row.author_id == *uid)))
                            || (row.status == "returned"
                                && current_uid
                                    .as_ref()
                                    .is_some_and(|uid| row.author_id == *uid))
                        {
                            row.remark.clone()
                        } else {
                            None
                        },
                    }
                })
                .collect();

            // Filter 10: Custom sort by difficulty_order
            problems.sort_by(|a, b| {
                let ai = difficulty_order.iter().position(|d| d == &a.difficulty);
                let bi = difficulty_order.iter().position(|d| d == &b.difficulty);
                ai.unwrap_or(999).cmp(&bi.unwrap_or(999))
            });

            Json(problems)
        }
        Err(e) => {
            // ── Fallback to HashMap ──
            tracing::warn!("SQLite query failed, falling back to HashMap: {}", e);

            let all_problems = state.problems.read().await;
            let contests = state.contests.read().await;

            let problems: Vec<ProblemListItem> = all_problems
                .values()
                .filter(|p| {
                    // Always exclude rejected problems
                    if p.status == "rejected" {
                        return false;
                    }
                    // 已退回题目：管理员/超级管理员可见全部；其他用户仅作者可见
                    if p.status == "returned"
                        && !is_admin_user
                        && !current_uid.as_ref().is_some_and(|uid| p.is_author(uid))
                    {
                        return false;
                    }
                    if skip_all_filters {
                        return true;
                    }
                    // Role-based access: 按细分后的浏览权限判断可见性
                    let is_author = current_uid.as_ref().is_some_and(|uid| p.is_author(uid));
                    let visible = match p.status.as_str() {
                        "published" => can_view_public,
                        "approved" => can_view_approved,
                        "pending" => {
                            can_view_pending
                                || is_author
                                || current_uid
                                    .as_ref()
                                    .is_some_and(|uid| p.visible_to.contains(uid))
                        }
                        "returned" => true,
                        _ => false,
                    };
                    if !visible {
                        return false;
                    }
                    // Filter 5: search on title
                    if let Some(q) = &search_q {
                        if !p.title.to_lowercase().contains(q) {
                            return false;
                        }
                    }
                    // Filter 6: difficulty filter
                    if let Some(d) = diff_filter.as_deref() {
                        if p.difficulty != d {
                            return false;
                        }
                    }
                    // Filter 7: status filter
                    if let Some(s) = status_filter.as_deref() {
                        if p.status != s {
                            return false;
                        }
                    }
                    // Filter 8: author filter
                    if let Some(a) = &author_filter {
                        if !p.author_name.to_lowercase().contains(a) {
                            return false;
                        }
                    }
                    true
                })
                .map(|p| {
                    // Filter 9: Contest name resolution
                    let contest_name = p
                        .contest_id
                        .as_ref()
                        .and_then(|cid| contests.get(cid))
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| p.contest.clone());
                    ProblemListItem {
                        id: p.id.clone(),
                        title: p.title.clone(),
                        author_id: p.author_id.clone(),
                        author_name: p.author_name.clone(),
                        contest: contest_name,
                        contest_id: p.contest_id.clone(),
                        difficulty: p.difficulty.clone(),
                        status: p.status.clone(),
                        created_at: p.created_at,
                        public_at: p.public_at,
                        claimed_by: p.claimed_by.clone(),
                        has_verifier_solution: p.verifier_solution.is_some(),
                        verifiers: p
                            .verifiers
                            .iter()
                            .map(|v| VerifierSummary {
                                user_id: v.user_id.clone(),
                                user_name: v.user_name.clone(),
                                has_solution: v.solution.is_some(),
                            })
                            .collect(),
                        visible_to: p.visible_to.clone(),
                        link: p.link.clone(),
                        remark: if (p.status == "pending"
                            && (is_admin_user
                                || current_uid.as_ref().is_some_and(|uid| p.is_author(uid))))
                            || (p.status == "returned"
                                && current_uid.as_ref().is_some_and(|uid| p.is_author(uid)))
                        {
                            p.remark.clone()
                        } else {
                            None
                        },
                    }
                })
                .collect::<Vec<_>>();

            // Filter 10: Sort by difficulty_order
            let mut problems = problems;
            problems.sort_by(|a, b| {
                let ai = difficulty_order.iter().position(|d| d == &a.difficulty);
                let bi = difficulty_order.iter().position(|d| d == &b.difficulty);
                ai.unwrap_or(999).cmp(&bi.unwrap_or(999))
            });

            Json(problems)
        }
    }
}

// ============== Get Problem Detail ==============

/// GET /api/problems/:id
/// Returns full problem (content, solution if permitted)
pub async fn get_problem_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let current_user = resolve_user(&state, &headers).await;

    let problem = if let Ok(Some(row)) =
        sqlx::query_as::<_, ProblemRow>("SELECT * FROM problems WHERE id = ?")
            .bind(&problem_id)
            .fetch_optional(&state.db)
            .await
    {
        Some(Problem {
            id: row.id,
            title: row.title,
            author_id: row.author_id,
            author_name: row.author_name,
            contest: row.contest.unwrap_or_default(),
            contest_id: row.contest_id,
            difficulty: row.difficulty,
            content: row.content,
            solution: row.solution,
            status: row.status,
            created_at: row.created_at.parse().unwrap_or_else(|_| Utc::now()),
            public_at: row.public_at.and_then(|s| s.parse().ok()),
            claimed_by: row.claimed_by,
            verifier_solution: row.verifier_solution,
            verifiers: serde_json::from_str(&row.verifiers).unwrap_or_default(),
            visible_to: serde_json::from_str(&row.visible_to).unwrap_or_default(),
            link: row.link,
            remark: row.remark,
            editable_by: serde_json::from_str(&row.editable_by).unwrap_or_default(),
        })
    } else {
        // Fallback to HashMap
        state.problems.read().await.get(&problem_id).cloned()
    };
    let problem = match problem {
        Some(p) => p,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                json_error(ErrorCode::PROBLEM_NOT_FOUND, "题目不存在"),
            ))
        }
    };

    // Check admin status properly
    let is_admin_user = if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::APPROVE_ALL_PROBLEMS).await
    } else {
        false
    };
    let is_member_user = if let Some((_, ref user)) = &current_user {
        user.team_status == "joined"
    } else {
        false
    };
    // 细分后的浏览权限检查（公开题目对未登录用户沿用原匿名可见行为）
    let can_view_pending = if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::VIEW_PENDING_PROBLEMS).await
    } else {
        false
    };
    let can_view_approved = if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::VIEW_APPROVED_PROBLEMS).await
    } else {
        false
    };
    let can_view_public = if current_user.is_none() {
        true
    } else if let Some((_, ref user)) = &current_user {
        check_permission(&state, user, crate::types::perms::VIEW_PUBLIC_PROBLEMS).await
    } else {
        false
    };
    let is_author = current_user
        .as_ref()
        .is_some_and(|(uid, _)| problem.is_author(uid));
    // Whether the current user is one of the verifiers
    let is_verifier = current_user
        .as_ref()
        .is_some_and(|(uid, _)| problem.verifiers.iter().any(|v| v.user_id == *uid));

    // Permission check
    let can_view = match problem.status.as_str() {
        "published" => can_view_public,
        "approved" => can_view_approved,
        "pending" => {
            if can_view_pending {
                true
            } else if let Some((uid, _)) = &current_user {
                // Author by user_id or display_name match
                is_author || problem.visible_to.contains(uid)
            } else {
                false
            }
        }
        // 已退回题目：管理员/超级管理员可查看全部；其他用户仅作者可见。
        "returned" => is_author || is_admin_user,
        _ => false,
    };
    if !can_view {
        return Err((
            StatusCode::FORBIDDEN,
            json_error(ErrorCode::PROBLEM_FORBIDDEN, "无权限查看"),
        ));
    }

    // Determine what to show
    let mut show_solution = match problem.status.as_str() {
        "published" => is_member_user || is_admin_user,
        "approved" => is_member_user || is_admin_user,
        "pending" => is_admin_user || is_author,
        "returned" => is_author,
        _ => false,
    };
    // A verifier (claimed) cannot see the author's solution (impartiality)
    if is_verifier {
        show_solution = false;
    }
    let in_visible_to = current_user
        .as_ref()
        .is_some_and(|(uid, _)| problem.visible_to.contains(uid));
    let show_content = match problem.status.as_str() {
        "published" => true,
        "approved" => true,
        "pending" => is_admin_user || is_author || in_visible_to,
        "returned" => is_author || is_admin_user,
        _ => false,
    };

    let contests = state.contests.read().await;
    // Derive contest name from contest_id if available
    let contest_name = problem
        .contest_id
        .as_ref()
        .and_then(|cid| contests.get(cid))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| problem.contest.clone());
    drop(contests);

    let mut resp = serde_json::json!({
        "id": problem.id,
        "title": problem.title,
        "author_id": problem.author_id,
        "author_name": problem.author_name,
        "contest": contest_name,
        "contest_id": problem.contest_id,
        "difficulty": problem.difficulty,
        "status": problem.status,
        "created_at": problem.created_at,
        "public_at": problem.public_at,
        "claimed_by": problem.claimed_by,
        "has_verifier_solution": problem.verifier_solution.is_some(),
        "verifiers": problem.verifiers,
        "is_verifier": is_verifier,
        "is_author": is_author,
        "link": problem.link,
    });

    if show_content {
        resp["content"] = serde_json::Value::String(problem.content);
        if let Some(remark) = &problem.remark {
            resp["remark"] = serde_json::Value::String(remark.clone());
        }
    }
    if show_solution {
        resp["solution"] = serde_json::Value::String(problem.solution.unwrap_or_default());
    }
    // All members can see the verifier's solution (backward-compat mirror)
    if let Some(vs) = &problem.verifier_solution {
        if is_member_user || is_admin_user {
            resp["verifier_solution"] = serde_json::Value::String(vs.clone());
        }
    }
    // Only a verifier can submit/edit their solution or post comments
    if let Some((user_id, _)) = &current_user {
        if problem.verifiers.iter().any(|v| v.user_id == *user_id) {
            resp["can_submit_verifier_solution"] = serde_json::Value::Bool(true);
        }
    }

    Ok(Json(resp))
}

// ============== Submit Problem ==============

pub async fn submit_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SubmitProblemPayload>,
) -> Json<SubmitResponse> {
    let user = match resolve_user(&state, &headers).await {
        Some((_, u)) if u.team_status == "joined" => u,
        Some(_) => {
            return Json(SubmitResponse {
                success: false,
                message: "只有团队成员才能投稿".to_string(),
                problem_id: None,
            })
        }
        None => {
            return Json(SubmitResponse {
                success: false,
                message: "未登录".to_string(),
                problem_id: None,
            })
        }
    };

    // Auto-fill contest name from contest_id if provided and contest is empty
    let mut contest = payload.contest;
    let contest_id = payload.contest_id.clone();
    if contest.is_empty() {
        if let Some(cid) = &contest_id {
            let contests = state.contests.read().await;
            if let Some(c) = contests.get(cid) {
                contest = c.name.clone();
            }
        }
    }

    let problem = Problem {
        id: Uuid::new_v4().to_string(),
        title: payload.title,
        author_id: user.id.clone(),
        author_name: user.display_name.clone(),
        contest,
        contest_id,
        difficulty: payload.difficulty,
        content: payload.content,
        solution: payload.solution,
        status: "pending".to_string(),
        created_at: Utc::now(),
        public_at: None,
        claimed_by: None,
        verifier_solution: None,
        verifiers: vec![],
        visible_to: vec![],
        link: payload.link,
        remark: payload.remark,
        editable_by: vec![],
    };

    let pid = problem.id.clone();
    state.insert_problem(&problem).await;

    Json(SubmitResponse {
        success: true,
        message: "提交成功，等待审核".to_string(),
        problem_id: Some(pid),
    })
}

// ============== Review Problem ==============

/// POST /api/problems/review/:problem_id/:action
/// action = "approve" | "reply" | "publish" | "return" | "reject" | "unpublish"
/// Body (optional): { "reason": "..." } — used for "return", "reject" and "reply" actions.
/// - "reject": pending -> returned (已退回), requires a reason of at least 10 chars.
/// - "return": approved -> pending, clearing all verifier solutions and comments.
pub async fn review_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((problem_id, action)): Path<(String, String)>,
    body: Option<Json<serde_json::Value>>,
) -> Json<ReviewResponse> {
    let reason = body
        .and_then(|b| {
            b.get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    let (_user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ReviewResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };
    if !check_permission(&state, &user, crate::types::perms::APPROVE_ALL_PROBLEMS).await {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }

    // For 'reply', notify the author with the admin's suggestion. The problem
    // stays in "pending" so the author can revise and resubmit (instead of the
    // old "reject" behavior which deleted the problem and hid the reason).
    if action == "reply" {
        if reason.trim().is_empty() {
            return Json(ReviewResponse {
                success: false,
                message: "请填写建议内容".to_string(),
            });
        }
        let problems = state.problems.read().await;
        let problem = match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ReviewResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        };
        if problem.status != "pending" {
            return Json(ReviewResponse {
                success: false,
                message: "只能回复待审核题目".to_string(),
            });
        }
        let author_id = problem.author_id.clone();
        let problem_title = problem.title.clone();
        drop(problems);

        let reply_msg = format!("题目「{}」的审核建议：{}", problem_title, reason);
        create_notification(
            &state,
            &author_id,
            "题目审核建议",
            &reply_msg,
            Some(&format!("/problems/{}", problem_id)),
        )
        .await;

        return Json(ReviewResponse {
            success: true,
            message: "已发送建议给出题人".to_string(),
        });
    }

    // Read problem to validate and capture info for notifications
    let problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ReviewResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };
    let author_id = problem.author_id.clone();
    let problem_title = problem.title.clone();

    let result = match action.as_str() {
        "approve" => {
            if problem.status != "pending" {
                return Json(ReviewResponse {
                    success: false,
                    message: "只能审核待审核题目".to_string(),
                });
            }
            state
                .update_problem_field(&problem_id, "status", "approved")
                .await;
            Json(ReviewResponse {
                success: true,
                message: "已批准题目".to_string(),
            })
        }
        "publish" => {
            if problem.status != "approved" {
                return Json(ReviewResponse {
                    success: false,
                    message: "只能发布已批准题目".to_string(),
                });
            }
            if problem.link.is_none() || problem.link.as_deref() == Some("") {
                return Json(ReviewResponse {
                    success: false,
                    message: "发布前请先设置题目链接".to_string(),
                });
            }
            // public_at is Option<DateTime>, use read-modify-write via insert_problem
            let mut p = state
                .problems
                .read()
                .await
                .get(&problem_id)
                .cloned()
                .unwrap();
            p.status = "published".to_string();
            p.public_at = Some(Utc::now());
            state.insert_problem(&p).await;
            Json(ReviewResponse {
                success: true,
                message: "已发布题目".to_string(),
            })
        }
        "reject" => {
            // 待审核 -> 已退回 (returned). Requires a reason of at least 10 chars.
            if problem.status != "pending" {
                return Json(ReviewResponse {
                    success: false,
                    message: "只能退回待审核题目".to_string(),
                });
            }
            if reason.trim().chars().count() < 10 {
                return Json(ReviewResponse {
                    success: false,
                    message: "退回理由不能少于 10 个字".to_string(),
                });
            }
            let mut p = state
                .problems
                .read()
                .await
                .get(&problem_id)
                .cloned()
                .unwrap();
            p.status = "returned".to_string();
            p.remark = Some(reason.trim().to_string());
            state.insert_problem(&p).await;
            Json(ReviewResponse {
                success: true,
                message: "已退回给出题人".to_string(),
            })
        }
        "return" => {
            if problem.status != "approved" {
                return Json(ReviewResponse {
                    success: false,
                    message: "只能退回已批准题目".to_string(),
                });
            }
            // approved -> pending, clearing ALL verifier solutions and comments.
            let mut p = state
                .problems
                .read()
                .await
                .get(&problem_id)
                .cloned()
                .unwrap();
            p.status = "pending".to_string();
            p.claimed_by = None;
            p.verifier_solution = None;
            p.verifiers.clear();
            state.insert_problem(&p).await;
            Json(ReviewResponse {
                success: true,
                message: "已退回至待审核".to_string(),
            })
        }
        "unpublish" => {
            if problem.status != "published" {
                return Json(ReviewResponse {
                    success: false,
                    message: "只能取消发布已发布题目".to_string(),
                });
            }
            // public_at is Option<DateTime>, use read-modify-write via insert_problem
            let mut p = state
                .problems
                .read()
                .await
                .get(&problem_id)
                .cloned()
                .unwrap();
            p.status = "approved".to_string();
            p.public_at = None;
            state.insert_problem(&p).await;
            Json(ReviewResponse {
                success: true,
                message: "已取消发布".to_string(),
            })
        }
        _ => Json(ReviewResponse {
            success: false,
            message: "无效操作".to_string(),
        }),
    };

    // Send notification to the problem author
    match action.as_str() {
        "approve" => {
            create_notification(
                &state,
                &author_id,
                "题目已批准",
                &format!("题目「{}」已通过审核", problem_title),
                Some("/problems"),
            )
            .await;
        }
        "publish" => {
            create_notification(
                &state,
                &author_id,
                "题目已发布",
                &format!("题目「{}」已成功发布", problem_title),
                Some("/problems"),
            )
            .await;
        }
        "reject" => {
            let reject_msg = format!("题目「{}」已被退回。\n退回理由：{}", problem_title, reason);
            create_notification(
                &state,
                &author_id,
                "题目已退回",
                &reject_msg,
                Some(&format!("/problems/{}", problem_id)),
            )
            .await;
        }
        "return" => {
            let return_msg = if reason.is_empty() {
                format!(
                    "题目「{}」已被退回至待审核状态，所有验题人题解和评论已清除",
                    problem_title
                )
            } else {
                format!(
                    "题目「{}」已被退回至待审核状态，所有验题人题解和评论已清除。\n退回理由：{}",
                    problem_title, reason
                )
            };
            create_notification(
                &state,
                &author_id,
                "题目已退回",
                &return_msg,
                Some("/problems"),
            )
            .await;
        }
        _ => {}
    }

    result
}

// ============== Claim Problem ==============

/// POST /api/problems/claim/:id
/// Multiple verifiers may claim the same approved problem. Each verifier keeps
/// their own solution and comments.
pub async fn claim_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ClaimResponse> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ClaimResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };
    if user.team_status != "joined" {
        return Json(ClaimResponse {
            success: false,
            message: "只有团队成员才能认领题目".to_string(),
        });
    }
    // Author cannot claim their own problem
    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ClaimResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };
    if problem.is_author(&user_id) {
        return Json(ClaimResponse {
            success: false,
            message: "不能认领自己的题目".to_string(),
        });
    }
    if problem.status != "approved" {
        return Json(ClaimResponse {
            success: false,
            message: "只能认领已批准的题目".to_string(),
        });
    }
    if problem.verifiers.iter().any(|v| v.user_id == user_id) {
        return Json(ClaimResponse {
            success: false,
            message: "您已认领该题目".to_string(),
        });
    }

    let first_verifier = problem.verifiers.is_empty();
    problem.verifiers.push(VerifierEntry {
        user_id: user_id.clone(),
        user_name: user.display_name.clone(),
        solution: None,
        comments: vec![],
        claimed_at: Utc::now(),
    });
    if first_verifier {
        problem.claimed_by = Some(user_id.clone());
    }
    state.insert_problem(&problem).await;

    Json(ClaimResponse {
        success: true,
        message: "认领成功".to_string(),
    })
}

/// POST /api/problems/unclaim/:id
pub async fn unclaim_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ClaimResponse> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ClaimResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };

    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ClaimResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };
    let before = problem.verifiers.len();
    problem.verifiers.retain(|v| v.user_id != user_id);
    if problem.verifiers.len() == before {
        return Json(ClaimResponse {
            success: false,
            message: "您不是该题目的验题人".to_string(),
        });
    }
    // Re-sync backward-compat mirror to the first remaining verifier (if any).
    match problem.verifiers.first() {
        Some(first) => {
            problem.claimed_by = Some(first.user_id.clone());
            problem.verifier_solution = first.solution.clone();
        }
        None => {
            problem.claimed_by = None;
            problem.verifier_solution = None;
        }
    }
    state.insert_problem(&problem).await;

    Json(ClaimResponse {
        success: true,
        message: "已取消认领".to_string(),
    })
}

// ============== Submit Verifier Solution ==============

/// POST /api/problems/verifier-solution/:id
pub async fn submit_verifier_solution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
    Json(payload): Json<VerifierSolutionPayload>,
) -> Json<ClaimResponse> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ClaimResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };

    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ClaimResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };
    let is_first = problem
        .verifiers
        .first()
        .is_some_and(|v| v.user_id == user_id);
    let mut found = false;
    for v in problem.verifiers.iter_mut() {
        if v.user_id == user_id {
            v.solution = if payload.solution.is_empty() {
                None
            } else {
                Some(payload.solution.clone())
            };
            found = true;
        }
    }
    if !found {
        return Json(ClaimResponse {
            success: false,
            message: "您不是该题目的验题人".to_string(),
        });
    }
    // Keep backward-compat mirror in sync if this is the first verifier.
    if is_first {
        problem.verifier_solution = if payload.solution.is_empty() {
            None
        } else {
            Some(payload.solution.clone())
        };
    }
    state.insert_problem(&problem).await;

    Json(ClaimResponse {
        success: true,
        message: "验题人题解已保存".to_string(),
    })
}

// ============== Verifier Comments ==============

/// POST /api/problems/verifier-comment/:id
/// A verifier posts a comment on the problem they are verifying.
pub async fn submit_verifier_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
    Json(payload): Json<VerifierCommentPayload>,
) -> Json<ClaimResponse> {
    let content = payload.content.trim();
    if content.is_empty() {
        return Json(ClaimResponse {
            success: false,
            message: "评论内容不能为空".to_string(),
        });
    }

    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ClaimResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };

    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ClaimResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };
    if !problem.verifiers.iter().any(|v| v.user_id == user_id) {
        return Json(ClaimResponse {
            success: false,
            message: "您不是该题目的验题人".to_string(),
        });
    }

    let comment = VerifierComment {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        user_name: user.display_name.clone(),
        content: content.to_string(),
        created_at: Utc::now(),
    };
    for v in problem.verifiers.iter_mut() {
        if v.user_id == user_id {
            v.comments.push(comment.clone());
        }
    }
    state.insert_problem(&problem).await;

    Json(ClaimResponse {
        success: true,
        message: "评论已发布".to_string(),
    })
}

// ============== Set Problem Visibility ==============

/// POST /api/problems/visibility/:id
/// Admin sets which members can see a pending problem's content + solution
pub async fn set_problem_visibility(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
    Json(payload): Json<VisibilityPayload>,
) -> Json<ClaimResponse> {
    let (_user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ClaimResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };
    if !check_permission(&state, &user, crate::types::perms::APPROVE_ALL_PROBLEMS).await {
        return Json(ClaimResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }

    // Validate the problem exists and is pending
    {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) if p.status == "pending" => {}
            Some(_) => {
                return Json(ClaimResponse {
                    success: false,
                    message: "只能设置待审核题目的可见性".to_string(),
                })
            }
            None => {
                return Json(ClaimResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    }

    // Only allow setting for actual 普通成员 (users.role == "member")
    let valid_ids: Vec<String> = {
        let members = state.team_members.read().await;
        let users = state.users.read().await;
        payload
            .user_ids
            .into_iter()
            .filter(|uid| {
                members.values().any(|m| &m.user_id == uid)
                    && users.get(uid).map(|u| u.role.as_str()) == Some("member")
            })
            .collect()
    };

    // visible_to is Vec<String>, use read-modify-write via insert_problem
    let mut p = state
        .problems
        .read()
        .await
        .get(&problem_id)
        .cloned()
        .unwrap();
    p.visible_to = valid_ids;
    state.insert_problem(&p).await;

    Json(ClaimResponse {
        success: true,
        message: "可见性已更新".to_string(),
    })
}

// ============== Admin: Get Pending Problems with Full Detail ==============

/// GET /api/problems/admin/pending
pub async fn get_pending_problems_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Vec<serde_json::Value>> {
    let (_user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(vec![]),
    };
    if !check_permission(&state, &user, crate::types::perms::APPROVE_ALL_PROBLEMS).await {
        return Json(vec![]);
    }

    let problems = state.problems.read().await;
    let contests = state.contests.read().await;
    let result: Vec<serde_json::Value> = problems
        .values()
        .filter(|p| p.status == "pending")
        .map(|p| {
            // Derive contest name from contest_id if available
            let contest_name = p
                .contest_id
                .as_ref()
                .and_then(|cid| contests.get(cid))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| p.contest.clone());
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "author_id": p.author_id,
                "author_name": p.author_name,
                "contest": contest_name,
                "difficulty": p.difficulty,
                "content": p.content,
                "solution": p.solution,
                "status": "pending",
                "created_at": p.created_at,
                "visible_to": p.visible_to,
                "claimed_by": p.claimed_by,
                "has_verifier_solution": p.verifier_solution.is_some(),
                "remark": p.remark,
            })
        })
        .collect();
    Json(result)
}

// ============== Admin: Get Members for Visibility ==============

/// GET /api/problems/admin/members
/// Returns only 普通成员 (role == "member") for visibility settings
pub async fn get_team_members_for_visibility(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Vec<serde_json::Value>> {
    let (_user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(vec![]),
    };
    // Admin only — for visibility settings
    if !check_permission(&state, &user, crate::types::perms::MANAGE_TEAM).await {
        return Json(vec![]);
    }

    let members = state.team_members.read().await;
    let users = state.users.read().await;
    let result: Vec<serde_json::Value> = members
        .values()
        .filter(|m| users.get(&m.user_id).map(|u| u.role.as_str()) == Some("member"))
        .map(|m| {
            let user_name = users
                .get(&m.user_id)
                .map(|u| u.display_name.clone())
                .unwrap_or_default();
            serde_json::json!({
                "user_id": m.user_id,
                "name": user_name,
            })
        })
        .collect();
    drop(members);
    drop(users);
    Json(result)
}

// ============== Edit Problem (Admin or Author) ==============

/// PUT /api/problems/:problem_id
/// Admin or the problem author can edit difficulty, content, and solution
pub async fn update_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
    Json(payload): Json<EditProblemPayload>,
) -> Json<ReviewResponse> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ReviewResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };
    let is_admin_user =
        check_permission(&state, &user, crate::types::perms::APPROVE_ALL_PROBLEMS).await;

    // Read problem and validate ownership
    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ReviewResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };

    // Check permission: author or admin
    if !problem.is_author(&user_id) && !is_admin_user {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }
    // 已退回题目仅作者可操作（管理员/超级管理员也只能查看，不能代操作）
    if problem.status == "returned" && !problem.is_author(&user_id) {
        return Json(ReviewResponse {
            success: false,
            message: "不能操作他人的已退回题目".to_string(),
        });
    }

    // Apply changes
    if let Some(val) = payload.title {
        problem.title = val;
    }
    if let Some(val) = payload.difficulty {
        problem.difficulty = val;
    }
    if let Some(val) = payload.content {
        problem.content = val;
    }
    if let Some(val) = payload.solution {
        problem.solution = if val.is_empty() { None } else { Some(val) };
    }
    if is_admin_user {
        if let Some(val) = payload.contest_id {
            problem.contest_id = match val {
                Some(s) if !s.is_empty() => Some(s),
                _ => None,
            };
        }
    }
    if is_admin_user {
        if let Some(val) = payload.link {
            problem.link = match val {
                Some(s) if !s.is_empty() => Some(s),
                _ => None,
            };
        }
    }
    if is_admin_user {
        if let Some(ref val) = payload.author_name {
            problem.author_name = val.clone();
        }
    }
    if is_admin_user {
        if let Some(val) = payload.author_id {
            if val == "unknown" {
                // Set to unknown author
                problem.author_id = "unknown".to_string();
                if payload.author_name.is_none() {
                    problem.author_name = "未知出题人".to_string();
                }
            } else if !val.is_empty() {
                // Assign to a specific member
                problem.author_id = val.clone();
                // Auto-update author_name to match user's display name if not explicitly set
                if payload.author_name.is_none() {
                    let users = state.users.read().await;
                    if let Some(u) = users.get(&val) {
                        problem.author_name = u.display_name.clone();
                    }
                    drop(users);
                }
            }
        }
    }
    if let Some(val) = payload.remark {
        problem.remark = if val.is_empty() { None } else { Some(val) };
    }

    state.insert_problem(&problem).await;

    Json(ReviewResponse {
        success: true,
        message: "已保存".to_string(),
    })
}

// ============== Resubmit Returned Problem (Author) ==============

/// POST /api/problems/:id/resubmit
/// Author resubmits a "returned" (已退回) problem back into "pending" (待审核).
pub async fn resubmit_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ReviewResponse> {
    let (user_id, _user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ReviewResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };

    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ReviewResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };

    // 已退回题目仅作者可再次提交（管理员/超级管理员也只能查看）
    if !problem.is_author(&user_id) {
        return Json(ReviewResponse {
            success: false,
            message: "只能操作自己的已退回题目".to_string(),
        });
    }
    if problem.status != "returned" {
        return Json(ReviewResponse {
            success: false,
            message: "只能再次提交已退回的题目".to_string(),
        });
    }

    problem.status = "pending".to_string();
    problem.remark = None;
    state.insert_problem(&problem).await;

    Json(ReviewResponse {
        success: true,
        message: "已重新提交，等待审核".to_string(),
    })
}

// ============== Delete Problem ==============

/// DELETE /api/problems/:id
/// Admin can delete any problem; author can delete their own pending/returned problem.
pub async fn delete_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ReviewResponse> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => {
            return Json(ReviewResponse {
                success: false,
                message: "未登录".to_string(),
            })
        }
    };
    let is_admin_user =
        check_permission(&state, &user, crate::types::perms::APPROVE_ALL_PROBLEMS).await;

    let problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => {
                return Json(ReviewResponse {
                    success: false,
                    message: "题目不存在".to_string(),
                })
            }
        }
    };

    // Author can delete their own pending/returned problem; admin can delete anything
    if !problem.is_author(&user_id) && !is_admin_user {
        return Json(ReviewResponse {
            success: false,
            message: "权限不足".to_string(),
        });
    }
    // 已退回题目仅作者可删除（管理员/超级管理员也只能查看）
    if problem.status == "returned" && !problem.is_author(&user_id) {
        return Json(ReviewResponse {
            success: false,
            message: "不能操作他人的已退回题目".to_string(),
        });
    }
    if !is_admin_user && problem.status != "pending" && problem.status != "returned" {
        return Json(ReviewResponse {
            success: false,
            message: "只能删除自己的待审核或已退回题目".to_string(),
        });
    }

    state.delete_problem_by_id(&problem_id).await;

    Json(ReviewResponse {
        success: true,
        message: "已删除题目".to_string(),
    })
}

// ============== Set Problem Contest ==============

/// POST /api/problems/contest/:problem_id
/// Admin only — sets which contest this problem belongs to
pub async fn set_problem_contest(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(problem_id): Path<String>,
    Json(payload): Json<SetProblemContestPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::APPROVE_ALL_PROBLEMS)
        .await?;

    let mut problem = {
        let problems = state.problems.read().await;
        match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => return Ok(json_error(ErrorCode::PROBLEM_NOT_FOUND, "题目不存在")),
        }
    };

    if let Some(cid) = &payload.contest_id {
        // Set contest
        let contests = state.contests.read().await;
        if let Some(c) = contests.get(cid) {
            problem.contest = c.name.clone();
            problem.contest_id = Some(cid.clone());
        } else {
            return Ok(json_error(ErrorCode::CONTEST_NOT_FOUND, "比赛不存在"));
        }
    } else {
        // Clear contest
        problem.contest = String::new();
        problem.contest_id = None;
    }

    state.insert_problem(&problem).await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "题目比赛已更新"}),
    ))
}
