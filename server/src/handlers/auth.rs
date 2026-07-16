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

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct UserRow {
    id: String,
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    email: Option<String>,
    role: String,
    team_status: String,
    created_at: String,
    bio: String,
    password_hash: Option<String>,
    effective_role: String,
    group_ids: String,
    user_permissions: String,
}

impl UserRow {
    fn into_user(self) -> User {
        User {
            id: self.id,
            username: self.username,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            email: self.email,
            role: self.role,
            team_status: self.team_status,
            created_at: self.created_at.parse().unwrap_or_else(|_| Utc::now()),
            bio: self.bio,
            password_hash: self.password_hash,
            effective_role: self.effective_role,
            group_ids: serde_json::from_str(&self.group_ids).unwrap_or_default(),
            user_permissions: serde_json::from_str(&self.user_permissions).unwrap_or_default(),
        }
    }
}

// ============== Permissions (GET) ==============

/// GET /api/auth/permissions
/// Returns the current role→permissions mapping (no auth required — public info for frontend rendering).
pub async fn get_permissions(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let map = state.role_permissions.read().await.clone();
    Json(serde_json::json!(map))
}

// ============== Merged Login (admin + account) ==============

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Json<LoginResponse> {
    // If identifier is provided, try account login (by username or display_name)
    if let Some(identifier) = &payload.identifier {
        let identifier = identifier.trim();
        if identifier.is_empty() {
            return Json(LoginResponse {
                success: false,
                message: "请输入账户名或显示名称".to_string(),
                token: None,
            });
        }

        // Try SQLite first, then fallback to HashMap
        let found = match sqlx::query_as::<_, UserRow>(
            "SELECT id, username, display_name, avatar_url, email, role, team_status, \
             created_at, bio, password_hash, effective_role, group_ids, user_permissions \
             FROM users WHERE username = ? OR display_name = ?",
        )
        .bind(identifier)
        .bind(identifier)
        .fetch_optional(&state.db)
        .await
        {
            Ok(Some(row)) => Some(row.into_user()),
            _ => {
                // Fallback to HashMap
                let users = state.users.lock().await;
                users
                    .values()
                    .find(|u| u.username == *identifier || u.display_name == *identifier)
                    .cloned()
            }
        };

        match found {
            Some(user) => {
                let password_ok = match &user.password_hash {
                    Some(hash) => bcrypt::verify(&payload.password, hash).unwrap_or(false),
                    // 若用户未设密码哈希，管理员（admin）可回退到配置密码
                    None if user.id == ADMIN_USER_ID => payload.password == *state.admin_password.read().await,
                    None => false,
                };
                if password_ok {
                    let session_token = state.create_session(&user.id).await;
                    Json(LoginResponse {
                        success: true,
                        message: "登录成功".to_string(),
                        token: Some(session_token),
                    })
                } else if user.id == ADMIN_USER_ID && user.password_hash.is_none() {
                    Json(LoginResponse {
                        success: false,
                        message: "密码错误（可在配置文件 admin.password 中修改）".to_string(),
                        token: None,
                    })
                } else {
                    Json(LoginResponse {
                        success: false,
                        message: "密码错误".to_string(),
                        token: None,
                    })
                }
            }
            None => Json(LoginResponse {
                success: false,
                message: "未找到该账户".to_string(),
                token: None,
            }),
        }
    } else {
        // No identifier — admin password login (backward compatible)
        if payload.password != *state.admin_password.read().await {
            return Json(LoginResponse {
                success: false,
                message: "密码错误".to_string(),
                token: None,
            });
        }

        let session_token = state.create_session(ADMIN_USER_ID).await;

        Json(LoginResponse {
            success: true,
            message: "登录成功".to_string(),
            token: Some(session_token),
        })
    }
}

// ============== OAuth Authorize ==============

pub async fn oauth_authorize(State(state): State<AppState>) -> impl IntoResponse {
    use base64::Engine;
    use sha2::{Digest, Sha256};

    // Generate PKCE code_verifier (43-128 chars, URL-safe)
    let code_verifier: String = (0..43)
        .map(|_| {
            const CHARSET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
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
    let state_cookie = axum::http::HeaderValue::from_str(&format!(
        "oauth_state={}; Path=/api/oauth; HttpOnly; Max-Age=600; SameSite=Lax",
        state_csrf
    ))
    .expect("valid cookie header value");
    let verifier_cookie = axum::http::HeaderValue::from_str(&format!(
        "oauth_code_verifier={}; Path=/api/oauth; HttpOnly; Max-Age=600; SameSite=Lax",
        code_verifier
    ))
    .expect("valid cookie header value");

    (
        axum::response::AppendHeaders(
            [
                axum::http::header::SET_COOKIE,
                axum::http::header::SET_COOKIE,
            ]
            .into_iter()
            .zip([state_cookie, verifier_cookie]),
        ),
        Redirect::to(&authorize_url),
    )
}

// ============== OAuth Callback ==============

pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    tracing::info!("OAuth callback: code={:?}, state_param={:?}",
        params.get("code").map(|c| &c[..10.min(c.len())]),
        params.get("state"));
    let fe = state.site_url.clone();
    let code = params.get("code").cloned().unwrap_or_default();

    if code.is_empty() {
        tracing::warn!("OAuth callback: 缺少 code 参数");
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
            tracing::warn!("OAuth callback: state 不匹配，cookie={:?}, callback={:?}", bs, cs);
            return Redirect::to(&format!("{}#/login?error=state_mismatch", fe));
        }
    }
    tracing::info!("OAuth callback: state 验证通过");

    // Read code_verifier from cookie
    let code_verifier = headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .find_map(|c| {
            let c = c.trim();
            c.strip_prefix("oauth_code_verifier=")
                .map(|v| v.to_string())
        })
        .unwrap_or_default();

    tracing::info!("OAuth callback: 开始交换 token (code_verifier 长度={})", code_verifier.len());
    match exchange_token(
        &code,
        &code_verifier,
        &state.cpoauth_client_id,
        &state.cpoauth_client_secret,
        &state.cpoauth_redirect_uri,
        &state.http_client,
    )
    .await
    {
        Ok(token_resp) => {
            tracing::info!("OAuth callback: token 交换成功");
            match get_user_info(&token_resp.access_token, &state.http_client).await {
                Ok(userinfo) => {
                    tracing::info!("OAuth callback: 用户信息获取成功: sub={}", &userinfo.sub[..8.min(userinfo.sub.len())]);
                    let user_id = userinfo.sub.clone();
                    let team_status = {
                        let members = state.team_members.read().await;
                        if members.values().any(|m| m.user_id == user_id) {
                            "joined".to_string()
                        } else {
                            let requests = state.join_requests.read().await;
                            if requests
                                .values()
                                .any(|r| r.user_id == user_id && r.status == "pending")
                            {
                                "pending".to_string()
                            } else {
                                "none".to_string()
                            }
                        }
                    };

                    let role = {
                        // New users: team member → "member", otherwise "guest"
                        // Existing users: preserve their current role
                        let users_map = state.users.lock().await;
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

                    tracing::info!("OAuth callback: role 计算完成, role={}", role);

                    // Truncate username to 30 characters max
                    const MAX_USERNAME_LEN: usize = 30;
                    let username = if userinfo.username.chars().count() > MAX_USERNAME_LEN {
                        userinfo
                            .username
                            .chars()
                            .take(MAX_USERNAME_LEN)
                            .collect::<String>()
                    } else {
                        userinfo.username
                    };

                    // 提前获取用户信息并释放锁（避免临时变量在 if let 中存活到 else 块）
                    let existing_user = {
                        let users = state.users.lock().await;
                        users.get(&user_id).cloned()
                    };

                    if let Some(mut existing) = existing_user {
                        tracing::info!("OAuth callback: 用户已存在 ({}), 更新中...", &existing.username);
                        // User already exists — preserve custom fields (display_name, avatar_url, bio, created_at)
                        // Only update OAuth-provided fields and computed role/status
                        existing.username = username.clone();
                        if userinfo.email.is_some() {
                            existing.email = userinfo.email;
                        }
                        existing.role = role;
                        existing.team_status = team_status;
                        state.upsert_user(&existing).await;
                        tracing::info!("OAuth callback: upsert_user(已存在) 完成");
                    } else {
                        tracing::info!("OAuth callback: 新用户, 创建中...");
                        // New user — use OAuth data, truncate display_name to 30 chars
                        let display_name = if userinfo.display_name.is_empty() {
                            username.clone()
                        } else {
                            let dn = &userinfo.display_name;
                            if dn.chars().count() > 30 {
                                dn.chars().take(30).collect::<String>()
                            } else {
                                dn.clone()
                            }
                        };
                        // Compute effective_role for the new user
                        let effective_role: String = match team_status.as_str() {
                            "pending" => "guest".into(),
                            _ => match role.as_str() {
                                "superadmin" => "superadmin".into(),
                                "admin" => "admin".into(),
                                "guest" if team_status != "joined" => "guest".into(),
                                "member" if team_status == "joined" => "member".into(),
                                r => r.to_string(),
                            },
                        };
                        let user = User {
                            id: user_id.clone(),
                            username,
                            display_name,
                            avatar_url: userinfo.avatar_url,
                            email: userinfo.email,
                            role,
                            team_status,
                            created_at: Utc::now(),
                            bio: String::new(),
                            password_hash: None,
                            effective_role,
                            group_ids: Vec::new(),
                            user_permissions: Vec::new(),
                        };
                        state.upsert_user(&user).await;
                        tracing::info!("OAuth callback: upsert_user(新用户) 完成");
                    }

                    tracing::info!("OAuth callback: 开始创建 session...");
                    let session_token = state.create_session(&user_id).await;
                    tracing::info!("OAuth callback: session 创建完成");
                    // Remove old refresh tokens for this user to prevent accumulation
                    tracing::info!("OAuth callback: 清除旧 refresh token...");
                    state.clear_user_refresh_tokens(&user_id).await;
                    tracing::info!("OAuth callback: 旧 refresh token 已清除");
                    tracing::info!("OAuth callback: 设置新 refresh token...");
                    state
                        .set_refresh_token(token_resp.refresh_token.clone(), user_id)
                        .await;
                    tracing::info!("OAuth callback: 新 refresh token 已设置");
                    tracing::info!("OAuth callback: 登录成功，重定向到 /#/auth/callback");
                    Redirect::to(&format!("{}#/auth/callback?token={}", fe, session_token))
                }
                Err(e) => {
                    tracing::error!("OAuth callback: 获取用户信息失败: {}", e);
                    Redirect::to(&format!(
                    "{}#/login?error=userinfo_failed&msg={}",
                    fe,
                    url_encode(&e)
                ))
                }
            }
        }
        Err(e) => {
            tracing::error!("OAuth callback: token 交换失败: {}", e);
            Redirect::to(&format!(
            "{}#/login?error=token_failed&msg={}",
            fe,
            url_encode(&e)
        ))
        }
    }
}

async fn exchange_token(
    code: &str,
    code_verifier: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    http_client: &reqwest::Client,
) -> Result<OAuthTokenResponse, String> {
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

    let response = http_client
        .post("https://www.cpoauth.com/api/oauth/token")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if response.status().is_success() {
        response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|e| e.to_string())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!(
            "CP OAuth token exchange failed ({}): {}",
            status, body
        ))
    }
}

async fn get_user_info(
    access_token: &str,
    http_client: &reqwest::Client,
) -> Result<OAuthUserInfo, String> {
    let response = http_client
        .get("https://www.cpoauth.com/api/oauth/userinfo")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if response.status().is_success() {
        response
            .json::<OAuthUserInfo>()
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("Failed to get user info".to_string())
    }
}

// ============== Refresh Token ==============

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenPayload>,
) -> Json<serde_json::Value> {
    let old_token = payload.refresh_token.clone();
    let user_id = state.refresh_tokens.read().await.get(&old_token).cloned();

    if let Some(uid) = user_id {
        let access_token = format!("access_{}", Uuid::new_v4());
        let new_refresh_token = format!("refresh_{}", Uuid::new_v4());

        state.remove_refresh_token(&old_token).await;
        state
            .set_refresh_token(new_refresh_token.clone(), uid)
            .await;

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
