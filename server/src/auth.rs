use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::{AppState, ADMIN_USER_ID};
use crate::types::*;
use crate::utils::url_encode;

// ============== Merged Login (admin + account) ==============

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Json<LoginResponse> {
    // If identifier is provided, try account login (by username or display_name)
    if let Some(identifier) = &payload.identifier {
        let identifier = identifier.trim();
        if identifier.is_empty() {
            return Json(LoginResponse { success: false, message: "请输入账户名或显示名称".to_string(), token: None });
        }

        let users = state.users.read().await;
        // Find user by username or display_name
        let found = users.values().find(|u| {
            u.username == identifier || u.display_name == identifier
        }).cloned();
        drop(users);

        match found {
            Some(user) => {
                match &user.password_hash {
                    Some(hash) => {
                        match bcrypt::verify(&payload.password, hash) {
                            Ok(true) => {
                                // Password correct — create session
                                let session_token = Uuid::new_v4().to_string();
                                state.sessions.write().await.insert(session_token.clone(), user.id.clone());
                                state.session_times.write().await.insert(session_token.clone(), Utc::now());
                                Json(LoginResponse {
                                    success: true,
                                    message: "登录成功".to_string(),
                                    token: Some(session_token),
                                })
                            }
                            _ => Json(LoginResponse {
                                success: false,
                                message: "密码错误".to_string(),
                                token: None,
                            })
                        }
                    }
                    None => Json(LoginResponse {
                        success: false,
                        message: "该用户未设置密码，请使用其他方式登录或先在个人资料中设置密码".to_string(),
                        token: None,
                    })
                }
            }
            None => Json(LoginResponse {
                success: false,
                message: "未找到该账户".to_string(),
                token: None,
            })
        }
    } else {
        // No identifier — admin password login (backward compatible)
        if payload.password != state.admin_password {
            return Json(LoginResponse {
                success: false,
                message: "密码错误".to_string(),
                token: None,
            });
        }

        let session_token = Uuid::new_v4().to_string();
        state.sessions.write().await.insert(session_token.clone(), ADMIN_USER_ID.to_string());
        state.session_times.write().await.insert(session_token.clone(), Utc::now());

        Json(LoginResponse {
            success: true,
            message: "登录成功".to_string(),
            token: Some(session_token),
        })
    }
}

// ============== OAuth Authorize ==============

pub async fn oauth_authorize(
    State(state): State<AppState>,
) -> impl IntoResponse {
    use sha2::{Sha256, Digest};
    use base64::Engine;
    
    // Generate PKCE code_verifier (43-128 chars, URL-safe)
    let code_verifier: String = (0..43)
        .map(|_| {
            const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
            let idx = (Uuid::new_v4().as_u128() % CHARSET.len() as u128) as usize;
            CHARSET[idx] as char
        })
        .collect();
    
    // Derive code_challenge = BASE64URL(SHA256(code_verifier))
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);
    
    // Generate CSRF state
    let state_csrf = Uuid::new_v4().to_string();
    
    let authorize_url = format!(
        "https://www.cpoauth.com/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=openid+profile&state={}&code_challenge={}&code_challenge_method=S256",
        state.cpoauth_client_id,
        url_encode(&state.cpoauth_redirect_uri),
        state_csrf,
        code_challenge,
    );
    
    // Set cookies for code_verifier and state (HttpOnly, 10min)
    let state_cookie = axum::http::HeaderValue::from_str(
        &format!("oauth_state={}; Path=/api/oauth; HttpOnly; Max-Age=600; SameSite=Lax", state_csrf)
    ).expect("valid cookie header value");
    let verifier_cookie = axum::http::HeaderValue::from_str(
        &format!("oauth_code_verifier={}; Path=/api/oauth; HttpOnly; Max-Age=600; SameSite=Lax", code_verifier)
    ).expect("valid cookie header value");
    
    (
        axum::response::AppendHeaders([
            axum::http::header::SET_COOKIE, axum::http::header::SET_COOKIE,
        ].into_iter().zip([state_cookie, verifier_cookie])),
        Redirect::to(&authorize_url),
    )
}

// ============== OAuth Callback ==============

pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let fe = state.site_url.clone();
    let code = params.get("code").cloned().unwrap_or_default();
    
    if code.is_empty() {
        return Redirect::to(&format!("{}#/login?error=no_code", fe));
    }
    
    // Validate state from cookie
    let cookie_state = headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .find_map(|c| {
            let c = c.trim();
            c.strip_prefix("oauth_state=").map(|v| v.to_string())
        });
    
    let callback_state = params.get("state").cloned();
    if let (Some(cs), Some(bs)) = (&callback_state, &cookie_state) {
        if cs != bs {
            return Redirect::to(&format!("{}#/login?error=state_mismatch", fe));
        }
    }
    
    // Read code_verifier from cookie
    let code_verifier = headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .find_map(|c| {
            let c = c.trim();
            c.strip_prefix("oauth_code_verifier=").map(|v| v.to_string())
        })
        .unwrap_or_default();
    
    match exchange_token(
        &code,
        &code_verifier,
        &state.cpoauth_client_id,
        &state.cpoauth_client_secret,
        &state.cpoauth_redirect_uri,
    ).await {
        Ok(token_resp) => {
            match get_user_info(&token_resp.access_token).await {
                Ok(userinfo) => {
                    let user_id = userinfo.sub.clone();
                    let team_status = {
                        let members = state.team_members.read().await;
                        if members.values().any(|m| m.user_id == user_id) {
                            "joined".to_string()
                        } else {
                            let requests = state.join_requests.read().await;
                            if requests.values().any(|r| r.user_id == user_id && r.status == "pending") {
                                "pending".to_string()
                            } else {
                                "none".to_string()
                            }
                        }
                    };
                    
                    let role = {
                        // New users: team member → "member", otherwise "guest"
                        // Existing users: preserve their current role
                        let users_map = state.users.read().await;
                        if let Some(existing) = users_map.get(&user_id) {
                            existing.role.clone()
                        } else {
                            let members = state.team_members.read().await;
                            if members.values().any(|m| m.user_id == user_id) {
                                "member".to_string()
                            } else {
                                "guest".to_string()
                            }
                        }
                    };
                    
                    let mut users = state.users.write().await;
                    if let Some(existing) = users.get_mut(&user_id) {
                        // User already exists — preserve custom fields (display_name, avatar_url, bio, created_at)
                        // Only update OAuth-provided fields and computed role/status
                        existing.username = userinfo.username;
                        if !userinfo.email.is_none() {
                            existing.email = userinfo.email;
                        }
                        existing.role = role;
                        existing.team_status = team_status;
                    } else {
                        // New user — use OAuth data
                        let display_name = if userinfo.display_name.is_empty() {
                            userinfo.username.clone()
                        } else {
                            userinfo.display_name
                        };
                        let user = User {
                            id: user_id.clone(),
                            username: userinfo.username,
                            display_name,
                            avatar_url: userinfo.avatar_url,
                            email: userinfo.email,
                            role,
                            team_status,
                            created_at: Utc::now(),
                            bio: String::new(),
                            password_hash: None,
                        };
                        users.insert(user.id.clone(), user);
                    }
                    drop(users);
                    
                    let session_token = Uuid::new_v4().to_string();
                    state.sessions.write().await.insert(session_token.clone(), user_id.clone());
                    state.session_times.write().await.insert(session_token.clone(), Utc::now());
                    // Remove old refresh tokens for this user to prevent accumulation
                    {
                        let mut rts = state.refresh_tokens.write().await;
                        rts.retain(|_, uid| uid != &user_id);
                        rts.insert(token_resp.refresh_token.clone(), user_id);
                    }
                    
                    state.save().await;
                    
                    Redirect::to(&format!("{}#/auth/callback?token={}", fe, session_token))
                }
                Err(e) => {
                    Redirect::to(&format!("{}#/login?error=userinfo_failed&msg={}", fe, url_encode(&e)))
                }
            }
        }
        Err(e) => {
            Redirect::to(&format!("{}#/login?error=token_failed&msg={}", fe, url_encode(&e)))
        }
    }
}

async fn exchange_token(code: &str, code_verifier: &str, client_id: &str, client_secret: &str, redirect_uri: &str) -> Result<OAuthTokenResponse, String> {
    let client = reqwest::Client::new();

    let mut body = serde_json::json!({
        "grant_type": "authorization_code",
        "code": code,
        "client_id": client_id,
        "client_secret": client_secret,
        "redirect_uri": redirect_uri,
    });
    
    if !code_verifier.is_empty() {
        body["code_verifier"] = serde_json::Value::String(code_verifier.to_string());
    }
    
    let response = client
        .post("https://www.cpoauth.com/api/oauth/token")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.status().is_success() {
        response.json::<OAuthTokenResponse>().await.map_err(|e| e.to_string())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("CP OAuth token exchange failed ({}): {}", status, body))
    }
}

async fn get_user_info(access_token: &str) -> Result<OAuthUserInfo, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.cpoauth.com/api/oauth/userinfo")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.status().is_success() {
        response.json::<OAuthUserInfo>().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to get user info".to_string())
    }
}

// ============== Refresh Token ==============

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenPayload>,
) -> Json<serde_json::Value> {
    let refresh_token = payload.refresh_token.clone();
    let user_id = state.refresh_tokens.read().await.get(&refresh_token).cloned();

    if let Some(uid) = user_id {
        let access_token = format!("access_{}", Uuid::new_v4());
        let new_refresh_token = format!("refresh_{}", Uuid::new_v4());

        state.refresh_tokens.write().await.remove(&refresh_token);
        state.refresh_tokens.write().await.insert(new_refresh_token.clone(), uid);

        state.save().await;

        Json(serde_json::json!({
            "success": true,
            "access_token": access_token,
            "refresh_token": new_refresh_token,
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "openid profile email",
        }))
    } else {
        Json(serde_json::json!({
            "success": false,
            "message": "无效的 refresh token",
        }))
    }
}
