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
use crate::notifications::create_notification;

// ============== List Suggestions ==============

pub async fn get_suggestions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!([])),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let is_team = is_team_member(&state, &user_id).await;
    let posts = state.posts.read().await;
    let users = state.users.read().await;
    let mut result: Vec<serde_json::Value> = Vec::new();
    for p in posts.values() {
        if p.kind != "suggestion" { continue; }
        if !is_admin_user && !is_team && p.author_id != user_id { continue; }
        let author_name = users.get(&p.author_id)
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
        b["created_at"].as_str().unwrap_or("").cmp(&a["created_at"].as_str().unwrap_or(""))
    });
    Json(serde_json::json!(result))
}

// ============== Create Suggestion ==============

pub async fn create_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSuggestionPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if user.team_status == "pending" {
        return Json(serde_json::json!({"success": false, "message": "待审核用户无法提交建议"}));
    }
    if payload.title.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
    }
    let now = Utc::now();
    let post = Post {
        id: Uuid::new_v4().to_string(),
        kind: "suggestion".to_string(),
        title: payload.title.trim().to_string(),
        content: payload.content,
        author_id: user_id,
        author_name: user.display_name,
        status: "open".to_string(),
        replies: vec![],
        created_at: now,
        updated_at: now,
        tags: vec![],
        pinned: false,
        team_only: false,
        emoji: None,
        reactions: std::collections::HashMap::new(),
        mentioned_user_ids: vec![],
        public: true,
    };
    let post_id = post.id.clone();
    state.posts.write().await.insert(post_id, post);
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "建议已提交"}))
}

// ============== Get Suggestion Detail ==============

pub async fn get_suggestion_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let posts = state.posts.read().await;
    if let Some(p) = posts.get(&id) {
        if p.kind != "suggestion" {
            return Json(serde_json::json!({"success": false, "message": "建议不存在"}));
        }
        let is_admin_user = is_admin(&state, &user_id).await;
        let is_team = is_team_member(&state, &user_id).await;
        if !is_admin_user && !is_team && p.author_id != user_id {
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
            "status": p.status,
            "replies": p.replies,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}

// ============== Update Suggestion ==============

pub async fn update_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSuggestionPayload>,
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
        if p.kind != "suggestion" {
            return Json(serde_json::json!({"success": false, "message": "建议不存在"}));
        }
        let _old_status = p.status.clone();
        let author_id = p.author_id.clone();
        let suggestion_title = p.title.clone();

        if let Some(ref new_status) = payload.status {
            let valid = ["open", "in_progress", "resolved", "closed"];
            if !valid.contains(&new_status.as_str()) {
                return Json(serde_json::json!({"success": false, "message": "无效状态"}));
            }
            p.status = new_status.clone();
        }
        p.updated_at = Utc::now();
        drop(posts);
        state.save().await;

        // Notify the suggestion author on resolve/close
        if payload.status.as_deref() == Some("resolved") && author_id != user_id {
            create_notification(
                &state,
                &author_id,
                "建议已解决",
                &format!("你的建议「{}」已被标记为已解决", suggestion_title),
                Some("/suggestions"),
            ).await;
        } else if payload.status.as_deref() == Some("closed") && author_id != user_id {
            create_notification(
                &state,
                &author_id,
                "建议已关闭",
                &format!("你的建议「{}」已被关闭", suggestion_title),
                Some("/suggestions"),
            ).await;
        }

        return Json(serde_json::json!({"success": true, "message": "建议已更新"}));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}

// ============== Reply to Suggestion ==============

pub async fn reply_to_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CreateSuggestionReplyPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_admin(&state, &user_id).await && !is_team_member(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }
    if payload.content.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "回复不能为空"}));
    }
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get_mut(&id) {
        if p.kind != "suggestion" {
            return Json(serde_json::json!({"success": false, "message": "建议不存在"}));
        }
        let author_id = p.author_id.clone();
        let suggestion_title = p.title.clone();
        let reply = PostReply {
            id: Uuid::new_v4().to_string(),
            author_id: user_id.clone(),
            author_name: user.display_name,
            content: payload.content.trim().to_string(),
            created_at: Utc::now(),
            reactions: std::collections::HashMap::new(),
            parent_id: None,
            reply_to: None,
        };
        p.replies.push(reply);
        p.updated_at = Utc::now();
        drop(posts);
        state.save().await;

        // Notify the suggestion author when someone replies
        if author_id != user_id {
            create_notification(
                &state,
                &author_id,
                "建议有新回复",
                &format!("你的建议「{}」收到了新回复", suggestion_title),
                Some("/suggestions"),
            ).await;
        }

        return Json(serde_json::json!({"success": true, "message": "回复成功"}));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}

// ============== Delete Reply ==============

pub async fn delete_suggestion_reply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((suggestion_id, reply_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get_mut(&suggestion_id) {
        if p.kind != "suggestion" {
            return Json(serde_json::json!({"success": false, "message": "建议不存在"}));
        }
        if let Some(reply) = p.replies.iter().find(|r| r.id == reply_id) {
            if !is_admin_user && reply.author_id != user_id {
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
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let mut posts = state.posts.write().await;
    if let Some(p) = posts.get(&id) {
        if p.kind != "suggestion" {
            return Json(serde_json::json!({"success": false, "message": "建议不存在"}));
        }
        if !is_admin_user && p.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权删除"}));
        }
        posts.remove(&id);
        drop(posts);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "建议已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}
