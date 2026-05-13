use axum::{
    extract::{Path, State},
    Json,
};
use axum::http::HeaderMap;
use chrono::Utc;
use uuid::Uuid;

use crate::notifications::create_notification;
use crate::state::AppState;
use crate::types::*;
use crate::utils::{is_admin, is_team_member, resolve_user};

// ============== Character Limits ==============

const TITLE_MAX_LEN: usize = 30;
const CONTENT_MAX_LEN: usize = 3000;
const REPLY_MAX_LEN: usize = 300;

/// Truncate all existing discussions/replies that exceed the character limits.
/// Called at server startup to clean up legacy data.
pub async fn truncate_existing_discussions(state: &AppState) {
    let mut discussions = state.discussions.write().await;
    let mut changed = false;
    for d in discussions.values_mut() {
        if d.title.chars().count() > TITLE_MAX_LEN {
            d.title = d.title.chars().take(TITLE_MAX_LEN).collect::<String>();
            changed = true;
        }
        if d.content.chars().count() > CONTENT_MAX_LEN {
            d.content = d.content.chars().take(CONTENT_MAX_LEN).collect::<String>();
            changed = true;
        }
        for r in d.replies.iter_mut() {
            if r.content.chars().count() > REPLY_MAX_LEN {
                r.content = r.content.chars().take(REPLY_MAX_LEN).collect::<String>();
                changed = true;
            }
        }
    }
    drop(discussions);
    if changed {
        tracing::info!("Truncated some existing discussions/replies to meet character limits");
        state.save().await;
    }
}

// ============== List Discussions ==============

pub async fn get_discussions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!([])),
    };
    let is_team_member = is_team_member(&state, &user_id).await;
    let discussions = state.discussions.read().await;
    let users = state.users.read().await;
    let all_tags = state.discussion_tags.read().await;
    let mut result: Vec<&Discussion> = discussions.values().filter(|d| {
        // Filter out team-only discussions for non-team-member users
        if d.team_only && !is_team_member { return false; }
        true
    }).collect();
    // Sort: pinned first, then by created_at desc
    result.sort_by(|a, b| {
        if a.pinned != b.pinned {
            if a.pinned { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
        } else {
            b.created_at.cmp(&a.created_at)
        }
    });
    // Return without replies in listing (replies are fetched in detail)
    let list: Vec<serde_json::Value> = result.iter().map(|d| {
        let user_info = users.get(&d.author_id);
        let display_name = user_info.map(|u| u.display_name.as_str())
            .unwrap_or(&d.author_name)
            .to_string();
        let avatar_url = user_info.and_then(|u| u.avatar_url.clone());
        let enriched_tags: Vec<serde_json::Value> = d.tags.iter().filter_map(|tid| {
            all_tags.get(tid).map(|t| serde_json::json!({
                "id": t.id,
                "name": t.name,
                "color": t.color,
            }))
        }).collect();
        serde_json::json!({
            "id": d.id,
            "title": d.title,
            "content": d.content,
            "author_id": d.author_id,
            "author_name": display_name,
            "author_avatar_url": avatar_url,
            "tags": enriched_tags,
            "reactions": d.reactions,
            "emoji": d.emoji,
            "reply_count": d.replies.len(),
            "created_at": d.created_at,
            "updated_at": d.updated_at,
            "pinned": d.pinned,
            "team_only": d.team_only,
        })
    }).collect();
    drop(all_tags);
    drop(users);
    Json(serde_json::json!(list))
}

// ============== Create Discussion ==============

pub async fn create_discussion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateDiscussionPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.title.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
    }
    if payload.title.chars().count() > TITLE_MAX_LEN {
        return Json(serde_json::json!({"success": false, "message": format!("标题不能超过{} 字", TITLE_MAX_LEN)}));
    }
    if payload.content.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "内容不能为空"}));
    }
    if payload.content.chars().count() > CONTENT_MAX_LEN {
        return Json(serde_json::json!({"success": false, "message": format!("内容不能超过{} 字", CONTENT_MAX_LEN)}));
    }
    let now = Utc::now();
    let is_admin_user = is_admin(&state, &user_id).await;
    let display_name = user.display_name;
    let discussion = Discussion {
        id: Uuid::new_v4().to_string(),
        title: payload.title.trim().to_string(),
        content: payload.content,
        author_id: user_id.clone(),
        author_name: display_name.clone(),
        tags: payload.tags,
        emoji: payload.emoji,
        pinned: payload.pinned.unwrap_or(false) && is_admin_user,
        team_only: payload.team_only.unwrap_or(false),
        replies: vec![],
        reactions: std::collections::HashMap::new(),
        created_at: now,
        updated_at: now,
    };
    let discussion_id = discussion.id.clone();
    state.discussions.write().await.insert(discussion_id.clone(), discussion);
    state.save().await;

    // Create notifications for @mentioned users
    for uid in &payload.mentioned_user_ids {
        if uid != &user_id {
            create_notification(
                &state,
                uid,
                &format!("在讨论中提到了你"),
                &format!("{} 在讨论「{}」中提到了你", display_name, payload.title.trim()),
                Some(&format!("/discussions/{}", discussion_id)),
            ).await;
        }
    }

    Json(serde_json::json!({"success": true, "message": "讨论已发布"}))
}

// ============== Get Discussion Detail ==============

pub async fn get_discussion_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let discussions = state.discussions.read().await;
    if let Some(d) = discussions.get(&id) {
        // Check access for team-only discussions
        if d.team_only && !is_team_member(&state, &user_id).await {
            return Json(serde_json::json!({"success": false, "message": "无权查看"}));
        }
        // Enrich replies with updated author display names and avatars
        let users = state.users.read().await;
        let enriched_replies: Vec<serde_json::Value> = d.replies.iter().map(|r| {
            let user_info = users.get(&r.author_id);
            let display_name = user_info.map(|u| u.display_name.clone())
                .unwrap_or_else(|| r.author_name.clone());
            let avatar_url = user_info.and_then(|u| u.avatar_url.clone());
            serde_json::json!({
                "id": r.id,
                "author_id": r.author_id,
                "author_name": display_name,
                "author_avatar_url": avatar_url,
                "content": r.content,
                "reactions": r.reactions,
                "parent_id": r.parent_id,
                "reply_to": r.reply_to,
                "created_at": r.created_at,
            })
        }).collect();
        let author_info = users.get(&d.author_id);
        let author_name = author_info.map(|u| u.display_name.clone())
            .unwrap_or_else(|| d.author_name.clone());
        let author_avatar_url = author_info.and_then(|u| u.avatar_url.clone());
        let all_tags = state.discussion_tags.read().await;
        let enriched_tags: Vec<serde_json::Value> = d.tags.iter().filter_map(|tid| {
            all_tags.get(tid).map(|t| serde_json::json!({
                "id": t.id,
                "name": t.name,
                "color": t.color,
            }))
        }).collect();
        drop(all_tags);
        drop(users);

        return Json(serde_json::json!({
            "id": d.id,
            "title": d.title,
            "content": d.content,
            "author_id": d.author_id,
            "author_name": author_name,
            "author_avatar_url": author_avatar_url,
            "tags": enriched_tags,
            "reactions": d.reactions,
            "emoji": d.emoji,
            "replies": enriched_replies,
            "created_at": d.created_at,
            "updated_at": d.updated_at,
            "pinned": d.pinned,
            "team_only": d.team_only,
        }));
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

// ============== Update Discussion ==============

pub async fn update_discussion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDiscussionPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let mut discussions = state.discussions.write().await;
    if let Some(d) = discussions.get_mut(&id) {
        if !is_admin_user && d.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权修改"}));
        }
        if let Some(ref title) = payload.title {
            if !title.trim().is_empty() {
                if title.chars().count() > TITLE_MAX_LEN {
                    return Json(serde_json::json!({"success": false, "message": format!("标题不能超过{} 字", TITLE_MAX_LEN)}));
                }
                d.title = title.trim().to_string();
            }
        }
        if let Some(ref content) = payload.content {
            if !content.trim().is_empty() {
                if content.chars().count() > CONTENT_MAX_LEN {
                    return Json(serde_json::json!({"success": false, "message": format!("内容不能超过{} 字", CONTENT_MAX_LEN)}));
                }
                d.content = content.clone();
            }
        }
        if let Some(ref tags) = payload.tags {
            d.tags = tags.clone();
        }
        if let Some(ref emoji) = payload.emoji {
            d.emoji = emoji.clone();
        }
        // pinned: only admin can change
        if let Some(pinned) = payload.pinned {
            if is_admin_user {
                d.pinned = pinned;
            }
        }
        // team_only: admin or author can change
        if let Some(team_only) = payload.team_only {
            if is_admin_user || d.author_id == user_id {
                d.team_only = team_only;
            }
        }
        d.updated_at = Utc::now();
        drop(discussions);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "讨论已更新"}));
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

// ============== Delete Discussion ==============

pub async fn delete_discussion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let mut discussions = state.discussions.write().await;
    if let Some(d) = discussions.get(&id) {
        if !is_admin_user && d.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权删除"}));
        }
        discussions.remove(&id);
        drop(discussions);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "讨论已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

// ============== Reply to Discussion ==============

pub async fn reply_to_discussion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CreateDiscussionReplyPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.content.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "回复不能为空"}));
    }
    if payload.content.chars().count() > REPLY_MAX_LEN {
        return Json(serde_json::json!({"success": false, "message": format!("回复不能超过{} 字", REPLY_MAX_LEN)}));
    }
    let mut discussions = state.discussions.write().await;
    if let Some(d) = discussions.get_mut(&id) {
        let display_name = user.display_name;
        let reply = DiscussionReply {
            id: Uuid::new_v4().to_string(),
            author_id: user_id.clone(),
            author_name: display_name.clone(),
            content: payload.content.trim().to_string(),
            reactions: std::collections::HashMap::new(),
            parent_id: payload.parent_id,
            reply_to: payload.reply_to,
            created_at: Utc::now(),
        };
        d.replies.push(reply);
        d.updated_at = Utc::now();
        drop(discussions);
        state.save().await;

        // Create notifications for @mentioned users
        let discussion_title = state.discussions.read().await.get(&id).map(|d| d.title.clone()).unwrap_or_default();
        for uid in &payload.mentioned_user_ids {
            if uid != &user_id {
                create_notification(
                    &state,
                    uid,
                    "在回复中提到了你",
                    &format!("{} 在讨论「{}」的回复中提到了你", display_name, discussion_title),
                    Some(&format!("/discussions/{}", id)),
                ).await;
            }
        }

        return Json(serde_json::json!({"success": true, "message": "回复成功"}));
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

// ============== Delete Reply ==============

pub async fn delete_discussion_reply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((discussion_id, reply_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = is_admin(&state, &user_id).await;
    let mut discussions = state.discussions.write().await;
    if let Some(d) = discussions.get_mut(&discussion_id) {
        if let Some(reply) = d.replies.iter().find(|r| r.id == reply_id) {
            if !is_admin_user && reply.author_id != user_id {
                return Json(serde_json::json!({"success": false, "message": "无权删除"}));
            }
        } else {
            return Json(serde_json::json!({"success": false, "message": "回复不存在"}));
        }
        d.replies.retain(|r| r.id != reply_id);
        // Also delete child replies (replies to this reply)
        d.replies.retain(|r| r.parent_id.as_deref() != Some(&reply_id));
        d.updated_at = Utc::now();
        drop(discussions);
        state.save().await;
        return Json(serde_json::json!({"success": true, "message": "回复已删除"}));
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

// ============== Tags (read-only — managed via config) ==============

pub async fn get_discussion_tags(
    State(state): State<AppState>,
) -> Json<Vec<DiscussionTag>> {
    let tags = state.discussion_tags.read().await;
    Json(tags.values().cloned().collect())
}

// ============== Emojis (read-only — managed via config) ==============

pub async fn get_discussion_emojis(
    State(state): State<AppState>,
) -> Json<Vec<DiscussionEmoji>> {
    let emojis = state.discussion_emojis.read().await;
    Json(emojis.values().cloned().collect())
}

// ============== Reactions ==============

/// Toggle a reaction on a discussion.
pub async fn react_to_discussion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<ReactPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.emoji.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "表情不能为空"}));
    }
    let mut discussions = state.discussions.write().await;
    if let Some(d) = discussions.get_mut(&id) {
        let emoji = payload.emoji.trim().to_string();
        let users = d.reactions.entry(emoji.clone()).or_insert_with(Vec::new);
        if let Some(pos) = users.iter().position(|u| u == &user_id) {
            users.remove(pos);
            if users.is_empty() {
                d.reactions.remove(&emoji);
            }
        } else {
            users.push(user_id.clone());
        }
        d.updated_at = Utc::now();
        drop(discussions);
        state.save().await;
        return Json(serde_json::json!({"success": true}));
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

/// Toggle a reaction on a reply.
pub async fn react_to_reply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((discussion_id, reply_id)): Path<(String, String)>,
    Json(payload): Json<ReactPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.emoji.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "表情不能为空"}));
    }
    let mut discussions = state.discussions.write().await;
    if let Some(d) = discussions.get_mut(&discussion_id) {
        if let Some(reply) = d.replies.iter_mut().find(|r| r.id == reply_id) {
            let emoji = payload.emoji.trim().to_string();
            let users = reply.reactions.entry(emoji.clone()).or_insert_with(Vec::new);
            if let Some(pos) = users.iter().position(|u| u == &user_id) {
                users.remove(pos);
                if users.is_empty() {
                    reply.reactions.remove(&emoji);
                }
            } else {
                users.push(user_id.clone());
            }
            d.updated_at = Utc::now();
            drop(discussions);
            state.save().await;
            return Json(serde_json::json!({"success": true}));
        } else {
            return Json(serde_json::json!({"success": false, "message": "回复不存在"}));
        }
    }
    Json(serde_json::json!({"success": false, "message": "讨论不存在"}))
}

/// Remove reactions whose emoji character is no longer in discussion_emojis.
/// Called at server startup to clean up orphaned reactions.
pub async fn cleanup_orphan_reactions(state: &AppState) {
    let valid_emojis: std::collections::HashSet<String> = {
        let emojis = state.discussion_emojis.read().await;
        emojis.values().map(|e| e.char.clone()).collect()
    };

    let mut discussions = state.discussions.write().await;
    let mut changed = false;

    for d in discussions.values_mut() {
        // Clean discussion-level reactions
        let before = d.reactions.len();
        d.reactions.retain(|emoji, _| valid_emojis.contains(emoji));
        if d.reactions.len() != before {
            changed = true;
        }

        // Clean reply-level reactions
        for r in d.replies.iter_mut() {
            let before_reply = r.reactions.len();
            r.reactions.retain(|emoji, _| valid_emojis.contains(emoji));
            if r.reactions.len() != before_reply {
                changed = true;
            }
        }
    }
    drop(discussions);

    if changed {
        tracing::info!("Cleaned up orphaned discussion reactions");
        state.save().await;
    }
}
