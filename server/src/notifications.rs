use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::state::AppState;
use crate::types::*;
use crate::utils::resolve_user;

// ============== Get Notifications ==============

pub async fn get_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };

    let notifications = state.notifications.read().await;
    let mut user_notifications: Vec<&Notification> = notifications
        .values()
        .filter(|n| n.user_id == user_id)
        .collect();
    user_notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let unread_count = user_notifications.iter().filter(|n| !n.read).count();

    Json(serde_json::json!(NotificationResponse {
        notifications: user_notifications.into_iter().cloned().collect(),
        unread_count,
    }))
}

// ============== Mark Notification as Read ==============

pub async fn mark_notification_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };

    let mut notifications = state.notifications.write().await;
    if let Some(n) = notifications.get_mut(&notification_id) {
        if n.user_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权操作"}));
        }
        n.read = true;
        drop(notifications);
        state.save().await;
        Json(serde_json::json!({"success": true, "message": "已标记为已读"}))
    } else {
        Json(serde_json::json!({"success": false, "message": "通知不存在"}))
    }
}

// ============== Mark All Notifications as Read ==============

pub async fn mark_all_notifications_read(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };

    let mut notifications = state.notifications.write().await;
    let mut changed = false;
    for n in notifications.values_mut() {
        if n.user_id == user_id && !n.read {
            n.read = true;
            changed = true;
        }
    }
    drop(notifications);
    if changed {
        state.save().await;
    }
    Json(serde_json::json!({"success": true, "message": "已全部标记为已读"}))
}

// ============== Helper: Create a Notification ==============

pub async fn create_notification(
    state: &AppState,
    user_id: &str,
    title: &str,
    body: &str,
    link: Option<&str>,
) {
    let notification = Notification {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        read: false,
        created_at: Utc::now(),
        link: link.map(|l| l.to_string()),
    };
    state.notifications.write().await.insert(notification.id.clone(), notification);
    state.save().await;
}
