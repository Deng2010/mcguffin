use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A named group of members that can be assigned permissions collectively.
/// A user can belong to multiple groups; group permissions are OR'd with
/// the user's role-based and individual permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// An entry in the permission audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub user_name: String,
    pub action: String,
    pub resource: String,
    pub result: String, // "allow" | "deny"
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub user_id: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_email: String,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
