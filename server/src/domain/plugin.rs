use serde::{Deserialize, Serialize};

/// Plugin manifest — declared by the plugin in its plugin.json / definePlugin() call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Permissions the plugin requests. Each string like "read:team", "storage", etc.
    #[serde(default, rename = "permissions_needed")]
    pub permissions: Vec<String>,
    /// Whether the plugin is enabled. Disabled plugins cannot use any API endpoints.
    /// Newly registered plugins default to enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Plugin registration payload sent by the frontend on load.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginRegistration {
    pub id: String,
    pub manifest: PluginManifest,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// Response for listing plugins (admin).
#[derive(Debug, Clone, Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginManifest>,
}

/// Payload for enabling/disabling a plugin.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginTogglePayload {
    pub enabled: bool,
}

// ── Plugin permission constants ──

/// Well-known plugin permission scopes.
/// Permissions are resource × action, e.g. "read:team", "write:team_roles".
pub mod plugin_perms {
    /// Access own KV store and files (always scoped to pluginId).
    pub const STORAGE: &str = "storage";
    /// Send notifications to users.
    pub const NOTIFY: &str = "notify";
    /// List team members.
    pub const READ_TEAM: &str = "read:team";
    /// Approve/reject join requests, remove members.
    pub const WRITE_TEAM: &str = "write:team";
    /// Change member roles (most sensitive team operation).
    pub const WRITE_TEAM_ROLES: &str = "write:team_roles";
    /// Read problem list.
    pub const READ_PROBLEMS: &str = "read:problems";
    /// Read contest list.
    pub const READ_CONTESTS: &str = "read:contests";
    /// Read community posts.
    pub const READ_POSTS: &str = "read:posts";
    /// Read user profiles (limited fields: id, username, display_name, avatar).
    pub const READ_USERS: &str = "read:users";
    /// Read user email (extends read:users).
    pub const READ_USERS_EMAIL: &str = "read:users:email";

    /// All known permission strings (for validation).
    pub const ALL: &[&str] = &[
        STORAGE,
        NOTIFY,
        READ_TEAM,
        WRITE_TEAM,
        WRITE_TEAM_ROLES,
        READ_PROBLEMS,
        READ_CONTESTS,
        READ_POSTS,
        READ_USERS,
        READ_USERS_EMAIL,
    ];

    /// Permissions that imply read:team.
    pub fn implies_read_team(perms: &[String]) -> bool {
        perms
            .iter()
            .any(|p| p == READ_TEAM || p == WRITE_TEAM || p == WRITE_TEAM_ROLES)
    }

    /// Permissions that imply write:team.
    pub fn has_write_team(perms: &[String]) -> bool {
        perms
            .iter()
            .any(|p| p == WRITE_TEAM || p == WRITE_TEAM_ROLES)
    }

    /// Permissions that allow changing roles.
    pub fn has_write_team_roles(perms: &[String]) -> bool {
        perms.iter().any(|p| p == WRITE_TEAM_ROLES)
    }
}

#[cfg(test)]
mod tests {
    use super::plugin_perms::{self, WRITE_TEAM, WRITE_TEAM_ROLES};

    fn perms(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn implies_read_team_covers_direct_and_implied() {
        assert!(plugin_perms::implies_read_team(&perms(&["read:team"])));
        assert!(plugin_perms::implies_read_team(&perms(&["write:team"])));
        assert!(
            plugin_perms::implies_read_team(&perms(&["write:team_roles"])),
            "write:team_roles 应蕴含 read:team"
        );
        assert!(!plugin_perms::implies_read_team(&perms(&["storage"])));
        assert!(!plugin_perms::implies_read_team(&[]));
    }

    #[test]
    fn has_write_team_covers_direct_and_roles() {
        assert!(plugin_perms::has_write_team(&perms(&[WRITE_TEAM])));
        assert!(plugin_perms::has_write_team(&perms(&[WRITE_TEAM_ROLES])));
        assert!(!plugin_perms::has_write_team(&perms(&["read:team"])));
        assert!(!plugin_perms::has_write_team(&[]));
    }

    #[test]
    fn has_write_team_roles_only_matches_roles() {
        assert!(plugin_perms::has_write_team_roles(&perms(&[WRITE_TEAM_ROLES])));
        assert!(!plugin_perms::has_write_team_roles(&perms(&[WRITE_TEAM])));
        assert!(!plugin_perms::has_write_team_roles(&perms(&["read:team"])));
    }
}

// ── Plugin KV data types ──

#[derive(Debug, Clone, Deserialize)]
pub struct SetDataPayload {
    pub namespace: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddDataPayload {
    pub namespace: String,
    pub key: String,
    pub delta: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetMemberPayload {
    pub namespace: String,
    pub key: String,
    pub member: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotifyPayload {
    pub user_id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub link: Option<String>,
}
