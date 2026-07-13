use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
