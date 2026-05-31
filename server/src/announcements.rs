// ============== Legacy Announcement Compat Layer ==============
//
// These routes are kept for backward compatibility.
// They delegate to the unified post handlers.

use axum::http::HeaderMap;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::state::AppState;
use crate::types::*;
use crate::utils::{check_permission, resolve_user, AuthUser};

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

// ============== List Announcements (backward compat) ==============

pub async fn get_announcements(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let can_see_all = if let Some((_user_id, user)) = resolve_user(&state, &headers).await {
        check_permission(&state, &user, crate::types::perms::MANAGE_DISCUSSIONS).await
            || user.team_status == "joined"
    } else {
        false
    };

    // Try SQLite first, fallback to HashMap
    let posts: Vec<Post> = if let Ok(rows) =
        sqlx::query_as::<_, PostRow>("SELECT * FROM posts WHERE tags LIKE '%\"公告\"%'")
            .fetch_all(&state.db)
            .await
    {
        rows.into_iter().filter_map(row_to_post).collect()
    } else {
        let map = state.posts.read().await;
        map.values()
            .filter(|p| p.tags.contains(&"公告".to_string()))
            .cloned()
            .collect()
    };

    let users = state.users.read().await;

    let mut result: Vec<serde_json::Value> = Vec::new();
    for p in &posts {
        if !can_see_all && p.team_only {
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
            "pinned": p.pinned,
            "public": !p.team_only,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        }));
    }
    result.sort_by(|a, b| {
        let a_pinned = a["pinned"].as_bool().unwrap_or(false);
        let b_pinned = b["pinned"].as_bool().unwrap_or(false);
        b_pinned.cmp(&a_pinned).then_with(|| {
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
    auth: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_DISCUSSIONS)
        .await?;
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
    let pinned = payload
        .get("pinned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_public = payload
        .get("public")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if title.trim().is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "标题不能为空"}),
        ));
    }
    let now = Utc::now();
    let post = Post {
        id: Uuid::new_v4().to_string(),
        title: title.trim().to_string(),
        content,
        author_id: auth.user_id,
        author_name: auth.user.display_name,
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
        visible_to: vec![],
        editable_by: vec![],
    };
    state.upsert_post(&post).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "公告已发布"}),
    ))
}

// ============== Get Announcement Detail (backward compat) ==============

pub async fn get_announcement_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let auth_info = resolve_user(&state, &headers).await;
    let is_admin_user = if let Some((_, ref user)) = auth_info {
        check_permission(&state, user, crate::types::perms::MANAGE_DISCUSSIONS).await
    } else {
        false
    };
    let is_team = if let Some((_, ref user)) = auth_info {
        user.team_status == "joined"
    } else {
        false
    };
    let posts = state.posts.read().await;
    if let Some(p) = posts.get(&id) {
        if !is_admin_user && !is_team && p.team_only {
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
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_DISCUSSIONS)
        .await?;
    let p_opt = {
        let posts = state.posts.read().await;
        posts.get(&id).cloned()
    };
    if let Some(mut p) = p_opt {
        if let Some(title) = payload.get("title").and_then(|v| v.as_str()) {
            if title.trim().is_empty() {
                return Ok(Json(
                    serde_json::json!({"success": false, "message": "标题不能为空"}),
                ));
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
        state.upsert_post(&p).await;
        return Ok(Json(
            serde_json::json!({"success": true, "message": "公告已更新"}),
        ));
    }
    Ok(Json(
        serde_json::json!({"success": false, "message": "公告不存在"}),
    ))
}

// ============== Delete Announcement ==============

pub async fn delete_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_DISCUSSIONS)
        .await?;
    let exists = { state.posts.read().await.contains_key(&id) };
    if exists {
        state.delete_post_by_id(&id).await;
        return Ok(Json(
            serde_json::json!({"success": true, "message": "公告已删除"}),
        ));
    }
    Ok(Json(
        serde_json::json!({"success": false, "message": "公告不存在"}),
    ))
}
