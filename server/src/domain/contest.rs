use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
