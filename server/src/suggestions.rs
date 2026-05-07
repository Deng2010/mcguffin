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
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let is_team = is_team_member(&state, &user_id).await;
    let suggestions = state.suggestions.read().await;
    let mut result: Vec<&Suggestion> = suggestions.values().collect();
    if !is_admin_user && !is_team {
        // Guests/pending see only their own suggestions
        result.retain(|s| s.author_id == user_id);
    }
    result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
    let suggestion = Suggestion {
        id: Uuid::new_v4().to_string(),
        title: payload.title.trim().to_string(),
        content: payload.content,
        author_id: user_id,
        author_name: user.display_name,
        status: "open".to_string(),
        replies: vec![],
        created_at: now,
        updated_at: now,
    };
    state.suggestions.write().await.insert(suggestion.id.clone(), suggestion);
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
    let suggestions = state.suggestions.read().await;
    if let Some(s) = suggestions.get(&id) {
        let is_admin_user = is_admin(&state, &user_id).await;
        let is_team = is_team_member(&state, &user_id).await;
        if !is_admin_user && !is_team && s.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权查看"}));
        }
        return Json(serde_json::json!(s));
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
    let mut suggestions = state.suggestions.write().await;
    if let Some(s) = suggestions.get_mut(&id) {
        let _old_status = s.status.clone();
        let author_id = s.author_id.clone();
        let suggestion_title = s.title.clone();

        if let Some(ref new_status) = payload.status {
            let valid = ["open", "in_progress", "resolved", "closed"];
            if !valid.contains(&new_status.as_str()) {
                return Json(serde_json::json!({"success": false, "message": "无效状态"}));
            }
            s.status = new_status.clone();
        }
        s.updated_at = Utc::now();
        drop(suggestions);
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
    let mut suggestions = state.suggestions.write().await;
    if let Some(s) = suggestions.get_mut(&id) {
        let author_id = s.author_id.clone();
        let suggestion_title = s.title.clone();
        let reply = SuggestionReply {
            id: Uuid::new_v4().to_string(),
            author_id: user_id.clone(),
            author_name: user.display_name,
            content: payload.content.trim().to_string(),
            created_at: Utc::now(),
        };
        s.replies.push(reply);
        s.updated_at = Utc::now();
        drop(suggestions);
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
    let mut suggestions = state.suggestions.write().await;
    if let Some(s) = suggestions.get_mut(&suggestion_id) {
        if let Some(reply) = s.replies.iter().find(|r| r.id == reply_id) {
            if !is_admin_user && reply.author_id != user_id {
                return Json(serde_json::json!({"success": false, "message": "无权删除"}));
            }
        } else {
            return Json(serde_json::json!({"success": false, "message": "回复不存在"}));
        }
        s.replies.retain(|r| r.id != reply_id);
        drop(suggestions);
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
    let mut suggestions = state.suggestions.write().await;
    if let Some(s) = suggestions.get(&id) {
        if !is_admin_user && s.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权删除"}));
        }
        suggestions.remove(&id);
        drop(suggestions);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "建议已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "建议不存在"}))
}
