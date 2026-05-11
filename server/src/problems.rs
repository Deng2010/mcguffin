use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum::http::HeaderMap;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::AppState;
use crate::types::*;
use crate::utils::{resolve_user, is_admin, is_team_member};
use crate::notifications::create_notification;

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
    let is_admin_user = if let Some((id, _)) = &current_user {
        is_admin(&state, id).await
    } else {
        false
    };
    let is_member = if let Some((id, _)) = &current_user {
        is_team_member(&state, id).await
    } else {
        false
    };
    let current_uid = current_user.as_ref().map(|(id, _)| id.clone());
    let current_display_name = current_user.as_ref().map(|(_, u)| u.display_name.clone());

    let all_problems = state.problems.read().await;
    let show_all = params.get("all").map(|v| v == "true").unwrap_or(false);

    // Search/filter query params
    let search_q = params.get("search").map(|v| v.to_lowercase()).filter(|v| !v.is_empty());
    let diff_filter = params.get("difficulty").filter(|v| !v.is_empty()).cloned();
    let status_filter = params.get("status").filter(|v| !v.is_empty()).cloned();
    let author_filter = params.get("author").map(|v| v.to_lowercase()).filter(|v| !v.is_empty());

    let contests = state.contests.read().await;

    let problems: Vec<ProblemListItem> = all_problems
        .values()
        .filter(|p| {
            // Always exclude rejected problems (reject = delete, but guard against stale data)
            if p.status == "rejected" {
                return false;
            }
            if is_admin_user {
                // Admin sees all by default
                if show_all {
                    return true;
                }
            } else if is_member {
                // Members see: published, approved, pending, their own problems,
                // and problems where they're in visible_to
                // Also match author by display_name
                let is_author = current_uid.as_ref().map_or(false, |uid| p.author_id == *uid)
                    || current_display_name.as_ref().map_or(false, |dn| p.author_name == *dn);
                let ok = p.status == "published"
                    || p.status == "approved"
                    || p.status == "pending"
                    || is_author
                    || current_uid.as_ref().map_or(false, |uid| p.visible_to.contains(uid));
                if !ok {
                    return false;
                }
            } else {
                // Guests (including unauthenticated): only published
                if p.status != "published" {
                    return false;
                }
            }

            // Apply search/filter query params
            if let Some(q) = &search_q {
                if !p.title.to_lowercase().contains(q) {
                    return false;
                }
            }
            if let Some(d) = diff_filter.as_deref() {
                if p.difficulty != d {
                    return false;
                }
            }
            if let Some(s) = status_filter.as_deref() {
                if p.status != s {
                    return false;
                }
            }
            if let Some(a) = &author_filter {
                if !p.author_name.to_lowercase().contains(a) {
                    return false;
                }
            }
            true
        })
        .map(|p| {
            // Derive contest name from contest_id if available (handles contest rename)
            let contest_name = p.contest_id.as_ref()
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
                link: p.link.clone(),
                remark: if p.status == "pending" && (is_admin_user || current_uid.as_ref().map_or(false, |uid| p.author_id == *uid) || current_display_name.as_ref().map_or(false, |dn| p.author_name == *dn)) {
                    p.remark.clone()
                } else {
                    None
                },
            }
        })
        .collect();

    Json(problems)
}

// ============== Get Problem Detail ==============

/// GET /api/problems/:id
/// Returns full problem (content, solution if permitted)
pub async fn get_problem_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<serde_json::Value> {
    let current_user = resolve_user(&state, &headers).await;

    let problems = state.problems.read().await;
    let problem = match problems.get(&problem_id) {
        Some(p) => p.clone(),
        None => return Json(serde_json::json!({"error": "题目不存在"})),
    };
    drop(problems);

    // Check admin status properly
    let is_admin_user = if let Some((id, _)) = &current_user {
        is_admin(&state, id).await
    } else {
        false
    };
    let is_member_user = if let Some((id, _)) = &current_user {
        is_team_member(&state, id).await
    } else {
        false
    };
    let current_display_name = current_user.as_ref().map(|(_, u)| u.display_name.clone());
    let is_author = current_user.as_ref().map_or(false, |(uid, _)| problem.author_id == *uid)
        || current_display_name.as_ref().map_or(false, |dn| problem.author_name == *dn);

    // Permission check
    let can_view = match problem.status.as_str() {
        "published" => true,
        "approved" => is_member_user || is_admin_user,
        "pending" => {
            if is_admin_user { true }
            else if let Some((uid, _)) = &current_user {
                // Author by user_id or display_name match
                is_author || problem.visible_to.contains(uid)
            } else { false }
        }
        _ => false,
    };
    if !can_view {
        return Json(serde_json::json!({"error": "无权限查看"}));
    }

    // Determine what to show
    let mut show_solution = match problem.status.as_str() {
        "published" => is_member_user || is_admin_user,
        "approved" => is_member_user || is_admin_user,
        "pending" => is_admin_user || is_author,
        _ => false,
    };
    // User who claimed this problem cannot see the author's solution (impartiality)
    if let Some((uid, _)) = &current_user {
        if problem.claimed_by.as_ref() == Some(uid) {
            show_solution = false;
        }
    }
    let in_visible_to = current_user.as_ref().map_or(false, |(uid, _)| problem.visible_to.contains(uid));
    let show_content = match problem.status.as_str() {
        "published" => true,
        "approved" => true,
        "pending" => is_admin_user || is_author || in_visible_to,
        _ => false,
    };

    let contests = state.contests.read().await;
    // Derive contest name from contest_id if available
    let contest_name = problem.contest_id.as_ref()
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
    // All members can see the verifier's solution
    if let Some(vs) = &problem.verifier_solution {
        if is_member_user || is_admin_user {
            resp["verifier_solution"] = serde_json::Value::String(vs.clone());
        }
    }
    // Only the actual verifier can submit/edit their solution
    if let Some((user_id, _)) = &current_user {
        if problem.claimed_by.as_ref() == Some(user_id) {
            resp["can_submit_verifier_solution"] = serde_json::Value::Bool(true);
        }
    }

    Json(resp)
}

// ============== Submit Problem ==============

pub async fn submit_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SubmitProblemPayload>,
) -> Json<SubmitResponse> {
    let user = match resolve_user(&state, &headers).await {
        Some((_, u)) if u.team_status == "joined" => u,
        Some(_) => return Json(SubmitResponse { success: false, message: "只有团队成员才能投稿".to_string(), problem_id: None }),
        None => return Json(SubmitResponse { success: false, message: "未登录".to_string(), problem_id: None }),
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
        visible_to: vec![],
        link: payload.link,
        remark: payload.remark,
    };

    let pid = problem.id.clone();
    state.problems.write().await.insert(problem.id.clone(), problem);
    state.save().await;

    Json(SubmitResponse { success: true, message: "提交成功，等待审核".to_string(), problem_id: Some(pid) })
}

// ============== Review Problem ==============

/// POST /api/problems/review/:problem_id/:action
/// action = "approve" | "reject" | "publish"
pub async fn review_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((problem_id, action)): Path<(String, String)>,
) -> Json<ReviewResponse> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(ReviewResponse { success: false, message: "未登录".to_string() }),
    };
    if !is_admin(&state, &user_id).await {
        return Json(ReviewResponse { success: false, message: "权限不足".to_string() });
    }

    // For 'reject', delete the problem entirely (not just mark as rejected)
    if action == "reject" {
        let problems = state.problems.read().await;
        let problem = match problems.get(&problem_id) {
            Some(p) => p.clone(),
            None => return Json(ReviewResponse { success: false, message: "题目不存在".to_string() }),
        };
        if problem.status != "pending" {
            return Json(ReviewResponse { success: false, message: "只能拒绝待审核题目".to_string() });
        }
        let author_id = problem.author_id.clone();
        let problem_title = problem.title.clone();
        drop(problems);

        state.problems.write().await.remove(&problem_id);
        state.save().await;

        create_notification(
            &state,
            &author_id,
            "题目未通过",
            &format!("题目「{}」未通过审核，已被删除", problem_title),
            Some("/problems"),
        ).await;

        return Json(ReviewResponse { success: true, message: "已拒绝题目，数据已删除".to_string() });
    }

    let mut problems = state.problems.write().await;
    let problem = match problems.get_mut(&problem_id) {
        Some(p) => p,
        None => return Json(ReviewResponse { success: false, message: "题目不存在".to_string() }),
    };

    // Capture info for notification before modifying
    let author_id = problem.author_id.clone();
    let problem_title = problem.title.clone();

    let result = match action.as_str() {
        "approve" => {
            if problem.status != "pending" {
                return Json(ReviewResponse { success: false, message: "只能审核待审核题目".to_string() });
            }
            problem.status = "approved".to_string();
            Json(ReviewResponse { success: true, message: "已批准题目".to_string() })
        }
        "publish" => {
            if problem.status != "approved" {
                return Json(ReviewResponse { success: false, message: "只能发布已批准题目".to_string() });
            }
            if problem.link.is_none() || problem.link.as_deref() == Some("") {
                return Json(ReviewResponse { success: false, message: "发布前请先设置题目链接".to_string() });
            }
            problem.status = "published".to_string();
            problem.public_at = Some(Utc::now());
            Json(ReviewResponse { success: true, message: "已发布题目".to_string() })
        }
        "return" => {
            if problem.status != "approved" {
                return Json(ReviewResponse { success: false, message: "只能退回已批准题目".to_string() });
            }
            problem.status = "pending".to_string();
            // Clear claim info since only approved problems can be claimed
            problem.claimed_by = None;
            problem.verifier_solution = None;
            Json(ReviewResponse { success: true, message: "已退回至待审核".to_string() })
        }
        "unpublish" => {
            if problem.status != "published" {
                return Json(ReviewResponse { success: false, message: "只能取消发布已发布题目".to_string() });
            }
            problem.status = "approved".to_string();
            problem.public_at = None;
            Json(ReviewResponse { success: true, message: "已取消发布".to_string() })
        }
        _ => Json(ReviewResponse { success: false, message: "无效操作".to_string() }),
    };

    drop(problems);
    state.save().await;

    // Send notification to the problem author
    match action.as_str() {
        "approve" => {
            create_notification(
                &state,
                &author_id,
                "题目已批准",
                &format!("题目「{}」已通过审核", problem_title),
                Some("/problems"),
            ).await;
        }
        "publish" => {
            create_notification(
                &state,
                &author_id,
                "题目已发布",
                &format!("题目「{}」已成功发布", problem_title),
                Some("/problems"),
            ).await;
        }
        _ => {}
    }

    result
}

// ============== Claim Problem ==============

/// POST /api/problems/claim/:id
pub async fn claim_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ClaimResponse> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(ClaimResponse { success: false, message: "未登录".to_string() }),
    };
    if !is_team_member(&state, &user_id).await {
        return Json(ClaimResponse { success: false, message: "只有团队成员才能认领题目".to_string() });
    }
    // Author cannot claim their own problem
    let problems = state.problems.read().await;
    let problem = match problems.get(&problem_id) {
        Some(p) => p,
        None => return Json(ClaimResponse { success: false, message: "题目不存在".to_string() }),
    };
    if problem.author_id == user_id {
        return Json(ClaimResponse { success: false, message: "不能认领自己的题目".to_string() });
    }
    if problem.status != "approved" {
        return Json(ClaimResponse { success: false, message: "只能认领已批准的题目".to_string() });
    }
    if problem.claimed_by.is_some() {
        return Json(ClaimResponse { success: false, message: "该题目已被认领".to_string() });
    }
    drop(problems);

    let mut problems = state.problems.write().await;
    if let Some(p) = problems.get_mut(&problem_id) {
        p.claimed_by = Some(user_id);
    }
    drop(problems);
    state.save().await;

    Json(ClaimResponse { success: true, message: "认领成功".to_string() })
}

// ============== Unclaim Problem ==============

/// POST /api/problems/unclaim/:id
pub async fn unclaim_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ClaimResponse> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(ClaimResponse { success: false, message: "未登录".to_string() }),
    };

    let mut problems = state.problems.write().await;
    let problem = match problems.get_mut(&problem_id) {
        Some(p) => p,
        None => return Json(ClaimResponse { success: false, message: "题目不存在".to_string() }),
    };
    if problem.claimed_by.as_deref() != Some(&user_id) {
        return Json(ClaimResponse { success: false, message: "您不是该题目的验题人".to_string() });
    }
    problem.claimed_by = None;
    problem.verifier_solution = None;
    drop(problems);
    state.save().await;

    Json(ClaimResponse { success: true, message: "已取消认领".to_string() })
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
        None => return Json(ClaimResponse { success: false, message: "未登录".to_string() }),
    };

    let mut problems = state.problems.write().await;
    let problem = match problems.get_mut(&problem_id) {
        Some(p) => p,
        None => return Json(ClaimResponse { success: false, message: "题目不存在".to_string() }),
    };
    if problem.claimed_by.as_deref() != Some(&user_id) {
        return Json(ClaimResponse { success: false, message: "您不是该题目的验题人".to_string() });
    }
    problem.verifier_solution = Some(payload.solution);
    drop(problems);
    state.save().await;

    Json(ClaimResponse { success: true, message: "验题人题解已保存".to_string() })
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
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(ClaimResponse { success: false, message: "未登录".to_string() }),
    };
    if !is_admin(&state, &user_id).await {
        return Json(ClaimResponse { success: false, message: "权限不足".to_string() });
    }

    let mut problems = state.problems.write().await;
    let problem = match problems.get_mut(&problem_id) {
        Some(p) => p,
        None => return Json(ClaimResponse { success: false, message: "题目不存在".to_string() }),
    };
    if problem.status != "pending" {
        return Json(ClaimResponse { success: false, message: "只能设置待审核题目的可见性".to_string() });
    }

    // Only allow setting for actual 普通成员 (users.role == "member")
    let members = state.team_members.read().await;
    let users = state.users.read().await;
    let valid_ids: Vec<String> = payload.user_ids.into_iter()
        .filter(|uid| {
            members.values().any(|m| &m.user_id == uid)
                && users.get(uid).map(|u| u.role.as_str()) == Some("member")
        })
        .collect();
    drop(members);
    drop(users);

    problem.visible_to = valid_ids;
    drop(problems);
    state.save().await;

    Json(ClaimResponse { success: true, message: "可见性已更新".to_string() })
}

// ============== Admin: Get Pending Problems with Full Detail ==============

/// GET /api/problems/admin/pending
pub async fn get_pending_problems_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Vec<serde_json::Value>> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(vec![]),
    };
    if !is_admin(&state, &user_id).await {
        return Json(vec![]);
    }

    let problems = state.problems.read().await;
    let contests = state.contests.read().await;
    let result: Vec<serde_json::Value> = problems
        .values()
        .filter(|p| p.status == "pending")
        .map(|p| {
            // Derive contest name from contest_id if available
            let contest_name = p.contest_id.as_ref()
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
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(vec![]),
    };
    // Admin only — for visibility settings
    if !is_admin(&state, &user_id).await {
        return Json(vec![]);
    }

    let members = state.team_members.read().await;
    let users = state.users.read().await;
    let result: Vec<serde_json::Value> = members
        .values()
        .filter(|m| users.get(&m.user_id).map(|u| u.role.as_str()) == Some("member"))
        .map(|m| {
            let user_name = users.get(&m.user_id).map(|u| u.display_name.clone()).unwrap_or_default();
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
    let (user_id, _user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(ReviewResponse { success: false, message: "未登录".to_string() }),
    };
    let is_admin_user = is_admin(&state, &user_id).await;

    let mut problems = state.problems.write().await;
    let problem = match problems.get_mut(&problem_id) {
        Some(p) => p,
        None => return Json(ReviewResponse { success: false, message: "题目不存在".to_string() }),
    };

    // Check permission: author or admin
    if problem.author_id != user_id && !is_admin_user {
        return Json(ReviewResponse { success: false, message: "权限不足".to_string() });
    }

    // Apply changes
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

    drop(problems);
    state.save().await;

    Json(ReviewResponse { success: true, message: "已保存".to_string() })
}

// ============== Delete Problem (Admin only) ==============

/// DELETE /api/problems/:id
pub async fn delete_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
) -> Json<ReviewResponse> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(ReviewResponse { success: false, message: "未登录".to_string() }),
    };
    if !is_admin(&state, &user_id).await {
        return Json(ReviewResponse { success: false, message: "权限不足".to_string() });
    }

    let mut problems = state.problems.write().await;
    if problems.remove(&problem_id).is_none() {
        return Json(ReviewResponse { success: false, message: "题目不存在".to_string() });
    }
    drop(problems);
    state.save().await;

    Json(ReviewResponse { success: true, message: "已删除题目".to_string() })
}

// ============== Set Problem Contest ==============

/// POST /api/problems/contest/:problem_id
/// Admin only — sets which contest this problem belongs to
pub async fn set_problem_contest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(problem_id): Path<String>,
    Json(payload): Json<SetProblemContestPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let mut problems = state.problems.write().await;
    let problem = match problems.get_mut(&problem_id) {
        Some(p) => p,
        None => return Json(serde_json::json!({"success": false, "message": "题目不存在"})),
    };

    if let Some(cid) = &payload.contest_id {
        // Set contest
        let contests = state.contests.read().await;
        if let Some(c) = contests.get(cid) {
            problem.contest = c.name.clone();
            problem.contest_id = Some(cid.clone());
        } else {
            return Json(serde_json::json!({"success": false, "message": "比赛不存在"}));
        }
    } else {
        // Clear contest
        problem.contest = String::new();
        problem.contest_id = None;
    }
    drop(problems);
    state.save().await;

    Json(serde_json::json!({"success": true, "message": "题目比赛已更新"}))
}
