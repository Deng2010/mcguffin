use crate::state::{AppState, ADMIN_USER_ID};
use crate::types::{self, AuditEntry, User};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;

const SESSION_MAX_AGE_SECS: i64 = 24 * 60 * 60; // 24 hours

// ============== URL Encoding ==============

pub fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}

// ============== Auth Header Parser ==============

pub fn get_token_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

// ============== Shared Auth Helpers ==============

/// Resolve user from token; returns (user_id, user)
/// Checks session expiry (24h inactivity) and updates last_active timestamp
pub async fn resolve_user(state: &AppState, headers: &axum::http::HeaderMap) -> Option<(String, User)> {
    let token = get_token_from_headers(headers)?;
    let now = Utc::now();

    // Check session expiry
    {
        let mut sessions = state.sessions.write().await;
        if let Some(entry) = sessions.get(&token) {
            let elapsed = (now - entry.last_active).num_seconds();
            if elapsed > SESSION_MAX_AGE_SECS {
                // Session expired — clean up
                sessions.remove(&token);
                // Drop lock before save
                drop(sessions);
                state.save().await;
                return None;
            }
        } else {
            return None;
        }

        // Update last_active time — we already have write lock
        if let Some(entry) = sessions.get_mut(&token) {
            entry.last_active = now;
        }

        let user_id = sessions.get(&token)?.user_id.clone();
        let users = state.users.read().await;
        let mut user = users.get(&user_id)?.clone();
        drop(users);
        drop(sessions);
        // Set computed effective_role on the response
        user.effective_role = user.compute_effective_role().to_string();
        Some((user_id, user))
    }
}

/// Check if user has admin role (includes superadmin)
pub async fn is_admin(state: &AppState, user_id: &str) -> bool {
    let users = state.users.read().await;
    users.get(user_id)
        .map(|u| u.role == "admin" || u.role == "superadmin")
        .unwrap_or(false)
}

/// Check if user is the superadmin
pub async fn is_superadmin(_state: &AppState, user_id: &str) -> bool {
    user_id == ADMIN_USER_ID
}

/// Check if user is a team member
pub async fn is_team_member(state: &AppState, user_id: &str) -> bool {
    let members = state.team_members.read().await;
    members.values().any(|m| m.user_id == user_id)
}

// ============== Unified Permission Check ==============

/// Resolve user from headers, check session validity, and verify the user has the
/// given permission. Returns the (user_id, User) pair on success.
///
/// On failure returns an appropriate HTTP error response (401 Unauthorized or
/// 403 Forbidden) that can be `?`-returned from the handler.
pub async fn require_permission(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    permission: &str,
) -> Result<(String, User), impl IntoResponse> {
    let (user_id, user) = resolve_user(state, headers)
        .await
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "success": false,
                    "message": "未登录或会话已过期"
                })),
            )
        })?;

    if !check_permission(state, &user, permission).await {
        state.log_audit(AuditEntry {
            timestamp: Utc::now(),
            user_id: user_id.clone(),
            user_name: user.display_name.clone(),
            action: permission.to_string(),
            resource: String::new(),
            result: "deny".to_string(),
            reason: "权限检查未通过".to_string(),
        }).await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "success": false,
                "message": format!("没有「{}」权限", permission)
            })),
        ));
    }

    Ok((user_id, user))
}

/// Check whether a user has a given permission, based on their role.
/// Supports the wildcard "*" (superadmin sees all).
pub async fn check_permission(state: &AppState, user: &User, permission: &str) -> bool {
    let perms_map = state.role_permissions.read().await;
    if let Some(user_perms) = perms_map.get(&user.role) {
        // Wildcard check
        if user_perms.iter().any(|p| p == types::PERM_WILDCARD) {
            return true;
        }
        user_perms.iter().any(|p| p == permission)
    } else {
        // Fallback: look up in default set (in case role isn't configured)
        let defaults = types::default_role_permissions();
        defaults
            .get(&user.role)
            .map(|p| p.iter().any(|p| p == permission))
            .unwrap_or(false)
    }
}

/// Like require_permission but returns a Json error response (for handlers
/// that return `Json<serde_json::Value>` instead of `Result<..., impl IntoResponse>`).
pub async fn require_permission_json(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    permission: &str,
) -> Result<(String, User), Json<serde_json::Value>> {
    let (uid, user) = match resolve_user(state, headers).await {
        Some(u) => u,
        None => return Err(Json(serde_json::json!({
            "success": false,
            "message": "未登录或会话已过期"
        }))),
    };
    let perms = state.role_permissions.read().await;
    let has_perm = perms
        .get(&user.role)
        .map(|p| p.iter().any(|p| p == types::PERM_WILDCARD || p == permission))
        .unwrap_or(false);
    if has_perm {
        Ok((uid, user))
    } else {
        state.log_audit(AuditEntry {
            timestamp: Utc::now(),
            user_id: uid.clone(),
            user_name: user.display_name.clone(),
            action: permission.to_string(),
            resource: String::new(),
            result: "deny".to_string(),
            reason: "权限检查未通过".to_string(),
        }).await;
        Err(Json(serde_json::json!({
            "success": false,
            "message": "权限不足"
        })))
    }
}

// ============== Tests ==============

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_url_encode_basic() {
        // Unreserved characters should pass through unchanged
        assert_eq!(url_encode("abc123"), "abc123");
        assert_eq!(url_encode("ABCDEF"), "ABCDEF");
        assert_eq!(url_encode("-_."), "-_.");
    }

    #[test]
    fn test_url_encode_special_chars() {
        // Reserved characters should be percent-encoded
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a&b"), "a%26b");
        assert_eq!(url_encode("a=b"), "a%3Db");
        assert_eq!(url_encode("https://example.com"), "https%3A%2F%2Fexample.com");
    }

    #[test]
    fn test_url_encode_unicode() {
        // Unicode multi-byte characters should be percent-encoded per byte
        assert_eq!(url_encode("中文"), "%E4%B8%AD%E6%96%87");
        assert_eq!(url_encode("日本語"), "%E6%97%A5%E6%9C%AC%E8%AA%9E");
        assert_eq!(url_encode("©"), "%C2%A9");
    }

    #[test]
    fn test_url_encode_mixed() {
        assert_eq!(url_encode("a b c"), "a%20b%20c");
        assert_eq!(url_encode(""), "");
        assert_eq!(url_encode("~"), "~");
    }

    #[test]
    fn test_get_token_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer my-test-token".parse().unwrap());
        assert_eq!(get_token_from_headers(&headers), Some("my-test-token".to_string()));
    }

    #[test]
    fn test_get_token_missing_header() {
        let headers = HeaderMap::new();
        assert_eq!(get_token_from_headers(&headers), None);
    }

    #[test]
    fn test_get_token_wrong_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Basic dXNlcjpwYXNz".parse().unwrap());
        assert_eq!(get_token_from_headers(&headers), None);
    }

    #[test]
    fn test_get_token_empty_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer ".parse().unwrap());
        assert_eq!(get_token_from_headers(&headers), Some("".to_string()));
    }
}
