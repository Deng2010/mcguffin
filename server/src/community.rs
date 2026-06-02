use axum::http::HeaderMap;
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};

use crate::state::AppState;
use crate::types::*;
use crate::utils::resolve_user;

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

// ============== Unified Community Feed ==============

pub async fn get_community_posts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CommunityQuery>,
) -> Json<serde_json::Value> {
    let auth_info = resolve_user(&state, &headers).await;
    let is_team = auth_info
        .as_ref()
        .map(|(_, user)| user.team_status == "joined")
        .unwrap_or(false);

    // Parse tag filter
    let filter_tags: Vec<String> = if let Some(ref tags_str) = query.tags {
        if !tags_str.is_empty() {
            tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![]
        }
    } else if let Some(ref tag) = query.tag {
        vec![tag.clone()]
    } else {
        vec![]
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

    let mut result: Vec<PostListItem> = Vec::new();

    for p in &posts {
        // ── Tag filter ──
        if !filter_tags.is_empty() && !p.tags.iter().any(|t| filter_tags.contains(t)) {
            continue;
        }

        let author_name = users
            .get(&p.author_id)
            .map(|u| u.display_name.clone())
            .unwrap_or_else(|| p.author_name.clone());
        let author_avatar_url = users.get(&p.author_id).and_then(|u| u.avatar_url.clone());

        let content_preview = p.content.chars().take(200).collect::<String>();

        result.push(PostListItem {
            id: p.id.clone(),
            title: p.title.clone(),
            content_preview,
            author_id: p.author_id.clone(),
            author_name,
            author_avatar_url,
            tags: p.tags.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
            pinned: p.pinned,
            status: p.status.clone(),
            team_only: p.team_only,
            reply_count: p.replies.len(),
            detail_url: format!("/post/{}", p.id),
        });
    }

    // Sort: pinned first, then by updated_at descending
    result.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });

    // ── Pagination ──
    let total = result.len() as u32;
    let total_pages = if total == 0 { 1 } else { (total + query.limit - 1) / query.limit };
    let page = query.page.max(1).min(total_pages);
    let offset = ((page - 1) * query.limit) as usize;
    let items: Vec<PostListItem> = result.into_iter().skip(offset).take(query.limit as usize).collect();

    Json(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "total_pages": total_pages,
        "limit": query.limit,
    }))
}
