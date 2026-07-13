use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub role: String,
    pub team_status: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub password_hash: Option<String>,
    /// Computed effective role for permission lookup — depends on role + team_status.
    /// Set at response time; deserialized from storage as empty / fallback.
    #[serde(default)]
    pub effective_role: String,
    /// IDs of member groups this user belongs to.
    #[serde(default)]
    pub group_ids: Vec<String>,
    /// Individual permissions granted directly to this user (not via role or group).
    #[serde(default)]
    pub user_permissions: Vec<String>,
}

impl User {
    /// Compute the effective role used for permission checks.
    ///
    /// Maps (role, team_status) → effective_role:
    ///   - superadmin → superadmin
    ///   - admin → admin
    ///   - team_status == "pending" → guest
    ///   - role == "guest" && team_status != "joined" → guest
    ///   - role == "member" && team_status == "joined" → member
    ///   - fallback → role as-is
    pub fn compute_effective_role(&self) -> &str {
        match self.team_status.as_str() {
            "pending" => "guest",
            _ => match self.role.as_str() {
                "superadmin" => "superadmin",
                "admin" => "admin",
                "guest" if self.team_status != "joined" => "guest",
                "member" if self.team_status == "joined" => "member",
                r => r,
            },
        }
    }
}

/// A single session entry combining user_id and last_active timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub user_id: String,
    pub last_active: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct UpdateProfilePayload {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct ApplyPayload {
    pub reason: String,
}

#[derive(Serialize)]
pub struct ApplyResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct ReviewResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct ChangeRolePayload {
    pub role: String,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    #[serde(default)]
    pub identifier: Option<String>,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub user_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_payload_deserialize() {
        let json = r#"{"password": "***"}"#;
        let payload: LoginPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.password, "***");
        assert!(payload.identifier.is_none());
    }
}
