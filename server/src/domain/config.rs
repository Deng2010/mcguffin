use serde::Deserialize;
use std::collections::HashMap;

/// Top-level app config, read from /usr/share/mcguffin/config.toml
#[derive(Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub admin: AdminConfig,
    #[serde(default)]
    pub site: SiteConfig,
    pub oauth: OAuthConfig,
    #[serde(default)]
    pub difficulty: HashMap<String, HashMap<String, String>>,
    /// Discussion tags: name → { color, description }
    #[serde(default)]
    pub discussion_tags: HashMap<String, HashMap<String, String>>,
    /// Discussion emojis: name → { char }
    #[serde(default)]
    pub discussion_emojis: HashMap<String, HashMap<String, String>>,
    /// Role→permissions mapping. Overrides the hardcoded defaults.
    /// superadmin = ["*"] means all permissions.
    /// Example: [permissions.roles.admin] = ["view_team", "manage_team", ...]
    #[serde(default, deserialize_with = "deserialize_role_permissions")]
    pub permissions: HashMap<String, Vec<String>>,
    /// Group permissions: uuid → { name, permissions }
    /// Example: [permissions.groups."uuid"] = { name = "出题组", permissions = ["submit_problem"] }
    #[serde(default, deserialize_with = "deserialize_permission_groups")]
    pub permission_groups: HashMap<String, HashMap<String, serde_json::Value>>,
}

/// Custom deserializer for role permissions that gracefully handles nested [permissions.roles] format
fn deserialize_role_permissions<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <HashMap<String, Vec<String>>>::deserialize(deserializer).or_else(|_| Ok(HashMap::new()))
}

/// Custom deserializer for permission_groups that gracefully handles TOML inline tables
fn deserialize_permission_groups<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, HashMap<String, serde_json::Value>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <HashMap<String, HashMap<String, serde_json::Value>>>::deserialize(deserializer)
        .or_else(|_| Ok(HashMap::new()))
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub site_url: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_port() -> u16 {
    3000
}

#[derive(Deserialize)]
pub struct AdminConfig {
    pub password: String,
    #[serde(default = "default_admin_display_name")]
    pub display_name: String,
}

fn default_admin_display_name() -> String {
    "管理员".to_string()
}

#[derive(Deserialize, Default)]
pub struct SiteConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub difficulty_order: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct OAuthConfig {
    pub cp_client_id: String,
    pub cp_client_secret: String,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DifficultyLevel {
    pub label: String,
    pub color: String,
}

/// Config.toml [difficulty] section: structured as table with level names as keys
/// Example:
///   [difficulty.Easy]
///   label = "简单"
///   color = "#22c55e"
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DifficultyConfig {
    #[serde(flatten)]
    pub levels: HashMap<String, DifficultyLevel>,
}

impl Default for DifficultyConfig {
    fn default() -> Self {
        let mut levels = HashMap::new();
        levels.insert(
            "Easy".to_string(),
            DifficultyLevel {
                label: "简单".to_string(),
                color: "#22c55e".to_string(),
            },
        );
        levels.insert(
            "Medium".to_string(),
            DifficultyLevel {
                label: "中等".to_string(),
                color: "#f59e0b".to_string(),
            },
        );
        levels.insert(
            "Hard".to_string(),
            DifficultyLevel {
                label: "困难".to_string(),
                color: "#ef4444".to_string(),
            },
        );
        Self { levels }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_config_default() {
        let config = DifficultyConfig::default();
        assert_eq!(config.levels.len(), 3);
        assert!(config.levels.contains_key("Easy"));
        assert!(config.levels.contains_key("Medium"));
        assert!(config.levels.contains_key("Hard"));
        assert_eq!(config.levels.get("Easy").unwrap().label, "简单");
        assert_eq!(config.levels.get("Medium").unwrap().color, "#f59e0b");
    }

    #[test]
    fn test_server_config_defaults() {
        let toml_str = r#"
[server]
site_url = "https://example.com"
[admin]
password = "pass"
[oauth]
cp_client_id = "id"
cp_client_secret = "secret"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.site_url, "https://example.com");
        assert_eq!(config.server.port, 3000); // default
        assert_eq!(config.admin.password, "pass");
        assert_eq!(config.site.name, None); // default
    }

    #[test]
    fn test_app_config_with_difficulty() {
        let toml_str = r##"
[server]
site_url = "https://example.com"
[admin]
password = "pass"
[oauth]
cp_client_id = "id"
cp_client_secret = "secret"

[difficulty.Easy]
label = "简单"
color = "#22c55e"
[difficulty.Hard]
label = "困难"
color = "#ef4444"
"##;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.difficulty.len(), 2);
        assert_eq!(
            config.difficulty.get("Easy").unwrap().get("label").unwrap(),
            "简单"
        );
    }
}
