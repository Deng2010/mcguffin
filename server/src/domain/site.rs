use serde::{Deserialize, Serialize};

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
    /// 站点时区（如 "UTC+8"）
    #[serde(default)]
    pub timezone: String,
    /// 服务器当前时间（Unix 毫秒），用于判定比赛状态。
    pub server_time: i64,
}

#[derive(Deserialize)]
pub struct UpdateSiteDescriptionPayload {
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            timezone: "UTC+8".to_string(),
            server_time: 0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"My Site\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"description\":\"A site\""));
        assert!(json.contains("\"title\":\"My Site\""));
        assert!(json.contains("\"timezone\":\"UTC+8\""));
    }
}
