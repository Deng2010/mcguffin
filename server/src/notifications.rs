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

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: String,
    user_id: String,
    title: String,
    body: String,
    read: i32,
    created_at: String,
    link: Option<String>,
}

// ============== Get Notifications ==============

pub async fn get_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };

    let result: Result<Vec<NotificationRow>, _> = sqlx::query_as(
        "SELECT id, user_id, title, body, read, created_at, link \
         FROM notifications WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&user_id)
    .fetch_all(&state.db)
    .await;

    match result {
        Ok(rows) => {
            let unread_count = rows.iter().filter(|n| n.read == 0).count();
            let notifications: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|n| {
                    serde_json::json!({
                        "id": n.id,
                        "user_id": n.user_id,
                        "title": n.title,
                        "body": n.body,
                        "read": n.read != 0,
                        "created_at": n.created_at,
                        "link": n.link,
                    })
                })
                .collect();
            Json(serde_json::json!({
                "notifications": notifications,
                "unread_count": unread_count,
            }))
        }
        Err(_) => {
            // Fallback to HashMap read
            let notifications = state.notifications.read().await;
            let mut user_notifications: Vec<&Notification> = notifications
                .values()
                .filter(|n| n.user_id == user_id)
                .collect();
            user_notifications.sort_by_key(|b| std::cmp::Reverse(b.created_at));
            let unread_count = user_notifications.iter().filter(|n| !n.read).count();
            Json(serde_json::json!(NotificationResponse {
                notifications: user_notifications.into_iter().cloned().collect(),
                unread_count,
            }))
        }
    }
}

// ============== Mark Notification as Read ==============

pub async fn mark_notification_read(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };

    // 验证所有权
    let owned = {
        let n = state
            .notifications
            .read()
            .await
            .get(&notification_id)
            .cloned();
        n.map(|n| n.user_id == user_id).unwrap_or(false)
    };
    if !owned {
        return Json(serde_json::json!({"success": false, "message": "通知不存在或无权操作"}));
    }

    state.mark_notification_read(&notification_id).await;
    Json(serde_json::json!({"success": true, "message": "已标记为已读"}))
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

    state.mark_all_user_notifications_read(&user_id).await;

    Json(serde_json::json!({"success": true, "message": "已标记所有通知为已读"}))
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
    state.insert_notification(&notification).await;
}
