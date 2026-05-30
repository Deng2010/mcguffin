// ============== Legacy Suggestion Compat Layer ==============
//
// These routes are kept for backward compatibility.
// They delegate to the unified discussion/post handlers.

use axum::http::HeaderMap;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::notifications::create_notification;
use crate::state::AppState;
use crate::types::*;
use crate::utils::{check_permission, resolve_user, AuthUser};

// ============== List Suggestions (backward compat) ==============

pub async fn get_suggestions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!([])),
    };
    let can_view_all = check_permission(&state, &user, crate::types::perms::VIEW_DISCUSSIONS).await
        || user.team_status == "joined";
    let posts = state.posts.read().await;
    let users = state.users.read().await;
    let mut result: Vec<serde_json::Value> = Vec::new();
    for p in posts.values() {
        if !p.tags.contains(&"建议".to_string()) && p.status.is_empty() {
            continue;
        }
        if !can_view_all && p.author_id != user_id {
            continue;
        }
        let author_name = users
            .get(&p.author_id)
            .map(|u| u.display_name.clone())
            .unwrap_or_else(|| p.author_name.clone());
        result.push(serde_json::json!({
            "id": p.id,
            "title": p.title,
            "content": p.content,
            "author_id": p.author_id,
            "author_name": author_name,
            "status": p.status,
            "replies": p.replies,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    result.sort_by(|a, b| {
        b["created_at"]
            .as_str()
            .unwrap_or("")
            .cmp(a["created_at"].as_str().unwrap_or(""))
    });
    Json(serde_json::json!(result))
}

// ============== Create Suggestion (backward compat) ==============

pub async fn create_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Forward to create_post with "建议" tag
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if user.team_status == "pending" {
        return Json(serde_json::json!({"success": false, "message": "待审核用户无法提交建议"}));
    }
    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
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
        tags: vec!["建议".to_string()],
        pinned: false,
        team_only: false,
        emoji: None,
        reactions: std::collections::HashMap::new(),
        replies: vec![],
        mentioned_user_ids: vec![],
        status: "open".to_string(),
        created_at: now,
        updated_at: now,
        visible_to: vec![],
        editable_by: vec![],
    };
    let post_id = post.id.clone();
    state.posts.write().await.insert(post_id, post);
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "建议已提交"}))
}

// ============== Get Suggestion Detail (backward compat) ==============

pub async fn get_suggestion_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let posts = state.posts.read().await;
    if let Some(p) = posts.get(&id) {
        let can_view_all = check_permission(&state, &user, crate::types::perms::VIEW_DISCUSSIONS)
            .await
            || user.team_status == "joined";
        if !can_view_all && p.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权查看"}));
        }
        let users = state.users.read().await;
        let author_name = users
            .get(&p.author_id)
            .map(|u| u.display_name.clone())
            .unwrap_or_else(|| p.author_name.clone());
        drop(users);
        return Json(serde_json::json!({
            "id": p.id,
            "title": p.title,
            "content": p.content,
            "author_id": p.author_id,
            "author_name": author_name,
            "status": p.status,
            "replies": p.replies,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}

// ============== Update Suggestion (backward compat) ==============

pub async fn update_suggestion(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_DISCUSSIONS)
        .await?;
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get_mut(&id) {
        let author_id = p.author_id.clone();
        let suggestion_title = p.title.clone();
        if let Some(new_status) = payload.get("status").and_then(|v| v.as_str()) {
            let valid = ["open", "in_progress", "resolved", "closed"];
            if !valid.contains(&new_status) {
                return Ok(Json(
                    serde_json::json!({"success": false, "message": "无效状态"}),
                ));
            }
            p.status = new_status.to_string();
        }
        p.updated_at = Utc::now();
        drop(posts);
        state.save().await;

        if let Some(new_status) = payload.get("status").and_then(|v| v.as_str()) {
            if new_status == "resolved" && author_id != auth.user_id {
                create_notification(
                    &state,
                    &author_id,
                    "建议已解决",
                    &format!("你的建议「{}」已被标记为已解决", suggestion_title),
                    Some(&format!("/post/{}", id)),
                )
                .await;
            } else if new_status == "closed" && author_id != auth.user_id {
                create_notification(
                    &state,
                    &author_id,
                    "建议已关闭",
                    &format!("你的建议「{}」已被关闭", suggestion_title),
                    Some(&format!("/post/{}", id)),
                )
                .await;
            }
        }
        return Ok(Json(
            serde_json::json!({"success": true, "message": "建议已更新"}),
        ));
    }
    Ok(Json(
        serde_json::json!({"success": false, "message": "建议不存在"}),
    ))
}

// ============== Reply to Suggestion (backward compat) ==============

pub async fn reply_to_suggestion(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_DISCUSSIONS)
        .await?;
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if content.trim().is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "回复不能为空"}),
        ));
    }
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get_mut(&id) {
        let author_id = p.author_id.clone();
        let suggestion_title = p.title.clone();
        let reply = PostReply {
            id: Uuid::new_v4().to_string(),
            author_id: auth.user_id.clone(),
            author_name: auth.user.display_name,
            content: content.trim().to_string(),
            created_at: Utc::now(),
            reactions: std::collections::HashMap::new(),
            parent_id: None,
            reply_to: None,
        };
        p.replies.push(reply);
        p.updated_at = Utc::now();
        drop(posts);
        state.save().await;
        if author_id != auth.user_id {
            create_notification(
                &state,
                &author_id,
                "建议有新回复",
                &format!("你的建议「{}」收到了新回复", suggestion_title),
                Some(&format!("/post/{}", id)),
            )
            .await;
        }
        return Ok(Json(
            serde_json::json!({"success": true, "message": "回复成功"}),
        ));
    }
    Ok(Json(
        serde_json::json!({"success": false, "message": "建议不存在"}),
    ))
}

// ============== Delete Reply ==============

pub async fn delete_suggestion_reply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((suggestion_id, reply_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let can_manage = check_permission(&state, &user, crate::types::perms::MANAGE_DISCUSSIONS).await;
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get_mut(&suggestion_id) {
        if let Some(reply) = p.replies.iter().find(|r| r.id == reply_id) {
            if !can_manage && reply.author_id != user_id {
                return Json(serde_json::json!({"success": false, "message": "无权删除"}));
            }
        } else {
            return Json(serde_json::json!({"success": false, "message": "回复不存在"}));
        }
        p.replies.retain(|r| r.id != reply_id);
        drop(posts);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "回复已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}

// ============== Delete Suggestion ==============

pub async fn delete_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let can_manage = check_permission(&state, &user, crate::types::perms::MANAGE_DISCUSSIONS).await;
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get(&id) {
        if !can_manage && p.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权删除"}));
        }
        posts.remove(&id);
        drop(posts);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "建议已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}
