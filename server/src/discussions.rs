use axum::extract::Path;
use axum::http::HeaderMap;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::notifications::create_notification;
use crate::state::AppState;
use crate::types::*;
use crate::utils::{check_permission, resolve_user};

// ============== SQLite Row Types ==============

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PostRow {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: String,
    pub pinned: i32,
    pub team_only: i32,
    pub emoji: Option<String>,
    pub reactions: String,
    pub replies: String,
    pub mentioned_user_ids: String,
    pub status: String,
    pub visible_to: String,
    pub editable_by: String,
}

fn row_to_post(row: PostRow) -> Option<Post> {
    let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
    let reactions: std::collections::HashMap<String, Vec<String>> =
        serde_json::from_str(&row.reactions).unwrap_or_default();
    let replies: Vec<PostReply> = serde_json::from_str(&row.replies).unwrap_or_default();
    let mentioned_user_ids: Vec<String> =
        serde_json::from_str(&row.mentioned_user_ids).unwrap_or_default();
    let visible_to: Vec<String> = serde_json::from_str(&row.visible_to).unwrap_or_default();
    let editable_by: Vec<String> = serde_json::from_str(&row.editable_by).unwrap_or_default();
    let created_at: DateTime<Utc> = row.created_at.parse().ok()?;
    let updated_at: DateTime<Utc> = row.updated_at.parse().unwrap_or_else(|_| Utc::now());

    Some(Post {
        id: row.id,
        title: row.title,
        content: row.content,
        author_id: row.author_id,
        author_name: row.author_name,
        created_at,
        updated_at,
        tags,
        pinned: row.pinned != 0,
        team_only: row.team_only != 0,
        emoji: row.emoji,
        reactions,
        replies,
        mentioned_user_ids,
        status: row.status,
        visible_to,
        editable_by,
    })
}

// ============== Character Limits ==============

const TITLE_MAX_LEN: usize = 30;
const CONTENT_MAX_LEN: usize = 3000;
const REPLY_MAX_LEN: usize = 300;

/// Truncate all existing posts/replies that exceed the character limits.
/// Called at server startup to clean up legacy data.
pub async fn truncate_existing_posts(state: &AppState) {
    let posts = state.posts.read().await;
    let to_fix: Vec<Post> = posts
        .values()
        .filter(|p| {
            p.title.chars().count() > TITLE_MAX_LEN
                || p.content.chars().count() > CONTENT_MAX_LEN
                || p.replies
                    .iter()
                    .any(|r| r.content.chars().count() > REPLY_MAX_LEN)
        })
        .cloned()
        .collect();
    drop(posts);

    if to_fix.is_empty() {
        return;
    }

    for mut p in to_fix {
        if p.title.chars().count() > TITLE_MAX_LEN {
            p.title = p.title.chars().take(TITLE_MAX_LEN).collect::<String>();
        }
        if p.content.chars().count() > CONTENT_MAX_LEN {
            p.content = p.content.chars().take(CONTENT_MAX_LEN).collect::<String>();
        }
        for r in p.replies.iter_mut() {
            if r.content.chars().count() > REPLY_MAX_LEN {
                r.content = r.content.chars().take(REPLY_MAX_LEN).collect::<String>();
            }
        }
        state.upsert_post(&p).await;
    }

    tracing::info!("Truncated some existing posts/replies to meet character limits");
    state.save().await;
}

// ============== List Posts ==============

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
}

pub async fn get_posts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListPostsQuery>,
) -> Json<serde_json::Value> {
    let is_team = if let Some((_, ref user)) = resolve_user(&state, &headers).await {
        user.team_status == "joined"
    } else {
        false
    };

    // Try SQLite first, fallback to HashMap
    let posts = if let Ok(rows) =
        sqlx::query_as::<_, PostRow>("SELECT * FROM posts WHERE (? = 1 OR team_only = 0)")
            .bind(is_team as i32)
            .fetch_all(&state.db)
            .await
    {
        rows.into_iter()
            .filter_map(row_to_post)
            .collect::<Vec<Post>>()
    } else {
        let map = state.posts.read().await;
        map.values().cloned().collect()
    };

    let users = state.users.read().await;
    let all_tags = state.discussion_tags.read().await;

    let mut result: Vec<&Post> = posts
        .iter()
        .filter(|p| {
            if let Some(ref tag_ids) = query.tags {
                if !tag_ids.is_empty() {
                    let ids: Vec<&str> = tag_ids
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !ids.is_empty() && !ids.iter().any(|id| p.tags.iter().any(|t| t == id)) {
                        return false;
                    }
                }
            } else if let Some(ref tag_id) = query.tag {
                if !p.tags.iter().any(|t| t == tag_id) {
                    return false;
                }
            }
            true
        })
        .collect();

    result.sort_by(|a, b| {
        if a.pinned != b.pinned {
            if a.pinned {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        } else {
            b.created_at.cmp(&a.created_at)
        }
    });

    let list: Vec<serde_json::Value> = result
        .iter()
        .map(|p| {
            let user_info = users.get(&p.author_id);
            let display_name = user_info
                .map(|u| u.display_name.as_str())
                .unwrap_or(&p.author_name)
                .to_string();
            let avatar_url = user_info.and_then(|u| u.avatar_url.clone());
            let enriched_tags: Vec<serde_json::Value> = p
                .tags
                .iter()
                .filter_map(|tid| {
                    all_tags.get(tid).map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "name": t.name,
                            "color": t.color,
                        })
                    })
                })
                .collect();
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "content": p.content,
                "author_id": p.author_id,
                "author_name": display_name,
                "author_avatar_url": avatar_url,
                "tags": enriched_tags,
                "reactions": p.reactions,
                "emoji": p.emoji,
                "reply_count": p.replies.len(),
                "created_at": p.created_at,
                "updated_at": p.updated_at,
                "pinned": p.pinned,
                "team_only": p.team_only,
                "status": p.status,
            })
        })
        .collect();

    Json(serde_json::json!(list))
}

// ============== Create Post ==============

pub async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreatePostPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.title.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "标题不能为空"}));
    }
    if payload.title.chars().count() > TITLE_MAX_LEN {
        return Json(
            serde_json::json!({"success": false, "message": format!("标题不能超过{} 字", TITLE_MAX_LEN)}),
        );
    }
    if payload.content.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "内容不能为空"}));
    }
    if payload.content.chars().count() > CONTENT_MAX_LEN {
        return Json(
            serde_json::json!({"success": false, "message": format!("内容不能超过{} 字", CONTENT_MAX_LEN)}),
        );
    }

    let now = Utc::now();
    let is_admin_user = check_permission(&state, &user, perms::MANAGE_POSTS).await;
    let display_name = user.display_name;

    // Validate admin-only tags
    let all_tags = state.discussion_tags.read().await;
    for tag_id in &payload.tags {
        if let Some(tag) = all_tags.get(tag_id) {
            if tag.admin_only && !is_admin_user {
                return Json(
                    serde_json::json!({"success": false, "message": format!("标签「{}」仅管理员可用", tag.name)}),
                );
            }
        }
    }
    drop(all_tags);

    let post = Post {
        id: Uuid::new_v4().to_string(),
        title: payload.title.trim().to_string(),
        content: payload.content,
        author_id: user_id.clone(),
        author_name: display_name.clone(),
        tags: payload.tags.clone(),
        pinned: payload.pinned.unwrap_or(false) && is_admin_user,
        team_only: payload.team_only.unwrap_or(false),
        emoji: payload.emoji,
        reactions: std::collections::HashMap::new(),
        replies: vec![],
        mentioned_user_ids: payload.mentioned_user_ids.clone(),
        status: String::new(),
        created_at: now,
        updated_at: now,
        visible_to: vec![],
        editable_by: vec![],
    };
    let post_id = post.id.clone();
    state.upsert_post(&post).await;
    state.save().await;

    // Create notifications for @mentioned users
    for uid in &payload.mentioned_user_ids {
        if uid != &user_id {
            create_notification(
                &state,
                uid,
                "在帖子中提到了你",
                &format!(
                    "{} 在帖子「{}」中提到了你",
                    display_name,
                    payload.title.trim()
                ),
                Some(&format!("/post/{}", post_id)),
            )
            .await;
        }
    }

    Json(serde_json::json!({"success": true, "message": "帖子已发布", "id": post_id}))
}

// ============== Get Post Detail ==============

pub async fn get_post_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let is_team = if let Some((_, ref user)) = resolve_user(&state, &headers).await {
        user.team_status == "joined"
    } else {
        false
    };

    // Try SQLite first, then fall back to HashMap
    let p = if let Ok(row) = sqlx::query_as::<_, PostRow>(
        "SELECT id, title, content, author_id, author_name, created_at, updated_at, \
         tags, pinned, team_only, emoji, reactions, replies, mentioned_user_ids, \
         status, visible_to, editable_by FROM posts WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    {
        let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
        let reactions: std::collections::HashMap<String, Vec<String>> =
            serde_json::from_str(&row.reactions).unwrap_or_default();
        let replies: Vec<PostReply> = serde_json::from_str(&row.replies).unwrap_or_default();
        let mentioned_user_ids: Vec<String> =
            serde_json::from_str(&row.mentioned_user_ids).unwrap_or_default();
        let visible_to: Vec<String> = serde_json::from_str(&row.visible_to).unwrap_or_default();
        let editable_by: Vec<String> = serde_json::from_str(&row.editable_by).unwrap_or_default();
        let created_at: DateTime<Utc> = row.created_at.parse().unwrap_or_else(|_| Utc::now());
        let updated_at: DateTime<Utc> = row.updated_at.parse().unwrap_or_else(|_| Utc::now());

        Post {
            id: row.id,
            title: row.title,
            content: row.content,
            author_id: row.author_id,
            author_name: row.author_name,
            created_at,
            updated_at,
            tags,
            pinned: row.pinned != 0,
            team_only: row.team_only != 0,
            emoji: row.emoji,
            reactions,
            replies,
            mentioned_user_ids,
            status: row.status,
            visible_to,
            editable_by,
        }
    } else {
        // Fallback: try HashMap
        let posts = state.posts.read().await;
        match posts.get(&id).cloned() {
            Some(p) => p,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
                ));
            }
        }
    };

    if p.team_only && !is_team {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"success": false, "message": "无权查看"})),
        ));
    }
    let users = state.users.read().await;
    let enriched_replies: Vec<serde_json::Value> = p
        .replies
        .iter()
        .map(|r| {
            let user_info = users.get(&r.author_id);
            let display_name = user_info
                .map(|u| u.display_name.clone())
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
        })
        .collect();
    let author_info = users.get(&p.author_id);
    let author_name = author_info
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| p.author_name.clone());
    let author_avatar_url = author_info.and_then(|u| u.avatar_url.clone());
    let all_tags = state.discussion_tags.read().await;
    let enriched_tags: Vec<serde_json::Value> = p
        .tags
        .iter()
        .filter_map(|tid| {
            all_tags.get(tid).map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": t.name,
                    "color": t.color,
                    "admin_only": t.admin_only,
                })
            })
        })
        .collect();
    drop(all_tags);
    drop(users);

    Ok(Json(serde_json::json!({
        "id": p.id,
        "title": p.title,
        "content": p.content,
        "author_id": p.author_id,
        "author_name": author_name,
        "author_avatar_url": author_avatar_url,
        "tags": enriched_tags,
        "reactions": p.reactions,
        "emoji": p.emoji,
        "replies": enriched_replies,
        "created_at": p.created_at,
        "updated_at": p.updated_at,
        "pinned": p.pinned,
        "team_only": p.team_only,
        "status": p.status,
    })))
}

// ============== Update Post ==============

pub async fn update_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdatePostPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = check_permission(&state, &user, perms::MANAGE_POSTS).await;
    let mut p = match state.posts.read().await.get(&id).cloned() {
        Some(p) => p,
        None => return Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
    };
    if !is_admin_user && p.author_id != user_id {
        return Json(serde_json::json!({"success": false, "message": "无权修改"}));
    }
    if let Some(ref title) = payload.title {
        if !title.trim().is_empty() {
            if title.chars().count() > TITLE_MAX_LEN {
                return Json(
                    serde_json::json!({"success": false, "message": format!("标题不能超过{} 字", TITLE_MAX_LEN)}),
                );
            }
            p.title = title.trim().to_string();
        }
    }
    if let Some(ref content) = payload.content {
        if !content.trim().is_empty() {
            if content.chars().count() > CONTENT_MAX_LEN {
                return Json(
                    serde_json::json!({"success": false, "message": format!("内容不能超过{} 字", CONTENT_MAX_LEN)}),
                );
            }
            p.content = content.clone();
        }
    }
    if let Some(ref tags) = payload.tags {
        // Validate admin-only tags
        if is_admin_user {
            p.tags = tags.clone();
        } else {
            // Non-admin: only keep tags they're allowed to set
            let all_tags = state.discussion_tags.read().await;
            p.tags = tags
                .iter()
                .filter(|tid| all_tags.get(*tid).map(|t| !t.admin_only).unwrap_or(true))
                .cloned()
                .collect();
        }
    }
    if let Some(ref emoji) = payload.emoji {
        p.emoji = emoji.clone();
    }
    if let Some(pinned) = payload.pinned {
        if is_admin_user {
            p.pinned = pinned;
        }
    }
    if let Some(team_only) = payload.team_only {
        if is_admin_user || p.author_id == user_id {
            p.team_only = team_only;
        }
    }
    if let Some(ref status) = payload.status {
        if is_admin_user {
            let valid = ["open", "in_progress", "resolved", "closed"];
            if valid.contains(&status.as_str()) {
                let old_status = p.status.clone();
                let author_id = p.author_id.clone();
                let post_title = p.title.clone();
                p.status = status.clone();

                // Notify post author on resolve/close
                if status == "resolved" && author_id != user_id {
                    create_notification(
                        &state,
                        &author_id,
                        "帖子状态已更新",
                        &format!("你的帖子「{}」已被标记为已解决", post_title),
                        Some(&format!("/post/{}", id)),
                    )
                    .await;
                } else if status == "closed" && author_id != user_id && old_status != "closed" {
                    create_notification(
                        &state,
                        &author_id,
                        "帖子已关闭",
                        &format!("你的帖子「{}」已被关闭", post_title),
                        Some(&format!("/post/{}", id)),
                    )
                    .await;
                }
            } else {
                return Json(serde_json::json!({"success": false, "message": "无效状态"}));
            }
        }
    }
    p.updated_at = Utc::now();
    state.upsert_post(&p).await;
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "帖子已更新"}))
}

// ============== Delete Post ==============

pub async fn delete_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = check_permission(&state, &user, perms::MANAGE_POSTS).await;
    let post = state.posts.read().await.get(&id).cloned();
    match post {
        Some(p) => {
            if !is_admin_user && p.author_id != user_id {
                return Json(serde_json::json!({"success": false, "message": "无权删除"}));
            }
            state.delete_post_by_id(&id).await;
            state.save().await;
            Json(serde_json::json!({"success": true, "message": "帖子已删除"}))
        }
        None => Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
    }
}

// ============== Reply to Post ==============

pub async fn reply_to_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CreatePostReplyPayload>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.content.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "回复不能为空"}));
    }
    if payload.content.chars().count() > REPLY_MAX_LEN {
        return Json(
            serde_json::json!({"success": false, "message": format!("回复不能超过{} 字", REPLY_MAX_LEN)}),
        );
    }
    let mut p = match state.posts.read().await.get(&id).cloned() {
        Some(p) => p,
        None => return Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
    };
    let display_name = user.display_name;
    let reply = PostReply {
        id: Uuid::new_v4().to_string(),
        author_id: user_id.clone(),
        author_name: display_name.clone(),
        content: payload.content.trim().to_string(),
        reactions: std::collections::HashMap::new(),
        parent_id: payload.parent_id,
        reply_to: payload.reply_to,
        created_at: Utc::now(),
    };
    p.replies.push(reply);
    p.updated_at = Utc::now();
    let post_title = p.title.clone();
    let post_author_id = p.author_id.clone();
    let mentioned = payload.mentioned_user_ids.clone();
    state.upsert_post(&p).await;
    state.save().await;

    // Notify post author when someone replies
    if post_author_id != user_id {
        create_notification(
            &state,
            &post_author_id,
            "帖子有新回复",
            &format!("你的帖子「{}」收到了新回复", post_title),
            Some(&format!("/post/{}", id)),
        )
        .await;
    }

    // Create notifications for @mentioned users
    for uid in &mentioned {
        if uid != &user_id {
            create_notification(
                &state,
                uid,
                "在回复中提到了你",
                &format!("{} 在帖子「{}」的回复中提到了你", display_name, post_title),
                Some(&format!("/post/{}", id)),
            )
            .await;
        }
    }

    Json(serde_json::json!({"success": true, "message": "回复成功"}))
}

// ============== Delete Reply ==============

pub async fn delete_post_reply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((post_id, reply_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let (user_id, user) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    let is_admin_user = check_permission(&state, &user, perms::MANAGE_POSTS).await;
    let mut p = match state.posts.read().await.get(&post_id).cloned() {
        Some(p) => p,
        None => return Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
    };
    if let Some(reply) = p.replies.iter().find(|r| r.id == reply_id) {
        if !is_admin_user && reply.author_id != user_id {
            return Json(serde_json::json!({"success": false, "message": "无权删除"}));
        }
    } else {
        return Json(serde_json::json!({"success": false, "message": "回复不存在"}));
    }
    p.replies.retain(|r| r.id != reply_id);
    p.replies
        .retain(|r| r.parent_id.as_deref() != Some(&reply_id));
    p.updated_at = Utc::now();
    state.upsert_post(&p).await;
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "回复已删除"}))
}

// ============== Tags (read-only — managed via config) ==============

pub async fn get_discussion_tags(State(state): State<AppState>) -> Json<Vec<DiscussionTag>> {
    let tags = state.discussion_tags.read().await;
    Json(tags.values().cloned().collect())
}

// ============== Emojis (read-only — managed via config) ==============

pub async fn get_discussion_emojis(State(state): State<AppState>) -> Json<Vec<DiscussionEmoji>> {
    let emojis = state.discussion_emojis.read().await;
    Json(emojis.values().cloned().collect())
}

// ============== Reactions ==============

/// Toggle a reaction on a post.
pub async fn react_to_post(
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
    let mut p = match state.posts.read().await.get(&id).cloned() {
        Some(p) => p,
        None => return Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
    };
    let emoji = payload.emoji.trim().to_string();
    let users_set = p.reactions.entry(emoji.clone()).or_insert_with(Vec::new);
    if let Some(pos) = users_set.iter().position(|u| u == &user_id) {
        users_set.remove(pos);
        if users_set.is_empty() {
            p.reactions.remove(&emoji);
        }
    } else {
        users_set.push(user_id);
    }
    state.upsert_post(&p).await;
    state.save().await;
    Json(serde_json::json!({"success": true}))
}

/// Toggle a reaction on a reply.
pub async fn react_to_reply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((post_id, reply_id)): Path<(String, String)>,
    Json(payload): Json<ReactPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if payload.emoji.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "表情不能为空"}));
    }
    let mut p = match state.posts.read().await.get(&post_id).cloned() {
        Some(p) => p,
        None => return Json(serde_json::json!({"success": false, "message": "帖子不存在"})),
    };
    let emoji = payload.emoji.trim().to_string();
    if let Some(reply) = p.replies.iter_mut().find(|r| r.id == reply_id) {
        let users_set = reply
            .reactions
            .entry(emoji.clone())
            .or_insert_with(Vec::new);
        if let Some(pos) = users_set.iter().position(|u| u == &user_id) {
            users_set.remove(pos);
            if users_set.is_empty() {
                reply.reactions.remove(&emoji);
            }
        } else {
            users_set.push(user_id);
        }
        state.upsert_post(&p).await;
        state.save().await;
        return Json(serde_json::json!({"success": true}));
    }
    Json(serde_json::json!({"success": false, "message": "回复不存在"}))
}

// ============== Cleanup Orphan Reactions ==============

/// Remove reactions that reference emoji IDs that no longer exist in the config.
pub async fn cleanup_orphan_reactions(state: &AppState) {
    let emojis = state.discussion_emojis.read().await;
    let valid_emoji_ids: std::collections::HashSet<String> = emojis.keys().cloned().collect();
    drop(emojis);

    let posts = state.posts.read().await;
    let posts_to_fix: Vec<Post> = posts.values().cloned().collect();
    drop(posts);

    let mut changed = false;
    for mut p in posts_to_fix {
        let mut post_changed = false;
        // Clean up post reactions
        let old_keys: Vec<String> = p.reactions.keys().cloned().collect();
        for k in old_keys {
            if !valid_emoji_ids.contains(&k) {
                p.reactions.remove(&k);
                post_changed = true;
            }
        }
        // Clean up reply reactions
        for r in p.replies.iter_mut() {
            let old_keys: Vec<String> = r.reactions.keys().cloned().collect();
            for k in old_keys {
                if !valid_emoji_ids.contains(&k) {
                    r.reactions.remove(&k);
                    post_changed = true;
                }
            }
        }
        if post_changed {
            state.upsert_post(&p).await;
            changed = true;
        }
    }
    if changed {
        tracing::info!("Cleaned up orphan reactions");
    }
}
