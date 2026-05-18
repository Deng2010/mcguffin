// ============== Legacy Announcement Compat Layer ==============
//
// These routes are kept for backward compatibility.
// They delegate to the unified post handlers.

use axum::{
    extract::{Path, State},
    Json,
};
use axum::http::HeaderMap;
use chrono::Utc;
use uuid::Uuid;

use crate::state::AppState;
use crate::types::*;
use crate::utils::{is_admin, is_team_member, resolve_user};

// ============== List Announcements (backward compat) ==============

pub async fn get_announcements(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let can_see_all = if let Some((user_id, _)) = resolve_user(&state, &headers).await {
        is_admin(&state, &user_id).await || is_team_member(&state, &user_id).await
    } else {
        false
    };
    let posts = state.posts.read().await;
    let users = state.users.read().await;
    let mut result: Vec<serde_json::Value> = Vec::new();
    for p in posts.values() {
        if !p.tags.contains(&"公告".to_string()) { continue; }
        if !can_see_all && p.team_only { continue; }
        let author_name = users.get(&p.author_id)
            .map(|u| u.display_name.clone())
            .unwrap_or_else(|| p.author_name.clone());
        result.push(serde_json::json!({
            "id": p.id,
            "title": p.title,
            "content": p.content,
            "author_id": p.author_id,
            "author_name": author_name,
            "pinned": p.pinned,
            "public": !p.team_only,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    result.sort_by(|a, b| {
        let a_pinned = a["pinned"].as_bool().unwrap_or(false);
        let b_pinned = b["pinned"].as_bool().unwrap_or(false);
        b_pinned.cmp(&a_pinned)
            .then_with(|| {
                let a_t = a["created_at"].as_str().unwrap_or("");
                let b_t = b["created_at"].as_str().unwrap_or("");
                b_t.cmp(a_t)
            })
    });
    Json(serde_json::json!(result))
}

// ============== Create Announcement (backward compat) ==============

pub async fn create_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }
    let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let content = payload.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let pinned = payload.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_public = payload.get("public").and_then(|v| v.as_bool()).unwrap_or(true);
    if title.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
    }
    let now = Utc::now();
    let post = Post {
        id: Uuid::new_v4().to_string(),
        title: title.trim().to_string(),
        content,
        author_id: user_id,
        author_name: user.display_name,
        tags: vec!["公告".to_string()],
        pinned,
        team_only: !is_public,
        emoji: None,
        reactions: std::collections::HashMap::new(),
        replies: vec![],
        mentioned_user_ids: vec![],
        status: String::new(),
        created_at: now,
        updated_at: now,
    };
    let post_id = post.id.clone();
    state.posts.write().await.insert(post_id, post);
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "公告已发布"}))
}

// ============== Get Announcement Detail (backward compat) ==============

pub async fn get_announcement_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let auth_info = resolve_user(&state, &headers).await;
    let is_admin_user = if let Some((ref uid, _)) = auth_info { is_admin(&state, uid).await } else { false };
    let is_team = if let Some((ref uid, _)) = auth_info { is_team_member(&state, uid).await } else { false };
    let posts = state.posts.read().await;
    if let Some(p) = posts.get(&id) {
        if !is_admin_user && !is_team && p.team_only {
            return Json(serde_json::json!({"success": false, "message": "无权查看"}));
        }
        let users = state.users.read().await;
        let author_name = users.get(&p.author_id)
            .map(|u| u.display_name.clone())
            .unwrap_or_else(|| p.author_name.clone());
        drop(users);
        return Json(serde_json::json!({
            "id": p.id,
            "title": p.title,
            "content": p.content,
            "author_id": p.author_id,
            "author_name": author_name,
            "pinned": p.pinned,
            "public": !p.team_only,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    Json(serde_json::json!({"success": false, "message": "公告不存在"}))
}

// ============== Update Announcement (backward compat) ==============

pub async fn update_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get_mut(&id) {
        if let Some(title) = payload.get("title").and_then(|v| v.as_str()) {
            if title.trim().is_empty() {
                return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
            }
            p.title = title.trim().to_string();
        }
        if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
            p.content = content.to_string();
        }
        if let Some(pinned) = payload.get("pinned").and_then(|v| v.as_bool()) {
            p.pinned = pinned;
        }
        if let Some(is_public) = payload.get("public").and_then(|v| v.as_bool()) {
            p.team_only = !is_public;
        }
        p.updated_at = Utc::now();
        drop(posts);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "公告已更新"}));
    }
    Json(serde_json::json!({"success": false, "message": "公告不存在"}))
}

// ============== Delete Announcement ==============

pub async fn delete_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }
    let mut posts = state.posts.write().await;
    if posts.contains_key(&id) {
        posts.remove(&id);
        drop(posts);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "公告已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "公告不存在"}))
}
