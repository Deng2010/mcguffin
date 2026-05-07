use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub solution: Option<String>,            // author's solution (markdown)
    pub status: String,                       // pending | approved | published | rejected
    pub created_at: DateTime<Utc>,
    pub public_at: Option<DateTime<Utc>>,
    pub claimed_by: Option<String>,           // user_id of verifier who claimed this
    pub verifier_solution: Option<String>,    // verifier's solution (markdown)
    pub visible_to: Vec<String>,              // user_ids who can see pending problem content
    #[serde(default)]
    pub link: Option<String>,
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
    /// author_name to set (admin only)
    #[serde(default)]
    pub author_name: Option<String>,
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
    pub showcase_problems: usize,
    pub showcase_contests: usize,
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
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub site_url: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_data_file")]
    pub data_file: String,
}

fn default_port() -> u16 { 3000 }
fn default_data_file() -> String { "mcguffin_data.json".to_string() }

#[derive(Deserialize)]
pub struct AdminConfig {
    pub password: String,
    #[serde(default = "default_admin_display_name")]
    pub display_name: String,
}

fn default_admin_display_name() -> String { "管理员".to_string() }

#[derive(Deserialize)]
pub struct SiteConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_showcase_problems")]
    pub showcase_problems: usize,
    #[serde(default = "default_showcase_contests")]
    pub showcase_contests: usize,
    #[serde(default)]
    pub difficulty_order: Option<Vec<String>>,
}

fn default_showcase_problems() -> usize { 8 }
fn default_showcase_contests() -> usize { 3 }

impl Default for SiteConfig {
    fn default() -> Self {
        Self { name: None, title: None, showcase_problems: 8, showcase_contests: 3, difficulty_order: None }
    }
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
        levels.insert("Easy".to_string(), DifficultyLevel { label: "简单".to_string(), color: "#22c55e".to_string() });
        levels.insert("Medium".to_string(), DifficultyLevel { label: "中等".to_string(), color: "#f59e0b".to_string() });
        levels.insert("Hard".to_string(), DifficultyLevel { label: "困难".to_string(), color: "#ef4444".to_string() });
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

// ============== Suggestion ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionReply {
    pub id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: String,
    pub status: String, // "open" | "in_progress" | "resolved" | "closed"
    #[serde(default)]
    pub replies: Vec<SuggestionReply>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateSuggestionPayload {
    pub title: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct CreateSuggestionReplyPayload {
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateSuggestionPayload {
    #[serde(default)]
    pub status: Option<String>,
}

// ============== Announcement ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: String,
    pub pinned: bool,
    #[serde(default = "default_public")]
    pub public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_public() -> bool { true }

#[derive(Deserialize)]
pub struct CreateAnnouncementPayload {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default = "default_public")]
    pub public: bool,
}

#[derive(Deserialize)]
pub struct UpdateAnnouncementPayload {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub pinned: Option<bool>,
    #[serde(default)]
    pub public: Option<bool>,
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
        assert_eq!(config.difficulty.get("Easy").unwrap().get("label").unwrap(), "简单");
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
            showcase_problems: 8,
            showcase_contests: 3,
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
}
