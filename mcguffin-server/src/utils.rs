use crate::state::{AppState, ADMIN_USER_ID};
use crate::types::User;

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
pub async fn resolve_user(state: &AppState, headers: &axum::http::HeaderMap) -> Option<(String, User)> {
    let token = get_token_from_headers(headers)?;
    let user_id = state.sessions.read().await.get(&token)?.clone();
    let user = state.users.read().await.get(&user_id)?.clone();
    Some((user_id, user))
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
