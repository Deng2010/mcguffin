use axum::{
    extract::{Query, State},
    Json,
};
use axum::http::HeaderMap;

use crate::state::AppState;
use crate::types::*;
use crate::utils::{is_admin, is_team_member, resolve_user};

// ============== Unified Community Feed ==============

pub async fn get_community_posts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CommunityQuery>,
) -> Json<serde_json::Value> {
    let auth_info = resolve_user(&state, &headers).await;
    let user_id = auth_info.as_ref().map(|(id, _)| id.clone()).unwrap_or_default();
    let is_admin_user = if let Some((ref uid, _)) = auth_info { is_admin(&state, uid).await } else { false };
    let is_team = if let Some((ref uid, _)) = auth_info { is_team_member(&state, uid).await } else { false };

    // Parse tag filter
    let filter_tags: Vec<String> = if let Some(ref tags_str) = query.tags {
        if !tags_str.is_empty() {
            tags_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        } else {
            vec![]
        }
    } else if let Some(ref tag) = query.tag {
        vec![tag.clone()]
    } else {
        vec![]
    };

    let posts = state.posts.read().await;
    let users = state.users.read().await;

    let mut result: Vec<PostListItem> = Vec::new();

    for p in posts.values() {
        // ── Visibility per kind ──
        match p.kind.as_str() {
            "discussion" => {
                if p.team_only && !is_team { continue; }
            }
            "suggestion" => {
                if !is_admin_user && !is_team && p.author_id != user_id { continue; }
            }
            "announcement" => {
                if !is_admin_user && !is_team && !p.public { continue; }
            }
            _ => continue,
        }

        // ── Kind filter ──
        if let Some(ref kind) = query.kind {
            if *kind != p.kind { continue; }
        }

        // ── Tag filter (only discussions have tags) ──
        if !filter_tags.is_empty() {
            if p.kind != "discussion" { continue; }
            if !p.tags.iter().any(|t| filter_tags.contains(t)) { continue; }
        }

        let author_name = users.get(&p.author_id)
            .map(|u| u.display_name.clone())
            .unwrap_or_else(|| p.author_name.clone());
        let author_avatar_url = users.get(&p.author_id).and_then(|u| u.avatar_url.clone());

        let content_preview = p.content.chars().take(200).collect::<String>();

        let detail_url = match p.kind.as_str() {
            "discussion" => format!("/discussions/{}", p.id),
            "suggestion" => format!("/suggestions/{}", p.id),
            _ => format!("/announcements/{}", p.id),
        };

        result.push(PostListItem {
            id: p.id.clone(),
            kind: p.kind.clone(),
            title: p.title.clone(),
            content_preview,
            author_id: p.author_id.clone(),
            author_name,
            author_avatar_url,
            tags: p.tags.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
            pinned: p.pinned,
            status: if p.kind == "suggestion" { Some(p.status.clone()) } else { None },
            public: if p.kind == "announcement" { Some(p.public) } else { None },
            team_only: if p.kind == "discussion" { Some(p.team_only) } else { None },
            reply_count: p.replies.len(),
            detail_url,
        });
    }

    // Sort: pinned first, then by updated_at descending
    result.sort_by(|a, b| {
        b.pinned.cmp(&a.pinned)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });

    Json(serde_json::json!(result))
}
