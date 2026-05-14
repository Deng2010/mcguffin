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

// ============== List Announcements ==============

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
        if p.kind != "announcement" { continue; }
        if !can_see_all && !p.public { continue; }
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
            "public": p.public,
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

// ============== Create Announcement ==============

pub async fn create_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateAnnouncementPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }
    if payload.title.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
    }
    let now = Utc::now();
    let post = Post {
        id: Uuid::new_v4().to_string(),
        kind: "announcement".to_string(),
        title: payload.title.trim().to_string(),
        content: payload.content,
        author_id: user_id,
        author_name: user.display_name,
        pinned: payload.pinned,
        public: payload.public,
        created_at: now,
        updated_at: now,
        tags: vec![],
        team_only: false,
        emoji: None,
        reactions: std::collections::HashMap::new(),
        replies: vec![],
        mentioned_user_ids: vec![],
        status: String::new(),
    };
    let post_id = post.id.clone();
    state.posts.write().await.insert(post_id, post);
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "公告已发布"}))
}

// ============== Get Announcement Detail ==============

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
        if p.kind != "announcement" {
            return Json(serde_json::json!({"success": false, "message": "公告不存在"}));
        }
        // Check visibility
        if !is_admin_user && !is_team && !p.public {
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
            "public": p.public,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    Json(serde_json::json!({"success": false, "message": "公告不存在"}))
}

// ============== Update Announcement ==============

pub async fn update_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAnnouncementPayload>,
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
        if p.kind != "announcement" {
            return Json(serde_json::json!({"success": false, "message": "公告不存在"}));
        }
        if let Some(ref title) = payload.title {
            if title.trim().is_empty() {
                return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
            }
            p.title = title.trim().to_string();
        }
        if let Some(ref content) = payload.content {
            p.content = content.clone();
        }
        if let Some(pinned) = payload.pinned {
            p.pinned = pinned;
        }
        if let Some(public) = payload.public {
            p.public = public;
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
    if let Some(p) = posts.get(&id) {
        if p.kind != "announcement" {
            return Json(serde_json::json!({"success": false, "message": "公告不存在"}));
        }
        posts.remove(&id);
        drop(posts);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "公告已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "公告不存在"}))
}
