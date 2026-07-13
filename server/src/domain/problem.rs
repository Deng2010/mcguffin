use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visible_to: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            visible_to: vec![],
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"id\":\"p1\""));
        assert!(json.contains("\"title\":\"Problem 1\""));
        // link should be skipped when None
        assert!(!json.contains("\"link\""));
        // contest_id is serialized as null when None (no skip_serializing_if)
        assert!(json.contains("\"contest_id\":null"));
    }
}
