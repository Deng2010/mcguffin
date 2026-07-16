use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{Mutex, RwLock};

use crate::types::{
    Contest, DiscussionEmoji, DiscussionTag, JoinRequest, MemberGroup, Notification, Post, Problem,
    SessionEntry, TeamMember, User,
};

pub const ADMIN_USER_ID: &str = "admin";

/// Resolve the config file path with platform awareness.
/// Priority: CWD 的 mcguffin.toml/config.toml > 平台默认路径。
pub fn resolve_config_path() -> PathBuf {
    // 0. MCGUFFIN_DATA_DIR 环境变量（Docker 场景）
    if let Ok(data_dir) = std::env::var("MCGUFFIN_DATA_DIR") {
        return PathBuf::from(&data_dir).join("config.toml");
    }
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

#[derive(Clone)]
pub struct AppState {
    pub users: Arc<Mutex<HashMap<String, User>>>,
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
    pub admin_password: Arc<RwLock<String>>,
    pub site_name: String,
    pub site_title: String,
    pub site_version: String,
    pub site_description: Arc<RwLock<String>>,
    /// Public-facing site URL (e.g. https://lba-oi.team)
    pub site_url: String,
    /// Path to the SQLite database file
    pub db_path: String,
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
    /// 自定义备份目录（None 时使用默认路径）
    pub backup_directory: Arc<RwLock<Option<String>>>,
    /// 复用 HTTP 客户端（带超时，连接池共享）
    pub http_client: reqwest::Client,
}


