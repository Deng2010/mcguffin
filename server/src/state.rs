use crate::types::{Announcement, Contest, JoinRequest, Notification, Problem, Suggestion, TeamMember, User, AppConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const ADMIN_USER_ID: &str = "admin";

/// System-wide config file path
const CONFIG_FILE: &str = "/usr/share/mcguffin/config.toml";

// ============== Persistence Format ==============

#[derive(Serialize, Deserialize)]
struct SavedData {
    users: HashMap<String, User>,
    sessions: HashMap<String, String>,
    refresh_tokens: HashMap<String, String>,
    team_members: HashMap<String, TeamMember>,
    problems: HashMap<String, Problem>,
    join_requests: HashMap<String, JoinRequest>,
    #[serde(default)]
    contests: HashMap<String, Contest>,
    #[serde(default)]
    site_description: String,
    #[serde(default)]
    suggestions: HashMap<String, Suggestion>,
    #[serde(default)]
    announcements: HashMap<String, Announcement>,
    #[serde(default)]
    notifications: HashMap<String, Notification>,
}

// ============== Application State ==============

#[derive(Clone)]
pub struct AppState {
    pub users: Arc<RwLock<HashMap<String, User>>>,
    pub sessions: Arc<RwLock<HashMap<String, String>>>,
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
    /// Suggestions (tickets/feedback)
    pub suggestions: Arc<RwLock<HashMap<String, Suggestion>>>,
    /// Announcements
    pub announcements: Arc<RwLock<HashMap<String, Announcement>>>,
    /// Notifications
    pub notifications: Arc<RwLock<HashMap<String, Notification>>>,
}

impl AppState {
    pub fn new() -> Self {
        // Load config from /usr/share/mcguffin/config.toml
        let config = load_config();
        let difficulty_config = load_difficulty_config(&config);

        let site_version = env!("CARGO_PKG_VERSION").to_string();

        // Load saved JSON data
        let data_file = &config.server.data_file;
        let saved = std::fs::read_to_string(data_file)
            .ok()
            .and_then(|s| serde_json::from_str::<SavedData>(&s).ok());

        let (mut users, sessions, refresh_tokens, mut team_members, problems, join_requests, contests, site_description, suggestions, announcements, notifications) =
            if let Some(data) = saved {
                tracing::info!("Loaded state from {}", data_file);
                (data.users, data.sessions, data.refresh_tokens, data.team_members, data.problems, data.join_requests, data.contests, data.site_description, data.suggestions, data.announcements, data.notifications)
            } else {
                tracing::info!("No saved state, using default seed data");
                (HashMap::new(), HashMap::new(), HashMap::new(), Self::default_team_members(), Self::default_problems(), HashMap::new(), HashMap::new(), String::new(), HashMap::new(), HashMap::new(), HashMap::new())
            };

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
        });
        // Force update role to superadmin (in case loaded from old data)
        if let Some(u) = users.get_mut(ADMIN_USER_ID) {
            u.role = "superadmin".to_string();
            u.display_name = admin_display_name.clone();
        }

        // Always ensure superadmin is a team member AND has correct role
        team_members.entry(ADMIN_USER_ID.to_string()).or_insert(TeamMember {
            id: ADMIN_USER_ID.to_string(),
            user_id: ADMIN_USER_ID.to_string(),
            name: admin_display_name.clone(),
            avatar: admin_display_name.chars().next().unwrap_or('A').to_string(),
            avatar_url: None,
            role: "superadmin".to_string(),
            joined_at: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
                .format("%Y-%m-%d")
                .to_string(),
        });
        // Force update team member role to superadmin
        if let Some(m) = team_members.get_mut(ADMIN_USER_ID) {
            m.role = "superadmin".to_string();
        }

        let redirect_uri = format!("{}/api/oauth/callback", config.server.site_url);

        Self {
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
            site_name: config.site.name.clone().unwrap_or_else(|| "McGuffin".to_string()),
            site_title: config.site.title.unwrap_or_else(|| config.site.name.unwrap_or_else(|| "McGuffin".to_string())),
            site_version,
            site_description: Arc::new(RwLock::new(site_description)),
            site_url: config.server.site_url,
            data_file: config.server.data_file,
            difficulty: Arc::new(RwLock::new(difficulty_config)),
            suggestions: Arc::new(RwLock::new(suggestions)),
            announcements: Arc::new(RwLock::new(announcements)),
            notifications: Arc::new(RwLock::new(notifications)),
        }
    }

    /// Persist all in-memory state to JSON file
    pub async fn save(&self) {
        let data = SavedData {
            users: self.users.read().await.clone(),
            sessions: self.sessions.read().await.clone(),
            refresh_tokens: self.refresh_tokens.read().await.clone(),
            team_members: self.team_members.read().await.clone(),
            problems: self.problems.read().await.clone(),
            join_requests: self.join_requests.read().await.clone(),
            contests: self.contests.read().await.clone(),
            site_description: self.site_description.read().await.clone(),
            suggestions: self.suggestions.read().await.clone(),
            announcements: self.announcements.read().await.clone(),
            notifications: self.notifications.read().await.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let tmp_path = format!("{}.tmp", self.data_file);
            // Write to temp file first, then atomically rename
            // This prevents data corruption if the process crashes mid-write
            if std::fs::write(&tmp_path, &json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &self.data_file);
            }
        }
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
                *self.suggestions.write().await = data.suggestions;
                *self.announcements.write().await = data.announcements;
                *self.notifications.write().await = data.notifications;
                tracing::info!("State reloaded from {}", self.data_file);
            }
        }
    }

    fn default_team_members() -> HashMap<String, TeamMember> {
        HashMap::new()
    }

    fn default_problems() -> HashMap<String, Problem> {
        HashMap::new()
    }
}

/// Convert raw HashMap config to DifficultyConfig
fn load_difficulty_config(config: &AppConfig) -> crate::types::DifficultyConfig {
    if !config.difficulty.is_empty() {
        let mut levels = std::collections::HashMap::new();
        for (name, fields) in &config.difficulty {
            levels.insert(name.clone(), crate::types::DifficultyLevel {
                label: fields.get("label").cloned().unwrap_or_else(|| name.clone()),
                color: fields.get("color").cloned().unwrap_or_else(|| "#888888".to_string()),
            });
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
    // Try reading the system config file first
    if let Ok(content) = std::fs::read_to_string(CONFIG_FILE) {
        if let Ok(config) = toml::from_str::<AppConfig>(&content) {
            tracing::info!("Loaded config from {}", CONFIG_FILE);
            return config;
        }
        tracing::warn!("Failed to parse {}, falling back to env vars", CONFIG_FILE);
    } else {
        tracing::warn!("{} not found, falling back to env vars", CONFIG_FILE);
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
                .and_then(|v| v.get("admin")?.get("password")?.as_str().map(|s| s.to_string()))
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
            display_name: std::env::var("ADMIN_DISPLAY_NAME").unwrap_or_else(|_| "管理员".to_string()),
        },
        site: crate::types::SiteConfig { title: None,
            name: std::env::var("SITE_NAME").ok(),
        },
        oauth: crate::types::OAuthConfig {
            cp_client_id: std::env::var("CPOAUTH_CLIENT_ID")
                .unwrap_or_else(|_| "c9d727f9-e18f-4c81-bfa4-4d4c8812840d".to_string()),
            cp_client_secret: std::env::var("CPOAUTH_CLIENT_SECRET")
                .unwrap_or_else(|_| "Q7DgkUHQIXMLWM-TzcdUEH_21zMK3JJwMlqX-2VrhuM".to_string()),
        },
        difficulty: std::collections::HashMap::new(),
    }
}

// ============== Tests ==============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ServerConfig, AdminConfig, SiteConfig, OAuthConfig};
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
            site: SiteConfig { name: None, title: None },
            oauth: OAuthConfig {
                cp_client_id: "id".to_string(),
                cp_client_secret: "secret".to_string(),
            },
            difficulty: HashMap::new(),
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
            site: SiteConfig { name: None, title: None },
            oauth: OAuthConfig {
                cp_client_id: "id".to_string(),
                cp_client_secret: "secret".to_string(),
            },
            difficulty: diff,
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
            site: SiteConfig { name: None, title: None },
            oauth: OAuthConfig {
                cp_client_id: "id".to_string(),
                cp_client_secret: "secret".to_string(),
            },
            difficulty: diff,
        };
        let dc = load_difficulty_config(&config);
        assert_eq!(dc.levels.get("CustomDiff").unwrap().label, "CustomDiff");
        assert_eq!(dc.levels.get("CustomDiff").unwrap().color, "#ff0000");
    }
}
