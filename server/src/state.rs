use crate::types::{
    AppConfig, AuditEntry, Contest, DiscussionEmoji, DiscussionTag, JoinRequest, MemberGroup,
    Notification, Post, Problem, SessionEntry, TeamMember, User,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub const ADMIN_USER_ID: &str = "admin";

/// 上次保存到 JSON 的时间戳（秒），用于节流
static LAST_JSON_SAVE: AtomicI64 = AtomicI64::new(0);
/// JSON 文件保存最小间隔（秒）
const JSON_SAVE_INTERVAL: i64 = 5;

/// Resolve the config file path with platform awareness.
/// Priority: CWD 的 mcguffin.toml/config.toml > 平台默认路径。
pub fn resolve_config_path() -> PathBuf {
    // 1. CWD 探索（开发环境便利）
    for name in &["mcguffin.toml", "config.toml"] {
        let cwd_path = PathBuf::from(name);
        if cwd_path.exists() {
            return cwd_path;
        }
    }
    // 2. 平台默认
    default_config_path()
}

/// 平台默认配置文件路径
fn default_config_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let system = PathBuf::from("/usr/share/mcguffin/config.toml");
        if system.exists() {
            return system;
        }
        if let Some(home) = std::env::var_os("HOME") {
            let user = PathBuf::from(home).join(".config/mcguffin/config.toml");
            if user.exists() {
                return user;
            }
        }
        system
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join("Library/Application Support/mcguffin/config.toml")
        } else {
            PathBuf::from("/usr/share/mcguffin/config.toml")
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata).join("mcguffin/config.toml")
        } else {
            PathBuf::from("C:/ProgramData/mcguffin/config.toml")
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/usr/share/mcguffin/config.toml")
    }
}

// ============== Persistence Format ==============

#[derive(Serialize, Deserialize)]
pub(crate) struct SavedData {
    pub(crate) users: HashMap<String, User>,
    /// token → SessionEntry (contains user_id + last_active).
    /// Custom deserializer handles both old format (token→String) and new (token→SessionEntry).
    #[serde(deserialize_with = "deserialize_sessions", default)]
    pub(crate) sessions: HashMap<String, SessionEntry>,
    #[serde(default)]
    pub(crate) refresh_tokens: HashMap<String, String>,
    pub(crate) team_members: HashMap<String, TeamMember>,
    pub(crate) problems: HashMap<String, Problem>,
    pub(crate) join_requests: HashMap<String, JoinRequest>,
    #[serde(default)]
    pub(crate) contests: HashMap<String, Contest>,
    #[serde(default)]
    pub(crate) site_description: String,
    #[serde(default)]
    pub(crate) notifications: HashMap<String, Notification>,
    #[serde(default)]
    pub(crate) showcase_problem_ids: Vec<String>,
    #[serde(default)]
    pub(crate) showcase_contest_ids: Vec<String>,

    // ── Unified posts (primary storage) ──
    #[serde(default)]
    pub(crate) posts: HashMap<String, Post>,

    // ── Legacy fields (kept for backward-compat deserialization) ──
    // These are read on load IF posts is empty, then migrated.
    #[serde(default)]
    pub(crate) suggestions: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub(crate) announcements: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub(crate) discussions: HashMap<String, serde_json::Value>,

    // ── Permission Groups ──
    #[serde(default)]
    pub(crate) member_groups: HashMap<String, MemberGroup>,
}

/// Migrate legacy data into unified posts.
/// Handles both old Post format (with `kind` field) and legacy maps.
fn migrate_legacy_data(
    posts: &mut HashMap<String, Post>,
    suggestions: &HashMap<String, serde_json::Value>,
    announcements: &HashMap<String, serde_json::Value>,
    discussions: &HashMap<String, serde_json::Value>,
) -> bool {
    let mut migrated = false;

    // Helper to convert a legacy value to a Post
    let extract = |v: &serde_json::Value, field: &str| -> String {
        v.get(field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let extract_bool = |v: &serde_json::Value, field: &str, default: bool| -> bool {
        v.get(field).and_then(|v| v.as_bool()).unwrap_or(default)
    };

    for (id, v) in suggestions {
        if !posts.contains_key(id) {
            let now = Utc::now();
            let created_at = v
                .get("created_at")
                .and_then(|c| {
                    c.as_str().and_then(|s| {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                })
                .unwrap_or(now);
            let updated_at = v
                .get("updated_at")
                .and_then(|c| {
                    c.as_str().and_then(|s| {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                })
                .unwrap_or(now);
            // Convert legacy SuggestionReply if any
            let replies: Vec<crate::types::PostReply> = v
                .get("replies")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|rv| {
                            let aid = extract(rv, "author_id");
                            let an = extract(rv, "author_name");
                            crate::types::PostReply {
                                id: extract(rv, "id"),
                                author_id: aid.clone(),
                                author_name: an,
                                content: extract(rv, "content"),
                                created_at: rv
                                    .get("created_at")
                                    .and_then(|c| {
                                        c.as_str().and_then(|s| {
                                            DateTime::parse_from_rfc3339(s)
                                                .ok()
                                                .map(|dt| dt.with_timezone(&Utc))
                                        })
                                    })
                                    .unwrap_or(now),
                                reactions: HashMap::new(),
                                parent_id: None,
                                reply_to: None,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            posts.insert(
                id.clone(),
                Post {
                    id: id.clone(),
                    title: extract(v, "title"),
                    content: extract(v, "content"),
                    author_id: extract(v, "author_id"),
                    author_name: extract(v, "author_name"),
                    tags: vec!["建议".to_string()],
                    pinned: false,
                    team_only: false,
                    emoji: None,
                    reactions: HashMap::new(),
                    replies,
                    mentioned_user_ids: vec![],
                    status: extract(v, "status"),
                    created_at,
                    updated_at,
                    visible_to: vec![],
                    editable_by: vec![],
                },
            );
            migrated = true;
        }
    }

    for (id, v) in announcements {
        if !posts.contains_key(id) {
            let now = Utc::now();
            let created_at = v
                .get("created_at")
                .and_then(|c| {
                    c.as_str().and_then(|s| {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                })
                .unwrap_or(now);
            let updated_at = v
                .get("updated_at")
                .and_then(|c| {
                    c.as_str().and_then(|s| {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                })
                .unwrap_or(now);
            posts.insert(
                id.clone(),
                Post {
                    id: id.clone(),
                    title: extract(v, "title"),
                    content: extract(v, "content"),
                    author_id: extract(v, "author_id"),
                    author_name: extract(v, "author_name"),
                    tags: vec!["公告".to_string()],
                    pinned: extract_bool(v, "pinned", false),
                    team_only: false,
                    emoji: None,
                    reactions: HashMap::new(),
                    replies: vec![],
                    mentioned_user_ids: vec![],
                    status: String::new(),
                    created_at,
                    updated_at,
                    visible_to: vec![],
                    editable_by: vec![],
                },
            );
            migrated = true;
        }
    }

    for (id, v) in discussions {
        if !posts.contains_key(id) {
            let now = Utc::now();
            let created_at = v
                .get("created_at")
                .and_then(|c| {
                    c.as_str().and_then(|s| {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                })
                .unwrap_or(now);
            let updated_at = v
                .get("updated_at")
                .and_then(|c| {
                    c.as_str().and_then(|s| {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                })
                .unwrap_or(now);
            let tags: Vec<String> = v
                .get("tags")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let emoji: Option<String> = v
                .get("emoji")
                .and_then(|e| e.as_str().map(|s| s.to_string()));
            let team_only = extract_bool(v, "team_only", false);
            let pinned = extract_bool(v, "pinned", false);
            let reactions: HashMap<String, Vec<String>> = v
                .get("reactions")
                .and_then(|r| serde_json::from_value(r.clone()).ok())
                .unwrap_or_default();
            let replies: Vec<crate::types::PostReply> = v
                .get("replies")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|rv| {
                            let aid = extract(rv, "author_id");
                            let an = extract(rv, "author_name");
                            let reactions: HashMap<String, Vec<String>> = rv
                                .get("reactions")
                                .and_then(|r| serde_json::from_value(r.clone()).ok())
                                .unwrap_or_default();
                            crate::types::PostReply {
                                id: extract(rv, "id"),
                                author_id: aid.clone(),
                                author_name: an,
                                content: extract(rv, "content"),
                                created_at: rv
                                    .get("created_at")
                                    .and_then(|c| {
                                        c.as_str().and_then(|s| {
                                            DateTime::parse_from_rfc3339(s)
                                                .ok()
                                                .map(|dt| dt.with_timezone(&Utc))
                                        })
                                    })
                                    .unwrap_or(now),
                                reactions,
                                parent_id: rv
                                    .get("parent_id")
                                    .and_then(|p| p.as_str().map(|s| s.to_string())),
                                reply_to: rv
                                    .get("reply_to")
                                    .and_then(|p| p.as_str().map(|s| s.to_string())),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            posts.insert(
                id.clone(),
                Post {
                    id: id.clone(),
                    title: extract(v, "title"),
                    content: extract(v, "content"),
                    author_id: extract(v, "author_id"),
                    author_name: extract(v, "author_name"),
                    tags,
                    pinned,
                    team_only,
                    emoji,
                    reactions,
                    replies,
                    mentioned_user_ids: vec![],
                    status: String::new(),
                    created_at,
                    updated_at,
                    visible_to: vec![],
                    editable_by: vec![],
                },
            );
            migrated = true;
        }
    }

    migrated
}

/// Custom deserializer for sessions that handles both old format
/// (`HashMap<String, String>` — just user_id) and new format
/// (`HashMap<String, SessionEntry>` — object with user_id + last_active).
fn deserialize_sessions<'de, D>(deserializer: D) -> Result<HashMap<String, SessionEntry>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Object(map) => {
            let mut sessions = HashMap::new();
            for (token, val) in map {
                match val {
                    // Old format: "token" → "user_id" (string)
                    serde_json::Value::String(user_id) => {
                        sessions.insert(
                            token,
                            SessionEntry {
                                user_id,
                                last_active: Utc::now(),
                            },
                        );
                    }
                    // New format: "token" → {"user_id": "...", "last_active": "..."}
                    serde_json::Value::Object(obj) => {
                        let user_id = obj
                            .get("user_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .ok_or_else(|| D::Error::custom("missing user_id in session entry"))?;
                        let last_active = obj
                            .get("last_active")
                            .and_then(|v| v.as_str())
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now);
                        sessions.insert(
                            token,
                            SessionEntry {
                                user_id,
                                last_active,
                            },
                        );
                    }
                    _ => {}
                }
            }
            Ok(sessions)
        }
        _ => Ok(HashMap::new()),
    }
}

#[derive(Clone)]
pub struct AppState {
    pub users: Arc<RwLock<HashMap<String, User>>>,
    /// token → SessionEntry (user_id + last_active timestamp)
    pub sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
    pub refresh_tokens: Arc<RwLock<HashMap<String, String>>>,
    pub team_members: Arc<RwLock<HashMap<String, TeamMember>>>,
    pub problems: Arc<RwLock<HashMap<String, Problem>>>,
    pub join_requests: Arc<RwLock<HashMap<String, JoinRequest>>>,
    pub contests: Arc<RwLock<HashMap<String, Contest>>>,
    pub cpoauth_client_id: String,
    pub cpoauth_client_secret: String,
    pub cpoauth_redirect_uri: String,
    pub admin_password: String,
    pub site_name: String,
    pub site_title: String,
    pub site_version: String,
    pub site_description: Arc<RwLock<String>>,
    /// Public-facing site URL (e.g. https://lba-oi.team)
    pub site_url: String,
    /// Path to the JSON data persistence file
    pub data_file: String,
    /// Customizable difficulty levels
    pub difficulty: Arc<RwLock<crate::types::DifficultyConfig>>,
    /// Unified posts (replaces suggestions, announcements, discussions)
    pub posts: Arc<RwLock<HashMap<String, Post>>>,
    /// Notifications
    pub notifications: Arc<RwLock<HashMap<String, Notification>>>,
    /// Showcase selections
    pub showcase_problem_ids: Arc<RwLock<Vec<String>>>,
    pub showcase_contest_ids: Arc<RwLock<Vec<String>>>,
    /// Difficulty display order
    pub difficulty_order: Arc<RwLock<Vec<String>>>,
    /// Discussion tags
    pub discussion_tags: Arc<RwLock<HashMap<String, DiscussionTag>>>,
    /// Discussion emojis
    pub discussion_emojis: Arc<RwLock<HashMap<String, DiscussionEmoji>>>,
    /// Role→permissions mapping (loaded from config or defaults)
    pub role_permissions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Member groups for group-based permission assignment
    pub member_groups: Arc<RwLock<HashMap<String, MemberGroup>>>,
    /// SQLite 连接池（双写模式：SQLite + HashMap）
    pub db: SqlitePool,
}

impl AppState {
    pub async fn new() -> Self {
        // Load config from /usr/share/mcguffin/config.toml
        let mut config = load_config();
        let difficulty_config = load_difficulty_config(&config);

        // Migration: if config.toml has no [permissions] section, write default role permissions
        {
            let raw_config = std::fs::read_to_string(resolve_config_path()).unwrap_or_default();
            let has_permissions_section =
                raw_config.contains("\n[permissions]") || raw_config.starts_with("[permissions]");
            if !has_permissions_section {
                tracing::info!(
                    "No [permissions] section in config.toml, writing default permissions"
                );
                let defaults = crate::types::default_role_permissions();
                if let Ok(raw) = std::fs::read_to_string(resolve_config_path()) {
                    use std::str::FromStr;
                    use toml_edit::{DocumentMut, Item, Value as TomlValue};
                    if let Ok(mut doc) = DocumentMut::from_str(&raw) {
                        // Ensure [permissions] table exists
                        if doc.get("permissions").is_none() {
                            doc["permissions"] = Item::Table(toml_edit::Table::new());
                        }
                        // Write [permissions.roles]
                        doc["permissions"]["roles"] = Item::Table(toml_edit::Table::new());
                        if let Some(roles_t) = doc["permissions"]["roles"].as_table_mut() {
                            for (role, perms) in &defaults {
                                if !perms.is_empty() {
                                    let arr = toml_edit::Array::from_iter(
                                        perms.iter().map(|p| TomlValue::from(p.as_str())),
                                    );
                                    roles_t[role] = Item::Value(TomlValue::Array(arr));
                                }
                            }
                        }
                        let _ = std::fs::write(resolve_config_path(), doc.to_string());
                        tracing::info!("Default permissions written to config.toml");
                    }
                }
                // Reload config to pick up the newly written defaults
                config = load_config();
            }
        }

        let site_version = env!("CARGO_PKG_VERSION").to_string();

        // ── SQLite 初始化 ──
        let data_file = &config.server.data_file;
        let db_path = std::path::Path::new(data_file)
            .with_extension("db")
            .to_string_lossy()
            .to_string();
        let db = crate::db::init_db(&db_path)
            .await
            .expect("SQLite 初始化失败，请检查数据库文件路径和权限");

        // Load saved JSON data
        let saved = std::fs::read_to_string(data_file)
            .ok()
            .and_then(|s| serde_json::from_str::<SavedData>(&s).ok());

        // ── 数据加载策略：SQLite 优先，JSON 回退 ──
        let sqlite_active = crate::db::sqlite_has_data(&db).await;
        if !sqlite_active {
            // SQLite 为空且 JSON 存在 → 首次导入
            if let Some(ref saved_data) = saved {
                match crate::db::import_saved_data(&db, saved_data).await {
                    Ok(n) if n > 0 => {
                        tracing::info!("已从 {} 导入 {} 条记录到 SQLite", data_file, n);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("从 JSON 导入 SQLite 失败（可忽略）: {}", e);
                    }
                }
            }
        } else {
            tracing::info!("SQLite 已有数据，优先从数据库加载");
        }

        // Discussion tags and emojis come from config.toml, not saved data
        let discussion_tags = load_discussion_tags(&config);
        let discussion_emojis = load_discussion_emojis(&config);

        let (
            mut users,
            sessions,
            refresh_tokens,
            mut team_members,
            problems,
            join_requests,
            contests,
            site_description,
            notifications,
            showcase_problem_ids,
            showcase_contest_ids,
            posts,
        ) = if let Some(data) = saved {
            tracing::info!("Loaded state from {}", data_file);

            // Migrate legacy data if needed
            let mut p = data.posts;
            let migrated = migrate_legacy_data(
                &mut p,
                &data.suggestions,
                &data.announcements,
                &data.discussions,
            );
            if migrated {
                tracing::info!("Migrated legacy data to unified posts model");
            }

            // Migration: if data.json has member_groups, write to config.toml
            if !data.member_groups.is_empty() && config.permission_groups.is_empty() {
                tracing::info!(
                    "Migrating {} member groups from data.json to config.toml",
                    data.member_groups.len()
                );
                // Write to config.toml via the admin helper
                let groups_json: Vec<serde_json::Value> = data
                    .member_groups
                    .values()
                    .map(|g| {
                        serde_json::json!({
                            "id": g.id,
                            "name": g.name,
                            "permissions": g.permissions,
                        })
                    })
                    .collect();
                if let Ok(raw) = std::fs::read_to_string(resolve_config_path()) {
                    use std::str::FromStr;
                    use toml_edit::{DocumentMut, Item, Value as TomlValue};
                    if let Ok(mut doc) = DocumentMut::from_str(&raw) {
                        if let Some(perms_root) =
                            doc.get_mut("permissions").and_then(|s| s.as_table_mut())
                        {
                            // Clear old groups section
                            if let Some(groups_t) =
                                perms_root.get_mut("groups").and_then(|s| s.as_table_mut())
                            {
                                let keys: Vec<String> =
                                    groups_t.iter().map(|(k, _)| k.to_string()).collect();
                                for k in keys {
                                    groups_t.remove(&k);
                                }
                            } else {
                                // Ensure groups sub-table exists
                                perms_root["groups"] = Item::Table(toml_edit::Table::new());
                            }
                            // Write migrated groups
                            if let Some(groups_t) =
                                perms_root.get_mut("groups").and_then(|s| s.as_table_mut())
                            {
                                for g in &groups_json {
                                    let id = g.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                    let name = g.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    let perms = g.get("permissions").and_then(|v| v.as_array());
                                    if id.is_empty() || name.is_empty() {
                                        continue;
                                    }
                                    let mut it = toml_edit::InlineTable::new();
                                    it.insert("name", TomlValue::from(name));
                                    if let Some(arr) = perms {
                                        let t_arr = toml_edit::Array::from_iter(
                                            arr.iter()
                                                .filter_map(|v| v.as_str().map(TomlValue::from)),
                                        );
                                        it.insert("permissions", TomlValue::Array(t_arr));
                                    }
                                    groups_t[id] = Item::Value(TomlValue::InlineTable(it));
                                }
                            }
                        }
                        let _ = std::fs::write(resolve_config_path(), doc.to_string());
                    }
                }
                // Also update in-memory member_groups
                // (loaded from config.toml again below — handled by load_member_groups fallback)
            }

            (
                data.users,
                data.sessions,
                data.refresh_tokens,
                data.team_members,
                data.problems,
                data.join_requests,
                data.contests,
                data.site_description,
                data.notifications,
                data.showcase_problem_ids,
                data.showcase_contest_ids,
                p,
            )
        } else {
            tracing::info!("No saved state, using default seed data");
            (
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                Self::default_team_members(),
                Self::default_problems(),
                HashMap::new(),
                HashMap::new(),
                String::new(),
                HashMap::new(),
                Vec::new(),
                Vec::new(),
                HashMap::new(),
            )
        };

        // Member groups come from config.toml (already migrated from data.json if needed)
        // Reload config after potential migration
        let config = load_config();
        let member_groups = load_member_groups(&config);

        // Always ensure superadmin user exists AND has correct role
        let admin_display_name = &config.admin.display_name;
        users.entry(ADMIN_USER_ID.to_string()).or_insert(User {
            id: ADMIN_USER_ID.to_string(),
            username: "admin".to_string(),
            display_name: admin_display_name.clone(),
            avatar_url: None,
            email: None,
            role: "superadmin".to_string(),
            team_status: "joined".to_string(),
            created_at: Utc::now(),
            bio: String::new(),
            password_hash: None,
            effective_role: "superadmin".to_string(),
            group_ids: Vec::new(),
            user_permissions: Vec::new(),
        });
        // Force update role to superadmin (in case loaded from old data)
        if let Some(u) = users.get_mut(ADMIN_USER_ID) {
            u.role = "superadmin".to_string();
            u.display_name = admin_display_name.clone();
        }

        // Migration: convert role="pending" to role="guest" (pending role removed)
        let mut pending_count = 0;
        for u in users.values_mut() {
            if u.role == "pending" {
                u.role = "guest".to_string();
                pending_count += 1;
            }
        }
        if pending_count > 0 {
            tracing::info!(
                "Migrated {} users from role=pending to role=guest",
                pending_count
            );
        }

        // Always ensure superadmin is a team member AND has correct role
        team_members
            .entry(ADMIN_USER_ID.to_string())
            .or_insert(TeamMember {
                id: ADMIN_USER_ID.to_string(),
                user_id: ADMIN_USER_ID.to_string(),
                joined_at: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc)
                    .format("%Y-%m-%d")
                    .to_string(),
            });

        let redirect_uri = format!("{}/api/oauth/callback", config.server.site_url);

        // Load role→permissions from config or use defaults
        let role_permissions: HashMap<String, Vec<String>> = if !config.permissions.is_empty() {
            // Validate permission names in config
            for (role, perms) in &config.permissions {
                for p in perms {
                    if p != crate::types::PERM_WILDCARD
                        && !crate::types::perms::ALL.contains(&p.as_str())
                    {
                        tracing::warn!(
                            "配置中的权限名「{}」（角色: {}）不在已知权限列表中，将被忽略",
                            p,
                            role
                        );
                    }
                }
            }
            config.permissions.clone()
        } else {
            crate::types::default_role_permissions()
        };

        let app_state = Self {
            users: Arc::new(RwLock::new(users)),
            sessions: Arc::new(RwLock::new(sessions)),
            refresh_tokens: Arc::new(RwLock::new(refresh_tokens)),
            team_members: Arc::new(RwLock::new(team_members)),
            problems: Arc::new(RwLock::new(problems)),
            join_requests: Arc::new(RwLock::new(join_requests)),
            contests: Arc::new(RwLock::new(contests)),
            cpoauth_client_id: config.oauth.cp_client_id,
            cpoauth_client_secret: config.oauth.cp_client_secret,
            cpoauth_redirect_uri: redirect_uri,
            admin_password: config.admin.password,
            site_name: config
                .site
                .name
                .clone()
                .unwrap_or_else(|| "McGuffin".to_string()),
            site_title: config
                .site
                .title
                .unwrap_or_else(|| config.site.name.unwrap_or_else(|| "McGuffin".to_string())),
            site_version,
            site_description: Arc::new(RwLock::new(site_description)),
            site_url: config.server.site_url,
            data_file: config.server.data_file,
            difficulty: Arc::new(RwLock::new(difficulty_config.clone())),
            posts: Arc::new(RwLock::new(posts)),
            notifications: Arc::new(RwLock::new(notifications)),
            showcase_problem_ids: Arc::new(RwLock::new(showcase_problem_ids)),
            showcase_contest_ids: Arc::new(RwLock::new(showcase_contest_ids)),
            difficulty_order: Arc::new(RwLock::new(
                config.site.difficulty_order.clone().unwrap_or_else(|| {
                    let mut keys: Vec<String> = difficulty_config.levels.keys().cloned().collect();
                    keys.sort();
                    keys
                }),
            )),
            discussion_tags: Arc::new(RwLock::new(discussion_tags)),
            discussion_emojis: Arc::new(RwLock::new(discussion_emojis)),
            role_permissions: Arc::new(RwLock::new(role_permissions)),
            member_groups: Arc::new(RwLock::new(member_groups)),
            db,
        };

        // 如果 SQLite 已有数据，导出到 JSON 以保持同步
        if sqlite_active {
            app_state.export_sqlite_to_json().await;
            tracing::info!("已将 SQLite 数据同步到 JSON 文件");
        }

        app_state
    }

    /// 持久化所有内存状态到 JSON 文件（每秒最多一次）
    pub async fn save(&self) {
        // 节流：每秒最多写一次
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let last = LAST_JSON_SAVE.load(Ordering::Relaxed);
        if now - last < JSON_SAVE_INTERVAL {
            return;
        }
        LAST_JSON_SAVE.store(now, Ordering::Relaxed);

        // 确保 JSON 文件父目录存在
        let data_path = std::path::Path::new(&self.data_file);
        if let Some(parent) = data_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let data = SavedData {
            users: self.users.read().await.clone(),
            sessions: self.sessions.read().await.clone(),
            refresh_tokens: self.refresh_tokens.read().await.clone(),
            team_members: self.team_members.read().await.clone(),
            problems: self.problems.read().await.clone(),
            join_requests: self.join_requests.read().await.clone(),
            contests: self.contests.read().await.clone(),
            site_description: self.site_description.read().await.clone(),
            notifications: self.notifications.read().await.clone(),
            showcase_problem_ids: self.showcase_problem_ids.read().await.clone(),
            showcase_contest_ids: self.showcase_contest_ids.read().await.clone(),
            posts: self.posts.read().await.clone(),
            suggestions: HashMap::new(),
            announcements: HashMap::new(),
            discussions: HashMap::new(),
            member_groups: self.member_groups.read().await.clone(),
        };
        let json = serde_json::to_string_pretty(&data).unwrap();
        let tmp_path = format!("{}.tmp", self.data_file);
        // Write to temp file first, then atomically rename
        // This prevents data corruption if the process crashes mid-write
        if std::fs::write(&tmp_path, &json).is_ok() {
            let _ = std::fs::rename(&tmp_path, &self.data_file);
        }
    }

    const MAX_SESSIONS_PER_USER: usize = 3;

    /// 记录审计日志条目到 SQLite。
    pub async fn log_audit(&self, entry: AuditEntry) {
        let _ = sqlx::query(
            "INSERT INTO audit_log (timestamp, user_id, user_name, action, resource, result, reason) \
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(entry.timestamp.to_rfc3339())
        .bind(&entry.user_id)
        .bind(&entry.user_name)
        .bind(&entry.action)
        .bind(&entry.resource)
        .bind(&entry.result)
        .bind(&entry.reason)
        .execute(&self.db)
        .await;
    }

    /// 设置 refresh token（双写：HashMap + SQLite）
    pub async fn set_refresh_token(&self, token: String, user_id: String) {
        self.refresh_tokens
            .write()
            .await
            .insert(token.clone(), user_id.clone());
        let _ = sqlx::query("INSERT OR REPLACE INTO refresh_tokens (token, user_id) VALUES (?, ?)")
            .bind(&token)
            .bind(&user_id)
            .execute(&self.db)
            .await;
    }

    /// 删除 refresh token（双写）
    pub async fn remove_refresh_token(&self, token: &str) {
        self.refresh_tokens.write().await.remove(token);
        let _ = sqlx::query("DELETE FROM refresh_tokens WHERE token = ?")
            .bind(token)
            .execute(&self.db)
            .await;
    }

    /// 清除指定用户的所有 refresh token（双写）
    pub async fn clear_user_refresh_tokens(&self, user_id: &str) {
        self.refresh_tokens
            .write()
            .await
            .retain(|_, uid| uid != user_id);
        let _ = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// Create a session for the given user, automatically evicting the oldest
    /// session if they already have MAX_SESSIONS_PER_USER sessions.
    /// 双写：HashMap + SQLite
    pub async fn create_session(&self, user_id: &str) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_rfc = now.to_rfc3339();

        // HashMap 写入
        {
            let mut sessions = self.sessions.write().await;

            // Collect sessions for this user, sorted oldest-first
            let mut user_sessions: Vec<(String, chrono::DateTime<Utc>)> = sessions
                .iter()
                .filter(|(_, e)| e.user_id == user_id)
                .map(|(token, e)| (token.clone(), e.last_active))
                .collect();
            user_sessions.sort_by_key(|(_, t)| *t);

            // Evict oldest sessions beyond the limit
            while user_sessions.len() >= Self::MAX_SESSIONS_PER_USER {
                let (oldest_token, _) = user_sessions.remove(0);
                sessions.remove(&oldest_token);
            }

            sessions.insert(
                token.clone(),
                SessionEntry {
                    user_id: user_id.to_string(),
                    last_active: now,
                },
            );
        }

        // SQLite 写入（双写）
        let _ = sqlx::query("INSERT INTO sessions (token, user_id, last_active) VALUES (?, ?, ?)")
            .bind(&token)
            .bind(user_id)
            .bind(&now_rfc)
            .execute(&self.db)
            .await;

        // 清理旧 session（SQLite 端）
        let _ = sqlx::query(
            "DELETE FROM sessions WHERE token IN (\
             SELECT token FROM sessions WHERE user_id = ? \
             ORDER BY last_active ASC \
             LIMIT -1 OFFSET ?
            )",
        )
        .bind(user_id)
        .bind(Self::MAX_SESSIONS_PER_USER as i32 - 1)
        .execute(&self.db)
        .await;

        token
    }

    /// 更新 session 的最后活跃时间（双写）
    /// 返回更新是否成功
    pub async fn touch_session(&self, token: &str, now: &str) -> bool {
        let updated = {
            let mut sessions = self.sessions.write().await;
            if let Some(entry) = sessions.get_mut(token) {
                entry.last_active = Utc::now();
                true
            } else {
                false
            }
        };
        if updated {
            let _ = sqlx::query("UPDATE sessions SET last_active = ? WHERE token = ?")
                .bind(now)
                .bind(token)
                .execute(&self.db)
                .await;
        }
        updated
    }

    /// 插入通知（双写）
    pub async fn insert_notification(&self, notification: &Notification) {
        self.notifications
            .write()
            .await
            .insert(notification.id.clone(), notification.clone());
        let _ = sqlx::query(
            "INSERT INTO notifications (id, user_id, title, body, read, created_at, link) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&notification.id)
        .bind(&notification.user_id)
        .bind(&notification.title)
        .bind(&notification.body)
        .bind(notification.read as i32)
        .bind(notification.created_at.to_rfc3339())
        .bind(&notification.link)
        .execute(&self.db)
        .await;
    }

    /// 标记通知为已读（双写）
    pub async fn mark_notification_read(&self, notification_id: &str) {
        if let Some(n) = self.notifications.write().await.get_mut(notification_id) {
            n.read = true;
        }
        let _ = sqlx::query("UPDATE notifications SET read = 1 WHERE id = ?")
            .bind(notification_id)
            .execute(&self.db)
            .await;
    }

    /// 标记用户所有通知为已读（双写）
    pub async fn mark_all_user_notifications_read(&self, user_id: &str) {
        let mut notifications = self.notifications.write().await;
        for n in notifications.values_mut() {
            if n.user_id == user_id && !n.read {
                n.read = true;
            }
        }
        drop(notifications);
        let _ = sqlx::query("UPDATE notifications SET read = 1 WHERE user_id = ? AND read = 0")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    // ===== users 双写辅助方法 =====

    /// 插入或替换用户（双写）
    pub async fn upsert_user(&self, user: &User) {
        self.users
            .write()
            .await
            .insert(user.id.clone(), user.clone());
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO users \
             (id, username, display_name, avatar_url, email, role, team_status, \
              created_at, bio, password_hash, effective_role, group_ids, user_permissions) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(&user.email)
        .bind(&user.role)
        .bind(&user.team_status)
        .bind(user.created_at.to_rfc3339())
        .bind(&user.bio)
        .bind(&user.password_hash)
        .bind(&user.effective_role)
        .bind(serde_json::to_string(&user.group_ids).unwrap_or_default())
        .bind(serde_json::to_string(&user.user_permissions).unwrap_or_default())
        .execute(&self.db)
        .await;
    }

    /// 更新用户特定字段（双写）
    pub async fn update_user_field(&self, user_id: &str, field: &str, value: String) {
        // HashMap 更新
        {
            let mut users = self.users.write().await;
            if let Some(u) = users.get_mut(user_id) {
                match field {
                    "role" => u.role = value.clone(),
                    "team_status" => u.team_status = value.clone(),
                    "display_name" => u.display_name = value.clone(),
                    "avatar_url" => u.avatar_url = Some(value.clone()),
                    "bio" => u.bio = value.clone(),
                    "email" => u.email = Some(value.clone()),
                    "username" => u.username = value.clone(),
                    "password_hash" => u.password_hash = Some(value.clone()),
                    _ => {}
                }
            }
        }
        // SQLite 更新（使用动态 SQL，字段已校验）
        let _ = sqlx::query(&format!("UPDATE users SET {} = ? WHERE id = ?", field))
            .bind(&value)
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// 更新用户多个字段（双写）
    pub async fn update_user(&self, user: &User) {
        self.users
            .write()
            .await
            .insert(user.id.clone(), user.clone());
        let _ = sqlx::query(
            "UPDATE users SET username=?, display_name=?, avatar_url=?, email=?, \
             role=?, team_status=?, bio=?, password_hash=?, effective_role=?, \
             group_ids=?, user_permissions=? WHERE id=?",
        )
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(&user.email)
        .bind(&user.role)
        .bind(&user.team_status)
        .bind(&user.bio)
        .bind(&user.password_hash)
        .bind(&user.effective_role)
        .bind(serde_json::to_string(&user.group_ids).unwrap_or_default())
        .bind(serde_json::to_string(&user.user_permissions).unwrap_or_default())
        .bind(&user.id)
        .execute(&self.db)
        .await;
    }

    /// 删除用户（双写）
    pub async fn delete_user(&self, user_id: &str) {
        self.users.write().await.remove(user_id);
        let _ = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// 从用户列表中移除指定 group_id（双写）
    pub async fn remove_group_from_all_users(&self, group_id: &str) {
        {
            let mut users = self.users.write().await;
            for u in users.values_mut() {
                u.group_ids.retain(|g| g != group_id);
            }
        }
        // SQLite 端需要用 JSON 函数处理
        // 由于 SQLite JSON 操作较复杂，这里使用读取-修改-写入循环
        if let Ok(rows) =
            sqlx::query("SELECT id, group_ids FROM users WHERE json_array_length(group_ids) > 0")
                .fetch_all(&self.db)
                .await
        {
            use sqlx::Row;
            for row in rows {
                let uid: String = row.get("id");
                let gids: String = row.get("group_ids");
                let mut list: Vec<String> = serde_json::from_str(&gids).unwrap_or_default();
                list.retain(|g| g != group_id);
                let _ = sqlx::query("UPDATE users SET group_ids = ? WHERE id = ?")
                    .bind(serde_json::to_string(&list).unwrap_or_default())
                    .bind(&uid)
                    .execute(&self.db)
                    .await;
            }
        }
    }

    // ===== posts 双写辅助方法 =====

    /// 插入或替换帖子（双写）
    pub async fn upsert_post(&self, post: &Post) {
        self.posts
            .write()
            .await
            .insert(post.id.clone(), post.clone());
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO posts \
             (id, title, content, author_id, author_name, created_at, updated_at, \
              tags, pinned, team_only, emoji, reactions, replies, \
              mentioned_user_ids, status, visible_to, editable_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&post.id)
        .bind(&post.title)
        .bind(&post.content)
        .bind(&post.author_id)
        .bind(&post.author_name)
        .bind(post.created_at.to_rfc3339())
        .bind(post.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&post.tags).unwrap_or_default())
        .bind(post.pinned as i32)
        .bind(post.team_only as i32)
        .bind(&post.emoji)
        .bind(serde_json::to_string(&post.reactions).unwrap_or_default())
        .bind(serde_json::to_string(&post.replies).unwrap_or_default())
        .bind(serde_json::to_string(&post.mentioned_user_ids).unwrap_or_default())
        .bind(&post.status)
        .bind(serde_json::to_string(&post.visible_to).unwrap_or_default())
        .bind(serde_json::to_string(&post.editable_by).unwrap_or_default())
        .execute(&self.db)
        .await;
    }

    /// 删除帖子（双写）
    pub async fn delete_post_by_id(&self, post_id: &str) {
        self.posts.write().await.remove(post_id);
        let _ = sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(post_id)
            .execute(&self.db)
            .await;
    }

    // ===== problems 双写辅助方法 =====

    /// 插入题目（双写）
    pub async fn insert_problem(&self, problem: &Problem) {
        self.problems
            .write()
            .await
            .insert(problem.id.clone(), problem.clone());
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO problems \
             (id, title, author_id, author_name, contest, contest_id, difficulty, \
              content, solution, status, created_at, public_at, claimed_by, \
              verifier_solution, visible_to, link, remark, editable_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&problem.id)
        .bind(&problem.title)
        .bind(&problem.author_id)
        .bind(&problem.author_name)
        .bind(&problem.contest)
        .bind(&problem.contest_id)
        .bind(&problem.difficulty)
        .bind(&problem.content)
        .bind(&problem.solution)
        .bind(&problem.status)
        .bind(problem.created_at.to_rfc3339())
        .bind(problem.public_at.map(|t| t.to_rfc3339()))
        .bind(&problem.claimed_by)
        .bind(&problem.verifier_solution)
        .bind(serde_json::to_string(&problem.visible_to).unwrap_or_default())
        .bind(&problem.link)
        .bind(&problem.remark)
        .bind(serde_json::to_string(&problem.editable_by).unwrap_or_default())
        .execute(&self.db)
        .await;
    }

    /// 更新题目单个字段（双写）
    pub async fn update_problem_field<T: ToString + Send>(
        &self,
        problem_id: &str,
        field: &str,
        value: T,
    ) {
        let val_str = value.to_string();
        {
            let mut problems = self.problems.write().await;
            if let Some(p) = problems.get_mut(problem_id) {
                match field {
                    "status" => p.status = val_str.clone(),
                    "claimed_by" => {
                        p.claimed_by = if val_str.is_empty() {
                            None
                        } else {
                            Some(val_str.clone())
                        }
                    }
                    "verifier_solution" => {
                        p.verifier_solution = if val_str.is_empty() {
                            None
                        } else {
                            Some(val_str.clone())
                        }
                    }
                    "public_at" => p.public_at = None, // handled separately
                    _ => {}
                }
            }
        }
        let _ = sqlx::query(&format!("UPDATE problems SET {} = ? WHERE id = ?", field))
            .bind(&val_str)
            .bind(problem_id)
            .execute(&self.db)
            .await;
    }

    /// 删除题目（双写）
    pub async fn delete_problem_by_id(&self, problem_id: &str) {
        self.problems.write().await.remove(problem_id);
        let _ = sqlx::query("DELETE FROM problems WHERE id = ?")
            .bind(problem_id)
            .execute(&self.db)
            .await;
    }

    /// 清除所有题目中指定 contest 的引用（双写）
    pub async fn clear_contest_from_problems(&self, contest_id: &str) {
        {
            let mut problems = self.problems.write().await;
            for p in problems.values_mut() {
                if p.contest_id.as_deref() == Some(contest_id) {
                    p.contest = String::new();
                    p.contest_id = None;
                }
            }
        }
        let _ =
            sqlx::query("UPDATE problems SET contest = '', contest_id = NULL WHERE contest_id = ?")
                .bind(contest_id)
                .execute(&self.db)
                .await;
    }

    // ===== contests 双写辅助方法 =====

    /// 插入比赛（双写）
    pub async fn insert_contest(&self, contest: &Contest) {
        self.contests
            .write()
            .await
            .insert(contest.id.clone(), contest.clone());
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO contests \
             (id, name, start_time, end_time, description, created_by, created_at, \
              status, link, problem_order, visible_to, editable_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&contest.id)
        .bind(&contest.name)
        .bind(&contest.start_time)
        .bind(&contest.end_time)
        .bind(&contest.description)
        .bind(&contest.created_by)
        .bind(contest.created_at.to_rfc3339())
        .bind(&contest.status)
        .bind(&contest.link)
        .bind(serde_json::to_string(&contest.problem_order).unwrap_or_default())
        .bind(serde_json::to_string(&contest.visible_to).unwrap_or_default())
        .bind(serde_json::to_string(&contest.editable_by).unwrap_or_default())
        .execute(&self.db)
        .await;
    }

    /// 删除比赛（双写）
    pub async fn delete_contest_by_id(&self, contest_id: &str) {
        self.contests.write().await.remove(contest_id);
        let _ = sqlx::query("DELETE FROM contests WHERE id = ?")
            .bind(contest_id)
            .execute(&self.db)
            .await;
    }

    // ===== team_members 双写辅助方法 =====

    /// 插入团队成员（双写）
    pub async fn insert_team_member(&self, member: &TeamMember) {
        self.team_members
            .write()
            .await
            .insert(member.id.clone(), member.clone());
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO team_members (id, user_id, joined_at) VALUES (?, ?, ?)",
        )
        .bind(&member.id)
        .bind(&member.user_id)
        .bind(&member.joined_at)
        .execute(&self.db)
        .await;
    }

    /// 按成员 ID 删除团队成员（双写）
    pub async fn remove_team_member_by_id(&self, member_id: &str) {
        self.team_members.write().await.remove(member_id);
        let _ = sqlx::query("DELETE FROM team_members WHERE id = ?")
            .bind(member_id)
            .execute(&self.db)
            .await;
    }

    /// 按用户 ID 删除团队成员（双写）
    pub async fn remove_team_member_by_user(&self, user_id: &str) {
        // 先从 HashMap 移除
        {
            let mut members = self.team_members.write().await;
            members.retain(|_, m| m.user_id != user_id);
        }
        let _ = sqlx::query("DELETE FROM team_members WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// 判断用户是否为团队成员
    pub async fn is_team_member(&self, user_id: &str) -> bool {
        self.team_members
            .read()
            .await
            .values()
            .any(|m| m.user_id == user_id)
    }

    // ===== join_requests 双写辅助方法 =====

    /// 插入入队申请（双写）
    pub async fn insert_join_request(&self, request: &JoinRequest) {
        self.join_requests
            .write()
            .await
            .insert(request.id.clone(), request.clone());
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO join_requests \
             (id, user_id, user_name, user_email, reason, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&request.id)
        .bind(&request.user_id)
        .bind(&request.user_name)
        .bind(&request.user_email)
        .bind(&request.reason)
        .bind(&request.status)
        .bind(request.created_at.to_rfc3339())
        .execute(&self.db)
        .await;
    }

    /// 更新入队申请状态（双写）
    pub async fn update_join_request_status(&self, request_id: &str, status: &str) {
        if let Some(r) = self.join_requests.write().await.get_mut(request_id) {
            r.status = status.to_string();
        }
        let _ = sqlx::query("UPDATE join_requests SET status = ? WHERE id = ?")
            .bind(status)
            .bind(request_id)
            .execute(&self.db)
            .await;
    }

    /// 删除 session（双写）
    pub async fn remove_session(&self, token: &str) {
        self.sessions.write().await.remove(token);
        let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.db)
            .await;
    }

    /// Reload all in-memory state from the current data file on disk
    /// Used after restoring from backup
    pub async fn reload(&self) {
        if let Ok(json) = std::fs::read_to_string(&self.data_file) {
            if let Ok(data) = serde_json::from_str::<SavedData>(&json) {
                *self.users.write().await = data.users;
                *self.sessions.write().await = data.sessions;
                *self.refresh_tokens.write().await = data.refresh_tokens;
                *self.team_members.write().await = data.team_members;
                *self.problems.write().await = data.problems;
                *self.join_requests.write().await = data.join_requests;
                *self.contests.write().await = data.contests;
                *self.site_description.write().await = data.site_description;
                *self.notifications.write().await = data.notifications;
                *self.showcase_problem_ids.write().await = data.showcase_problem_ids;
                *self.showcase_contest_ids.write().await = data.showcase_contest_ids;
                // member_groups come from config.toml, not data.json
                let reloaded_config = load_config();
                *self.member_groups.write().await = load_member_groups(&reloaded_config);

                let mut p = data.posts;
                migrate_legacy_data(
                    &mut p,
                    &data.suggestions,
                    &data.announcements,
                    &data.discussions,
                );
                *self.posts.write().await = p;

                // discussion_tags and discussion_emojis stay from config.toml
                tracing::info!("State reloaded from {}", self.data_file);
            }
        }
    }

    /// 将 SQLite 中的所有数据导出到 JSON 文件。
    /// 用于 SQLite 备份恢复后将数据库状态同步回 JSON（从而同步到 HashMap）。
    pub async fn export_sqlite_to_json(&self) {
        use sqlx::Row;

        let mut data = crate::state::SavedData {
            users: HashMap::new(),
            sessions: HashMap::new(),
            refresh_tokens: HashMap::new(),
            team_members: HashMap::new(),
            problems: HashMap::new(),
            join_requests: HashMap::new(),
            contests: HashMap::new(),
            site_description: self.site_description.read().await.clone(),
            notifications: HashMap::new(),
            showcase_problem_ids: self.showcase_problem_ids.read().await.clone(),
            showcase_contest_ids: self.showcase_contest_ids.read().await.clone(),
            posts: HashMap::new(),
            suggestions: HashMap::new(),
            announcements: HashMap::new(),
            discussions: HashMap::new(),
            member_groups: self.member_groups.read().await.clone(),
        };

        // 从 SQLite 读取用户
        if let Ok(rows) = sqlx::query(
            "SELECT id, username, display_name, avatar_url, email, role, team_status, \
             created_at, bio, password_hash, effective_role, group_ids, user_permissions \
             FROM users",
        )
        .fetch_all(&self.db)
        .await
        {
            for row in rows {
                let id: String = row.get("id");
                data.users.insert(
                    id.clone(),
                    User {
                        id,
                        username: row.get("username"),
                        display_name: row.get("display_name"),
                        avatar_url: row.get("avatar_url"),
                        email: row.get("email"),
                        role: row.get("role"),
                        team_status: row.get("team_status"),
                        created_at: row
                            .get::<String, _>("created_at")
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                        bio: row.get("bio"),
                        password_hash: row.get("password_hash"),
                        effective_role: row.get("effective_role"),
                        group_ids: serde_json::from_str(&row.get::<String, _>("group_ids"))
                            .unwrap_or_default(),
                        user_permissions: serde_json::from_str(
                            &row.get::<String, _>("user_permissions"),
                        )
                        .unwrap_or_default(),
                    },
                );
            }
        }

        // 从 SQLite 读取团队成员
        if let Ok(rows) = sqlx::query("SELECT id, user_id, joined_at FROM team_members")
            .fetch_all(&self.db)
            .await
        {
            for row in rows {
                let id: String = row.get("id");
                data.team_members.insert(
                    id.clone(),
                    TeamMember {
                        id,
                        user_id: row.get("user_id"),
                        joined_at: row.get("joined_at"),
                    },
                );
            }
        }

        // 写入 JSON 文件（确保父目录存在）
        let json = serde_json::to_string_pretty(&data).unwrap_or_default();
        let data_path = std::path::Path::new(&self.data_file);
        if let Some(parent) = data_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let tmp_path = format!("{}.tmp", self.data_file);
        if std::fs::write(&tmp_path, &json).is_ok() {
            let _ = std::fs::rename(&tmp_path, &self.data_file);
            tracing::info!("SQLite 数据已导出到 JSON: {}", self.data_file);
        } else {
            tracing::warn!("无法写入 JSON 数据文件: {}", self.data_file);
        }
    }

    /// 启动后台自动备份任务。
    /// 每隔 `interval_secs` 秒创建一个 SQLite 备份，保留最多 `max_backups` 个。
    pub fn start_auto_backup(self: &Arc<Self>, interval_secs: u64, max_backups: usize) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            // 跳过第一次立即执行，等一个间隔
            interval.tick().await;

            loop {
                interval.tick().await;

                // 1. 保存当前状态到 JSON
                state.save().await;

                // 2. 同步到 SQLite
                let data_file = &state.data_file;
                if let Ok(json) = std::fs::read_to_string(data_file) {
                    if let Ok(saved) = serde_json::from_str::<crate::state::SavedData>(&json) {
                        let _ = crate::db::import_saved_data(&state.db, &saved).await;
                    }
                }

                // 3. 创建 SQLite 备份
                let db_path = std::path::Path::new(&state.data_file).with_extension("db");
                if db_path.exists() {
                    let dir = db_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join("backups");
                    let _ = std::fs::create_dir_all(&dir);

                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let backup_name = format!("mcguffin_auto_{}.db", timestamp);
                    let dest = dir.join(&backup_name);

                    match crate::db::create_consistent_backup(
                        &db_path.to_string_lossy(),
                        &dest.to_string_lossy(),
                    ) {
                        Ok(()) => tracing::debug!("Auto backup created: {:?}", dest),
                        Err(e) => tracing::warn!("Auto backup failed: {}", e),
                    }

                    // 4. 清理旧备份
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        let mut db_backups: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path().extension().map(|ext| ext == "db").unwrap_or(false)
                            })
                            .collect();
                        // 按修改时间排序，最旧的在前面
                        db_backups
                            .sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

                        // 删除超出限制的旧备份
                        while db_backups.len() > max_backups {
                            if let Some(oldest) = db_backups.first() {
                                let path = oldest.path();
                                if std::fs::remove_file(&path).is_ok() {
                                    tracing::info!("Pruned old auto-backup: {:?}", path);
                                }
                            }
                            db_backups.remove(0);
                        }
                    }
                }
            }
        });
    }

    fn default_team_members() -> HashMap<String, TeamMember> {
        HashMap::new()
    }

    fn default_problems() -> HashMap<String, Problem> {
        HashMap::new()
    }
}

impl Default for AppState {
    fn default() -> Self {
        // Default 仅供少数边缘场景使用（如某些推导 trait），
        // 正常情况下应使用 AppState::new().await
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(Self::new()))
    }
}

/// Convert config.toml discussion_tags format to HashMap<String, DiscussionTag>
fn load_discussion_tags(config: &AppConfig) -> std::collections::HashMap<String, DiscussionTag> {
    let mut map = std::collections::HashMap::new();
    for (name, fields) in &config.discussion_tags {
        let color = fields
            .get("color")
            .cloned()
            .unwrap_or_else(|| "#888888".to_string());
        let description = fields.get("description").cloned().unwrap_or_default();
        map.insert(
            name.clone(),
            DiscussionTag {
                id: name.clone(),
                name: name.clone(),
                color,
                description,
                admin_only: fields
                    .get("admin_only")
                    .and_then(|v| v.parse::<bool>().ok())
                    .unwrap_or(false),
            },
        );
    }
    map
}

/// Convert config.toml discussion_emojis format to HashMap<String, DiscussionEmoji>
fn load_discussion_emojis(
    config: &AppConfig,
) -> std::collections::HashMap<String, DiscussionEmoji> {
    let mut map = std::collections::HashMap::new();
    for (name, fields) in &config.discussion_emojis {
        let char = fields.get("char").cloned().unwrap_or_default();
        if char.is_empty() {
            continue;
        }
        map.insert(
            name.clone(),
            DiscussionEmoji {
                id: name.clone(),
                name: name.clone(),
                char,
            },
        );
    }
    map
}

/// Load member groups from config.toml [permissions.groups]
/// Format: uuid → { name, permissions: [...] }
fn load_member_groups(config: &AppConfig) -> HashMap<String, MemberGroup> {
    let mut map = HashMap::new();
    for (id, fields) in &config.permission_groups {
        let name = fields
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let perms: Vec<String> = fields
            .get("permissions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if !id.is_empty() && !name.is_empty() {
            map.insert(
                id.clone(),
                MemberGroup {
                    id: id.clone(),
                    name,
                    permissions: perms,
                },
            );
        }
    }
    map
}

/// Convert raw HashMap config to DifficultyConfig
fn load_difficulty_config(config: &AppConfig) -> crate::types::DifficultyConfig {
    if !config.difficulty.is_empty() {
        let mut levels = std::collections::HashMap::new();
        for (name, fields) in &config.difficulty {
            levels.insert(
                name.clone(),
                crate::types::DifficultyLevel {
                    label: fields.get("label").cloned().unwrap_or_else(|| name.clone()),
                    color: fields
                        .get("color")
                        .cloned()
                        .unwrap_or_else(|| "#888888".to_string()),
                },
            );
        }
        if !levels.is_empty() {
            return crate::types::DifficultyConfig { levels };
        }
    }
    crate::types::DifficultyConfig::default()
}

/// Load application config from /usr/share/mcguffin/config.toml,
/// with fallback to environment variables and then hardcoded defaults.
fn load_config() -> AppConfig {
    let config_path = resolve_config_path();
    // Try reading the system config file first
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = toml::from_str::<AppConfig>(&content) {
            tracing::info!("Loaded config from {}", config_path.display());
            return config;
        }
        tracing::warn!(
            "Failed to parse {}, falling back to env vars",
            config_path.display()
        );
    } else {
        tracing::warn!(
            "{} not found, falling back to env vars",
            config_path.display()
        );
    }

    // Fallback: read from environment variables with defaults
    let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| "https://lba-oi.team".to_string());
    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(v) => v,
        Err(_) => {
            // Try reading mcguffin.toml (legacy)
            std::fs::read_to_string("mcguffin.toml")
                .ok()
                .and_then(|c| toml::from_str::<serde_json::Value>(&c).ok())
                .and_then(|v| {
                    v.get("admin")?
                        .get("password")?
                        .as_str()
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "admin123".to_string())
        }
    };

    AppConfig {
        server: crate::types::ServerConfig {
            site_url,
            port: 3000,
            data_file: "mcguffin_data.json".to_string(),
        },
        admin: crate::types::AdminConfig {
            password: admin_password,
            display_name: std::env::var("ADMIN_DISPLAY_NAME")
                .unwrap_or_else(|_| "管理员".to_string()),
        },
        site: crate::types::SiteConfig {
            title: None,
            name: std::env::var("SITE_NAME").ok(),
            ..Default::default()
        },
        oauth: crate::types::OAuthConfig {
            cp_client_id: std::env::var("CPOAUTH_CLIENT_ID")
                .unwrap_or_else(|_| "c9d727f9-e18f-4c81-bfa4-4d4c8812840d".to_string()),
            cp_client_secret: std::env::var("CPOAUTH_CLIENT_SECRET")
                .unwrap_or_else(|_| "Q7DgkUHQIXMLWM-TzcdUEH_21zMK3JJwMlqX-2VrhuM".to_string()),
        },
        difficulty: std::collections::HashMap::new(),
        discussion_tags: std::collections::HashMap::new(),
        discussion_emojis: std::collections::HashMap::new(),
        permissions: std::collections::HashMap::new(),
        permission_groups: std::collections::HashMap::new(),
    }
}

// ============== Tests ==============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AdminConfig, OAuthConfig, ServerConfig, SiteConfig};
    use std::collections::HashMap;

    #[test]
    fn test_load_difficulty_config_empty() {
        let config = AppConfig {
            server: ServerConfig {
                site_url: "https://test.com".to_string(),
                port: 3000,
                data_file: "test.json".to_string(),
            },
            admin: AdminConfig {
                password: "pass".to_string(),
                display_name: "Admin".to_string(),
            },
            site: SiteConfig {
                name: None,
                title: None,
                ..Default::default()
            },
            oauth: OAuthConfig {
                cp_client_id: "id".to_string(),
                cp_client_secret: "secret".to_string(),
            },
            difficulty: HashMap::new(),
            discussion_tags: HashMap::new(),
            discussion_emojis: HashMap::new(),
            permissions: HashMap::new(),
            permission_groups: HashMap::new(),
        };
        let dc = load_difficulty_config(&config);
        assert_eq!(dc.levels.len(), 3); // falls back to Default
        assert!(dc.levels.contains_key("Easy"));
    }

    #[test]
    fn test_load_difficulty_config_custom() {
        let mut diff = HashMap::new();
        let mut easy_fields = HashMap::new();
        easy_fields.insert("label".to_string(), "简单".to_string());
        easy_fields.insert("color".to_string(), "#22c55e".to_string());
        diff.insert("Easy".to_string(), easy_fields);

        let config = AppConfig {
            server: ServerConfig {
                site_url: "https://test.com".to_string(),
                port: 3000,
                data_file: "test.json".to_string(),
            },
            admin: AdminConfig {
                password: "pass".to_string(),
                display_name: "Admin".to_string(),
            },
            site: SiteConfig {
                name: None,
                title: None,
                ..Default::default()
            },
            oauth: OAuthConfig {
                cp_client_id: "id".to_string(),
                cp_client_secret: "secret".to_string(),
            },
            difficulty: diff,
            discussion_tags: HashMap::new(),
            discussion_emojis: HashMap::new(),
            permissions: HashMap::new(),
            permission_groups: HashMap::new(),
        };
        let dc = load_difficulty_config(&config);
        assert_eq!(dc.levels.len(), 1);
        assert_eq!(dc.levels.get("Easy").unwrap().label, "简单");
    }

    #[test]
    fn test_load_difficulty_config_missing_label() {
        let mut diff = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("color".to_string(), "#ff0000".to_string());
        // label is missing — should fall back to the key name
        diff.insert("CustomDiff".to_string(), fields);

        let config = AppConfig {
            server: ServerConfig {
                site_url: "https://test.com".to_string(),
                port: 3000,
                data_file: "test.json".to_string(),
            },
            admin: AdminConfig {
                password: "pass".to_string(),
                display_name: "Admin".to_string(),
            },
            site: SiteConfig {
                name: None,
                title: None,
                ..Default::default()
            },
            oauth: OAuthConfig {
                cp_client_id: "id".to_string(),
                cp_client_secret: "secret".to_string(),
            },
            difficulty: diff,
            discussion_tags: HashMap::new(),
            discussion_emojis: HashMap::new(),
            permissions: HashMap::new(),
            permission_groups: HashMap::new(),
        };
        let dc = load_difficulty_config(&config);
        assert_eq!(dc.levels.len(), 1);
        assert_eq!(dc.levels.get("CustomDiff").unwrap().label, "CustomDiff");
    }
}
