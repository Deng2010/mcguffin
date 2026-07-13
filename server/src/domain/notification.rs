use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
