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
    // Determine visibility: team members see all, others see only public
    let can_see_all = if let Some((user_id, _)) = resolve_user(&state, &headers).await {
        is_admin(&state, &user_id).await || is_team_member(&state, &user_id).await
    } else {
        false
    };
    let announcements = state.announcements.read().await;
    let mut result: Vec<&Announcement> = announcements.values()
        .filter(|a| can_see_all || a.public)
        .collect();
    // Pinned first, then by created_at descending
    result.sort_by(|a, b| {
        b.pinned.cmp(&a.pinned)
            .then_with(|| b.created_at.cmp(&a.created_at))
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
    let announcement = Announcement {
        id: Uuid::new_v4().to_string(),
        title: payload.title.trim().to_string(),
        content: payload.content,
        author_id: user_id,
        author_name: user.display_name,
        pinned: payload.pinned,
        public: payload.public,
        created_at: now,
        updated_at: now,
    };
    state.announcements.write().await.insert(announcement.id.clone(), announcement);
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "公告已发布"}))
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
    let mut announcements = state.announcements.write().await;
    if let Some(a) = announcements.get_mut(&id) {
        if let Some(ref title) = payload.title {
            if title.trim().is_empty() {
                return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
            }
            a.title = title.trim().to_string();
        }
        if let Some(ref content) = payload.content {
            a.content = content.clone();
        }
        if let Some(pinned) = payload.pinned {
            a.pinned = pinned;
        }
        if let Some(public) = payload.public {
            a.public = public;
        }
        a.updated_at = Utc::now();
        drop(announcements);
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
    let mut announcements = state.announcements.write().await;
    if announcements.remove(&id).is_some() {
        drop(announcements);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "公告已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "公告不存在"}))
}
