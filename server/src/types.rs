use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============== Permission System ==============

/// All permission identifiers used in the system.
/// These are the canonical names — both backend and frontend use them.
pub mod perms {
    // ── Public / Guest ──
    pub const VIEW_SHOWCASE: &str = "view_showcase";
    pub const APPLY_JOIN: &str = "apply_join";

    // ── Team ──
    pub const VIEW_TEAM: &str = "view_team";
    /// Approve/reject join requests
    pub const MANAGE_TEAM: &str = "manage_team";
    /// Kick members, change roles
    pub const MANAGE_MEMBERS: &str = "manage_members";

    // ── Problems ──
    pub const SUBMIT_PROBLEM: &str = "submit_problem";
    pub const VIEW_PROBLEMS: &str = "view_problems";
    pub const APPROVE_PROBLEM: &str = "approve_problem";

    // ── Contests ──
    pub const MANAGE_CONTESTS: &str = "manage_contests";
    /// View all contests including drafts
    pub const VIEW_ALL_CONTESTS: &str = "view_all_contests";
    /// View only public contests
    pub const VIEW_PUBLIC_CONTESTS: &str = "view_public_contests";

    // ── Site ──
    pub const MANAGE_SITE: &str = "manage_site";

    // ── Discussions / Community ──
    pub const VIEW_DISCUSSIONS: &str = "view_discussions";
    pub const MANAGE_DISCUSSIONS: &str = "manage_discussions";
    pub const MANAGE_TAGS: &str = "manage_tags";

    // ── System ──
    pub const EDIT_SHOWCASE: &str = "edit_showcase";
    pub const MANAGE_NOTIFICATIONS: &str = "manage_notifications";
    pub const MANAGE_BACKUPS: &str = "manage_backups";
    pub const VIEW_STATS: &str = "view_stats";
    /// Manage unified posts (discussions, suggestions, announcements).
    /// Replaces the deprecated manage_discussions.
    pub const MANAGE_POSTS: &str = "manage_posts";

    /// All defined permissions (used for config validation)
    pub const ALL: &[&str] = &[
        VIEW_SHOWCASE,
        APPLY_JOIN,
        VIEW_TEAM,
        MANAGE_TEAM,
        MANAGE_MEMBERS,
        SUBMIT_PROBLEM,
        VIEW_PROBLEMS,
        APPROVE_PROBLEM,
        MANAGE_CONTESTS,
        VIEW_ALL_CONTESTS,
        VIEW_PUBLIC_CONTESTS,
        MANAGE_SITE,
        EDIT_SHOWCASE,
        VIEW_DISCUSSIONS,
        MANAGE_DISCUSSIONS,
        MANAGE_TAGS,
        MANAGE_NOTIFICATIONS,
        MANAGE_BACKUPS,
        VIEW_STATS,
        MANAGE_POSTS,
    ];
}

/// The special wildcard permission meaning "all permissions" (superadmin only).
pub const PERM_WILDCARD: &str = "*";

// ============== Member Groups ==============

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

// ============== Audit Log ==============

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

/// Return the default role→permissions mapping.
pub fn default_role_permissions() -> HashMap<String, Vec<String>> {
    let mut m = HashMap::new();
    // superadmin gets wildcard + all explicit permissions (frontend doesn't understand wildcards)
    let all_perms: Vec<String> = perms::ALL.iter().map(|p| p.to_string()).collect();
    let mut superadmin_perms = vec![PERM_WILDCARD.to_string()];
    superadmin_perms.extend(all_perms);
    m.insert("superadmin".to_string(), superadmin_perms);
    m.insert(
        "admin".to_string(),
        vec![
            perms::VIEW_SHOWCASE.to_string(),
            perms::VIEW_TEAM.to_string(),
            perms::MANAGE_TEAM.to_string(),
            perms::MANAGE_MEMBERS.to_string(),
            perms::SUBMIT_PROBLEM.to_string(),
            perms::VIEW_PROBLEMS.to_string(),
            perms::APPROVE_PROBLEM.to_string(),
            perms::MANAGE_CONTESTS.to_string(),
            perms::VIEW_ALL_CONTESTS.to_string(),
            perms::VIEW_PUBLIC_CONTESTS.to_string(),
            perms::MANAGE_SITE.to_string(),
            perms::EDIT_SHOWCASE.to_string(),
            perms::VIEW_DISCUSSIONS.to_string(),
            perms::MANAGE_POSTS.to_string(),
            perms::MANAGE_TAGS.to_string(),
            perms::MANAGE_NOTIFICATIONS.to_string(),
            perms::VIEW_STATS.to_string(),
        ],
    );
    m.insert(
        "member".to_string(),
        vec![
            perms::VIEW_SHOWCASE.to_string(),
            perms::VIEW_TEAM.to_string(),
            perms::SUBMIT_PROBLEM.to_string(),
            perms::VIEW_PROBLEMS.to_string(),
            perms::VIEW_ALL_CONTESTS.to_string(),
            perms::VIEW_PUBLIC_CONTESTS.to_string(),
            perms::VIEW_DISCUSSIONS.to_string(),
        ],
    );
    m.insert(
        "guest".to_string(),
        vec![
            perms::VIEW_SHOWCASE.to_string(),
            perms::APPLY_JOIN.to_string(),
            perms::VIEW_PUBLIC_CONTESTS.to_string(),
            perms::VIEW_DISCUSSIONS.to_string(),
        ],
    );
    m
}

// ============== Session ==============

/// A single session entry combining user_id and last_active timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub user_id: String,
    pub last_active: DateTime<Utc>,
}

// ============== User ==============

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

// ============== Team ==============

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

// ============== Contest ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contest {
    pub id: String,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub description: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub problem_order: Vec<String>,
    #[serde(default)]
    pub visible_to: Vec<String>,
    #[serde(default)]
    pub editable_by: Vec<String>,
}

#[derive(Deserialize)]
pub struct CreateContestPayload {
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub description: String,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateContestPayload {
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub description: String,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Deserialize)]
pub struct SetProblemContestPayload {
    /// contest_id to assign, or None to clear
    #[serde(default)]
    pub contest_id: Option<String>,
}

#[derive(Serialize)]
pub struct ContestListItem {
    pub id: String,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub description: String,
    pub created_by: String,
    pub created_at: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default)]
    pub problem_order: Vec<String>,
    #[serde(default)]
    pub visible_to: Vec<String>,
    #[serde(default)]
    pub editable_by: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetContestStatusPayload {
    pub status: String,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Deserialize)]
pub struct ContestProblemOrderPayload {
    pub problem_ids: Vec<String>,
}

// ============== Problem ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub contest: String,
    #[serde(default)]
    pub contest_id: Option<String>,
    pub difficulty: String,
    pub content: String,
    pub solution: Option<String>, // author's solution (markdown)
    pub status: String,           // pending | approved | published | rejected
    pub created_at: DateTime<Utc>,
    pub public_at: Option<DateTime<Utc>>,
    pub claimed_by: Option<String>, // user_id of verifier who claimed this
    pub verifier_solution: Option<String>, // verifier's solution (markdown)
    pub visible_to: Vec<String>,    // user_ids who can see pending problem content
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    /// Internal remark for reviewers (hidden after approval)
    pub remark: Option<String>,
    /// User IDs and/or group IDs (prefixed "group:xxx") who can edit this problem.
    /// If empty, falls back to default permission check.
    #[serde(default)]
    pub editable_by: Vec<String>,
}

#[derive(Deserialize)]
pub struct SubmitProblemPayload {
    pub title: String,
    #[serde(default)]
    pub contest: String,
    #[serde(default)]
    pub contest_id: Option<String>,
    pub difficulty: String,
    pub content: String,
    pub solution: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Deserialize)]
pub struct EditProblemPayload {
    /// difficulty to set, if changed
    #[serde(default)]
    pub difficulty: Option<String>,
    /// content to set, if changed
    #[serde(default)]
    pub content: Option<String>,
    /// solution to set (None = no change, Some("") = clear, Some("...") = update)
    #[serde(default)]
    pub solution: Option<String>,
    /// contest_id to set (None = no change, Some("") = clear, Some("id") = assign)
    #[serde(default)]
    pub contest_id: Option<Option<String>>,
    #[serde(default)]
    pub link: Option<Option<String>>,
    /// author_name to set (admin only) — the display name for the author
    #[serde(default)]
    pub author_name: Option<String>,
    /// author_id to set (admin only, None = no change, Some("unknown") = set unknown, Some("id") = assign to member)
    #[serde(default)]
    pub author_id: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Serialize)]
pub struct SubmitResponse {
    pub success: bool,
    pub message: String,
    pub problem_id: Option<String>,
}

#[derive(Deserialize)]
pub struct VerifierSolutionPayload {
    pub solution: String,
}

#[derive(Deserialize)]
pub struct VisibilityPayload {
    pub user_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct ClaimPayload {}

#[derive(Serialize)]
pub struct ClaimResponse {
    pub success: bool,
    pub message: String,
}

impl Problem {
    /// Check if a given user_id is the author of this problem
    pub fn is_author(&self, user_id: &str) -> bool {
        self.author_id == user_id
    }
}

/// Stripped-down problem view for listing (no solutions, no content for unauthorized)
#[derive(Debug, Clone, Serialize)]
pub struct ProblemListItem {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub contest: String,
    #[serde(default)]
    pub contest_id: Option<String>,
    pub difficulty: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub public_at: Option<DateTime<Utc>>,
    pub claimed_by: Option<String>,
    pub has_verifier_solution: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
}

// ============== OAuth ==============

#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthUserInfo {
    pub sub: String,
    pub username: String,
    #[serde(default)]
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize)]
pub struct RefreshTokenPayload {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub user_id: String,
}

// ============== Site Info ==============

#[derive(Serialize)]
pub struct SiteInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub title: String,
    #[serde(default)]
    pub difficulty_order: Vec<String>,
    #[serde(default)]
    pub showcase_problem_ids: Vec<String>,
    #[serde(default)]
    pub showcase_contest_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateSiteDescriptionPayload {
    pub description: String,
}

#[derive(Deserialize)]
pub struct ShowcaseConfigPayload {
    #[serde(default)]
    pub problem_ids: Vec<String>,
    #[serde(default)]
    pub contest_ids: Vec<String>,
}

// ============== Member Groups API ==============

#[derive(Deserialize)]
pub struct CreateGroupPayload {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateGroupPayload {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetUserGroupsPayload {
    pub group_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetUserPermissionsPayload {
    pub permissions: Vec<String>,
}

// ============== Resource ACL ==============

#[derive(Deserialize)]
pub struct SetProblemAclPayload {
    #[serde(default)]
    pub editable_by: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetAclPayload {
    #[serde(default)]
    pub visible_to: Vec<String>,
    #[serde(default)]
    pub editable_by: Vec<String>,
}

// ============== Application Configuration ==============

/// Top-level app config, read from /usr/share/mcguffin/config.toml
#[derive(Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub admin: AdminConfig,
    #[serde(default)]
    pub site: SiteConfig,
    pub oauth: OAuthConfig,
    #[serde(default)]
    pub difficulty: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    /// Discussion tags: name → { color, description }
    #[serde(default)]
    pub discussion_tags:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    /// Discussion emojis: name → { char }
    #[serde(default)]
    pub discussion_emojis:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    /// Role→permissions mapping. Overrides the hardcoded defaults.
    /// superadmin = ["*"] means all permissions.
    /// Example: [permissions.roles.admin] = ["view_team", "manage_team", ...]
    #[serde(default, deserialize_with = "deserialize_role_permissions")]
    pub permissions: std::collections::HashMap<String, Vec<String>>,
    /// Group permissions: uuid → { name, permissions }
    /// Example: [permissions.groups."uuid"] = { name = "出题组", permissions = ["submit_problem"] }
    #[serde(default, deserialize_with = "deserialize_permission_groups")]
    pub permission_groups:
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
}

/// Custom deserializer for role permissions that gracefully handles nested [permissions.roles] format
fn deserialize_role_permissions<'de, D>(
    deserializer: D,
) -> Result<std::collections::HashMap<String, Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::collections::HashMap;
    <HashMap<String, Vec<String>>>::deserialize(deserializer).or_else(|_| Ok(HashMap::new()))
}

/// Custom deserializer for permission_groups that gracefully handles TOML inline tables
fn deserialize_permission_groups<'de, D>(
    deserializer: D,
) -> Result<
    std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    D::Error,
>
where
    D: serde::Deserializer<'de>,
{
    use std::collections::HashMap;
    <HashMap<String, HashMap<String, serde_json::Value>>>::deserialize(deserializer)
        .or_else(|_| Ok(HashMap::new()))
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub site_url: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_data_file")]
    pub data_file: String,
}

fn default_port() -> u16 {
    3000
}
fn default_data_file() -> String {
    "mcguffin_data.json".to_string()
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

// ============== Difficulty Configuration ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyLevel {
    pub label: String,
    pub color: String,
}

/// Config.toml [difficulty] section: structured as table with level names as keys
/// Example:
///   [difficulty.Easy]
///   label = "简单"
///   color = "#22c55e"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyConfig {
    #[serde(flatten)]
    pub levels: std::collections::HashMap<String, DifficultyLevel>,
}

impl Default for DifficultyConfig {
    fn default() -> Self {
        let mut levels = std::collections::HashMap::new();
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

// ============== Login (merged admin + account) ==============

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

// ============== Notification ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub read: bool,
    pub created_at: DateTime<Utc>,
    /// Link to navigate to when clicked (e.g. "/problems", "/suggestions")
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Serialize)]
pub struct NotificationResponse {
    pub notifications: Vec<Notification>,
    pub unread_count: usize,
}

// ============== Discussion Tags & Emojis ==============

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

// ============== Unified Post Model ==============

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
    pub reactions: std::collections::HashMap<String, Vec<String>>,
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
    pub reactions: std::collections::HashMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

// ============== Post Payloads ==============

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

// ============== Community (unified post list) ==============

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
}

// ============== Tests ==============

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
    fn test_login_payload_deserialize() {
        let json = r#"{"password": "***"}"#;
        let payload: LoginPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.password, "***");
        assert!(payload.identifier.is_none());
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
        assert_eq!(config.server.data_file, "mcguffin_data.json"); // default
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

    #[test]
    fn test_problem_serde_roundtrip() {
        let problem = Problem {
            id: "test-id".to_string(),
            title: "Test Problem".to_string(),
            author_id: "user1".to_string(),
            author_name: "Author".to_string(),
            contest: "Round 1".to_string(),
            contest_id: Some("c1".to_string()),
            difficulty: "Hard".to_string(),
            content: "# Problem".to_string(),
            solution: Some("# Solution".to_string()),
            status: "published".to_string(),
            created_at: Utc::now(),
            public_at: Some(Utc::now()),
            claimed_by: None,
            verifier_solution: None,
            visible_to: vec![],
            link: Some("https://example.com".to_string()),
            remark: None,
            editable_by: vec![],
        };

        let json = serde_json::to_string(&problem).unwrap();
        let decoded: Problem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "test-id");
        assert_eq!(decoded.title, "Test Problem");
        assert_eq!(decoded.difficulty, "Hard");
        assert_eq!(decoded.status, "published");
        assert_eq!(decoded.link.unwrap(), "https://example.com");
    }

    #[test]
    fn test_problem_list_item_serialization() {
        let item = ProblemListItem {
            id: "p1".to_string(),
            title: "Problem 1".to_string(),
            author_id: "u1".to_string(),
            author_name: "User".to_string(),
            contest: "".to_string(),
            contest_id: None,
            difficulty: "Easy".to_string(),
            status: "published".to_string(),
            created_at: Utc::now(),
            public_at: None,
            claimed_by: None,
            has_verifier_solution: false,
            link: None,
            remark: None,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"id\":\"p1\""));
        assert!(json.contains("\"title\":\"Problem 1\""));
        // link should be skipped when None
        assert!(!json.contains("\"link\""));
        // contest_id is serialized as null when None (no skip_serializing_if)
        assert!(json.contains("\"contest_id\":null"));
    }

    #[test]
    fn test_oauth_user_info() {
        let json = r#"{
            "sub": "123",
            "username": "test_user",
            "display_name": "Test User",
            "avatar_url": "https://example.com/av.png",
            "email": "test@example.com"
        }"#;
        let info: OAuthUserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.sub, "123");
        assert_eq!(info.username, "test_user");
        assert_eq!(info.display_name, "Test User");
        assert_eq!(info.email.unwrap(), "test@example.com");
    }

    #[test]
    fn test_oauth_user_info_minimal() {
        // Only required fields
        let json = r#"{"sub": "42", "username": "minimal"}"#;
        let info: OAuthUserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.sub, "42");
        assert_eq!(info.display_name, ""); // default
        assert!(info.avatar_url.is_none());
        assert!(info.email.is_none());
    }

    #[test]
    fn test_site_info_serialization() {
        let info = SiteInfo {
            name: "My Site".to_string(),
            version: "1.0.0".to_string(),
            description: "A site".to_string(),
            title: "My Site".to_string(),
            difficulty_order: vec![],
            showcase_problem_ids: vec![],
            showcase_contest_ids: vec![],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"My Site\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"description\":\"A site\""));
        assert!(json.contains("\"title\":\"My Site\""));
    }

    /// Verify all permissions in default_role_permissions() are in perms::ALL.
    /// This catches drift when adding new permissions — prevents the frontend
    /// from being out of sync with the backend's known permission set.
    #[test]
    fn test_all_role_permissions_are_in_known_set() {
        let role_perms = default_role_permissions();
        let known: std::collections::HashSet<&str> = perms::ALL.iter().cloned().collect();
        for (role, perms) in &role_perms {
            for p in perms {
                if p == PERM_WILDCARD {
                    continue; // wildcard is not in perms::ALL
                }
                assert!(
                    known.contains(p.as_str()),
                    "权限「{}」（角色: {}）不在 perms::ALL 中，请先在 perms 模块中定义",
                    p,
                    role,
                );
            }
        }
    }
}
