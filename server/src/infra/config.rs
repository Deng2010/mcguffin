use std::collections::HashMap;
use std::fs;
use tracing;

use crate::domain::config::PluginsConfig;
use crate::state::resolve_config_path;
use crate::types::*;

/// Convert config.toml discussion_tags format to HashMap<String, DiscussionTag>
pub(crate) fn load_discussion_tags(
    config: &AppConfig,
) -> std::collections::HashMap<String, DiscussionTag> {
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
pub(crate) fn load_discussion_emojis(
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
pub(crate) fn load_member_groups(config: &AppConfig) -> HashMap<String, MemberGroup> {
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
pub(crate) fn load_difficulty_config(config: &AppConfig) -> DifficultyConfig {
    if !config.difficulty.is_empty() {
        let mut levels = std::collections::HashMap::new();
        for (name, fields) in &config.difficulty {
            levels.insert(
                name.clone(),
                DifficultyLevel {
                    label: fields.get("label").cloned().unwrap_or_else(|| name.clone()),
                    color: fields
                        .get("color")
                        .cloned()
                        .unwrap_or_else(|| "#888888".to_string()),
                },
            );
        }
        if !levels.is_empty() {
            return DifficultyConfig { levels };
        }
    }
    DifficultyConfig::default()
}

/// Load application config from /usr/share/mcguffin/config.toml,
/// with fallback to environment variables and then hardcoded defaults.
pub(crate) fn load_config() -> AppConfig {
    let config_path = resolve_config_path();

    // 1. 尝试从配置文件加载
    let mut config: Option<AppConfig> =
        fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| match toml::from_str::<AppConfig>(&content) {
                Ok(c) => {
                    tracing::info!("Loaded config from {}", config_path.display());
                    Some(c)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse {}: {}, falling back to env vars",
                        config_path.display(),
                        e
                    );
                    None
                }
            });

    if config.is_none() {
        tracing::warn!(
            "{} not found, falling back to env vars",
            config_path.display()
        );
    }

    // 2. 环境变量覆盖（Docker 场景：环境变量优先级高于配置文件）
    if let Some(ref mut cfg) = config {
        if let Ok(v) = std::env::var("SITE_URL") {
            cfg.server.site_url = v;
        }
        if let Ok(v) = std::env::var("ADMIN_PASSWORD") {
            cfg.admin.password = v;
        }
        if let Ok(v) = std::env::var("ADMIN_DISPLAY_NAME") {
            cfg.admin.display_name = v;
        }
        if let Ok(v) = std::env::var("SITE_NAME") {
            cfg.site.name = Some(v);
        }
        if let Ok(v) = std::env::var("CPOAUTH_CLIENT_ID") {
            cfg.oauth.cp_client_id = v;
        }
        if let Ok(v) = std::env::var("CPOAUTH_CLIENT_SECRET") {
            cfg.oauth.cp_client_secret = v;
        }
    }

    // 3. 最终 fallback：全环境变量模式
    config.unwrap_or_else(|| AppConfig {
        server: ServerConfig {
            site_url: std::env::var("SITE_URL")
                .unwrap_or_else(|_| "https://lba-oi.team".to_string()),
            port: 3000,
        },
        admin: AdminConfig {
            password: std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string()),
            display_name: std::env::var("ADMIN_DISPLAY_NAME")
                .unwrap_or_else(|_| "管理员".to_string()),
        },
        site: SiteConfig {
            title: None,
            name: std::env::var("SITE_NAME").ok(),
            ..Default::default()
        },
        oauth: OAuthConfig {
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
        plugins: PluginsConfig::default(),
    })
}

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
            plugins: PluginsConfig::default(),
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
            plugins: PluginsConfig::default(),
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
            plugins: PluginsConfig::default(),
        };
        let dc = load_difficulty_config(&config);
        assert_eq!(dc.levels.len(), 1);
        assert_eq!(dc.levels.get("CustomDiff").unwrap().label, "CustomDiff");
    }
}
