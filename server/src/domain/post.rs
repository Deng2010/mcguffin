use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified post type — categorization is done via `tags` field only.
/// There is no "kind" distinction; "公告", "建议" etc are just tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // ── Categorization ──
    #[serde(default)]
    pub tags: Vec<String>,

    // ── Common flags ──
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub team_only: bool,

    // ── Discussion features ──
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub reactions: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub replies: Vec<PostReply>,
    #[serde(default)]
    pub mentioned_user_ids: Vec<String>,

    // ── Review status (suggestion-style, applicable to any post) ──
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub visible_to: Vec<String>,
    #[serde(default)]
    pub editable_by: Vec<String>,
}

/// Unified reply type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostReply {
    pub id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub reactions: HashMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionTag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: String,
    #[serde(default)]
    pub admin_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionEmoji {
    pub id: String,
    pub name: String,
    /// unicode single character
    pub char: String,
}

#[derive(Deserialize)]
pub struct CreatePostPayload {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub pinned: Option<bool>,
    #[serde(default)]
    pub team_only: Option<bool>,
    /// User IDs of team members @mentioned in the content
    #[serde(default)]
    pub mentioned_user_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdatePostPayload {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub emoji: Option<Option<String>>,
    #[serde(default)]
    pub pinned: Option<bool>,
    #[serde(default)]
    pub team_only: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct CreatePostReplyPayload {
    pub content: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    /// User IDs of team members @mentioned in the content
    #[serde(default)]
    pub mentioned_user_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct ReactPayload {
    pub emoji: String,
}

#[derive(Deserialize)]
pub struct CreateDiscussionTagPayload {
    pub name: String,
    pub color: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct UpdateDiscussionTagPayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateDiscussionEmojiPayload {
    pub name: String,
    pub char: String,
}

#[derive(Deserialize)]
pub struct UpdateDiscussionEmojiPayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub char: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostListItem {
    pub id: String,
    pub title: String,
    pub content_preview: String,
    pub author_id: String,
    pub author_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_avatar_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub team_only: bool,
    #[serde(default)]
    pub reply_count: usize,
    /// Detail URL path (e.g. "/post/id")
    pub detail_url: String,
}

#[derive(Deserialize)]
pub struct CommunityQuery {
    #[serde(default)]
    pub tags: Option<String>, // comma-separated tag IDs
    #[serde(default)]
    pub tag: Option<String>, // single tag (backward compat)
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}
fn default_limit() -> u32 {
    10
}
