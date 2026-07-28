// ============== Plugin API handlers ==============
//
// Endpoints:
//   POST   /api/plugins/register              — register plugin metadata
//   GET    /api/admin/plugins                  — list registered plugins
//   DELETE /api/admin/plugins/{plugin_id}      — unregister plugin & delete data
//   POST   /api/admin/plugins/{plugin_id}/enable   — enable a plugin
//   POST   /api/admin/plugins/{plugin_id}/disable  — disable a plugin
//   GET    /api/plugins/{plugin_id}/users      — list team members (needs read:team)
//   GET    /api/plugins/{plugin_id}/users/me   — current user info
//   GET    /api/plugins/{plugin_id}/users/{id} — user info (needs read:users)
//   GET    /api/plugins/{plugin_id}/data       — read KV (needs storage)
//   POST   /api/plugins/{plugin_id}/data       — write KV (needs storage)
//   POST   /api/plugins/{plugin_id}/notify     — send notification (needs notify)
//
// Permission model:
//   - Each plugin declares requested permissions in its manifest.
//   - At registration time, all requested permissions are granted (trust model;
//     admin review UI can be added later).
//   - Data APIs check the stored plugin permissions before serving.
//   - Disabled plugins are rejected from all API endpoints (except listing).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;

use crate::domain::plugin::{
    plugin_perms, NotifyPayload, PluginManifest, PluginRegistration, SetDataPayload,
};
use crate::state::AppState;
use crate::types::{Notification, PERM_WILDCARD};
use crate::utils::AuthUser;

// ── Helpers ──

/// Check if a plugin has a specific permission.
fn plugin_has_perm(plugin: &PluginManifest, perm: &str) -> bool {
    plugin.permissions.iter().any(|p| p == perm)
}

/// Check if a plugin is enabled, returning 403 if disabled.
fn require_plugin_enabled(plugin: &PluginManifest) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !plugin.enabled {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "success": false,
                "message": "插件已被禁用，请联系管理员启用"
            })),
        ));
    }
    Ok(())
}

/// Require a plugin permission, returning 403 if not granted.
macro_rules! require_plugin_perm {
    ($plugin:expr, $perm:expr) => {
        if !plugin_has_perm($plugin, $perm) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("插件未申请权限: {}", $perm)
                })),
            ));
        }
    };
}

// ── Register ──

/// POST /api/plugins/register
/// Called by the frontend PluginRegistry on load. Stores plugin metadata
/// and grants requested permissions. Idempotent — re-registering overwrites.
pub async fn register_plugin(
    State(state): State<AppState>,
    Json(payload): Json<PluginRegistration>,
) -> Json<serde_json::Value> {
    // Validate permissions: only allow known permission strings
    let valid_perms: Vec<String> = payload
        .permissions
        .iter()
        .filter(|p| plugin_perms::ALL.contains(&p.as_str()))
        .cloned()
        .collect();

    let manifest = PluginManifest {
        id: payload.id.clone(),
        name: payload.manifest.name.clone(),
        version: payload.manifest.version.clone(),
        description: payload.manifest.description.clone(),
        author: payload.manifest.author.clone(),
        permissions: valid_perms.clone(),
        enabled: true,
    };

    {
        let mut plugins = state.plugins.write().await;
        plugins.insert(payload.id.clone(), manifest.clone());
    }

    tracing::info!(
        "plugin registered: {} v{} (permissions: {:?})",
        manifest.id,
        manifest.version,
        valid_perms,
    );

    Json(serde_json::json!({
        "success": true,
        "message": "插件已注册",
        "plugin": {
            "id": manifest.id,
            "name": manifest.name,
            "version": manifest.version,
            "permissions_needed": valid_perms,
        }
    }))
}

// ── List plugins (public) ──

/// GET /api/plugins
/// Returns minimal plugin info (no auth required). Used by the frontend to
/// know which plugins are enabled/disabled so it can hide disabled ones.
pub async fn list_plugins_public(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let plugins = state.plugins.read().await;
    let list: Vec<serde_json::Value> = plugins
        .values()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "version": p.version,
                "enabled": p.enabled,
            })
        })
        .collect();

    Json(serde_json::json!({"plugins": list}))
}

// ── List plugins (admin) ──

/// GET /api/admin/plugins
pub async fn list_plugins(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let plugins = state.plugins.read().await;
    let list: Vec<&PluginManifest> = plugins.values().collect();

    Ok(Json(serde_json::json!({
        "plugins": list,
    })))
}

// ── Unregister plugin (admin) ──

/// DELETE /api/admin/plugins/{plugin_id}
pub async fn unregister_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let removed = {
        let mut plugins = state.plugins.write().await;
        plugins.remove(&plugin_id)
    };

    // Also clean up plugin data
    {
        let mut data = state.plugin_data.write().await;
        data.remove(&plugin_id);
    }

    match removed {
        Some(p) => {
            tracing::info!("plugin unregistered: {} ({})", p.name, p.id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("插件「{}」已卸载", p.name),
            })))
        }
        None => Ok(Json(serde_json::json!({
            "success": false,
            "message": "插件不存在",
        }))),
    }
}

// ── Enable / Disable plugin (admin) ──

/// POST /api/admin/plugins/{plugin_id}/enable
pub async fn enable_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let mut plugins = state.plugins.write().await;
    if let Some(plugin) = plugins.get_mut(&plugin_id) {
        plugin.enabled = true;
        tracing::info!("plugin enabled: {} ({})", plugin.name, plugin.id);
        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("插件「{}」已启用", plugin.name),
        })))
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "插件不存在",
        })))
    }
}

/// POST /api/admin/plugins/{plugin_id}/disable
pub async fn disable_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let mut plugins = state.plugins.write().await;
    if let Some(plugin) = plugins.get_mut(&plugin_id) {
        plugin.enabled = false;
        tracing::info!("plugin disabled: {} ({})", plugin.name, plugin.id);
        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("插件「{}」已禁用", plugin.name),
        })))
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "插件不存在",
        })))
    }
}

// ── Users: list team members ──

/// GET /api/plugins/{plugin_id}/users
/// Returns team members. Requires read:team plugin permission.
pub async fn plugin_list_users(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "插件未注册"})),
            )
        })?
        .clone();
    drop(plugins);

    require_plugin_enabled(&plugin)?;
    require_plugin_perm!(&plugin, plugin_perms::READ_TEAM);

    let members = state.team_members.read().await;
    let users = state.users.read().await;

    let list: Vec<serde_json::Value> = members
        .values()
        .filter_map(|m| {
            let user = users.get(&m.user_id)?;
            Some(serde_json::json!({
                "user_id": m.user_id,
                "username": user.username,
                "display_name": user.display_name,
                "avatar_url": user.avatar_url,
                "role": user.role,
                "joined_at": m.joined_at,
            }))
        })
        .collect();

    Ok(Json(serde_json::json!({
        "members": list,
        "count": list.len(),
    })))
}

// ── Users: current user ──

/// GET /api/plugins/{plugin_id}/users/me
/// Returns the currently authenticated user. No plugin permission required
/// (every plugin should know who the caller is).
pub async fn plugin_user_me(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
    auth: AuthUser,
) -> Json<serde_json::Value> {
    let plugins = state.plugins.read().await;
    let plugin = match plugins.get(&plugin_id) {
        Some(p) => p.clone(),
        None => return Json(serde_json::json!({"success": false, "message": "插件未注册"})),
    };
    drop(plugins);

    if !plugin.enabled {
        return Json(serde_json::json!({"success": false, "message": "插件已被禁用"}));
    }

    let users = state.users.read().await;
    let user = match users.get(&auth.user_id) {
        Some(u) => u.clone(),
        None => {
            return Json(serde_json::json!({"success": false, "message": "用户不存在"}))
        }
    };

    Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "avatar_url": user.avatar_url,
        "role": user.role,
        "effective_role": user.effective_role,
        "team_status": user.team_status,
        "bio": user.bio,
        "created_at": user.created_at,
    }))
}

// ── Users: get specific user ──

/// GET /api/plugins/{plugin_id}/users/{user_id}
/// Returns limited user profile. Requires read:users permission.
/// Email is only included if the plugin has read:users:email.
pub async fn plugin_user_get(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((plugin_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "插件未注册"})),
            )
        })?
        .clone();
    drop(plugins);

    require_plugin_enabled(&plugin)?;
    require_plugin_perm!(&plugin, plugin_perms::READ_USERS);

    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"success": false, "message": "用户不存在"})),
        )
    })?;

    let has_email = plugin_has_perm(&plugin, plugin_perms::READ_USERS_EMAIL);

    let mut result = serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "avatar_url": user.avatar_url,
        "role": user.role,
        "effective_role": user.effective_role,
        "team_status": user.team_status,
        "bio": user.bio,
        "created_at": user.created_at,
    });

    if has_email {
        result["email"] = serde_json::Value::String(user.email.clone().unwrap_or_default());
    }

    Ok(Json(result))
}

// ── Data: KV store ──

/// GET /api/plugins/{plugin_id}/data?namespace=X&key=Y
pub async fn plugin_get_data(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "插件未注册"})),
            )
        })?
        .clone();
    drop(plugins);

    require_plugin_enabled(&plugin)?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let namespace = params.get("namespace").cloned().unwrap_or_default();
    let key = params.get("key").cloned().unwrap_or_default();

    if namespace.is_empty() || key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "message": "namespace 和 key 不能为空"})),
        ));
    }

    let data = state.plugin_data.read().await;
    let value = data
        .get(&plugin_id)
        .and_then(|ns| ns.get(&namespace))
        .and_then(|kv| kv.get(&key))
        .cloned()
        .unwrap_or_default();

    Ok(Json(serde_json::json!({"value": value})))
}

/// POST /api/plugins/{plugin_id}/data
/// Body: { namespace, key, value }
pub async fn plugin_set_data(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<SetDataPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "插件未注册"})),
            )
        })?
        .clone();
    drop(plugins);

    require_plugin_enabled(&plugin)?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    if payload.namespace.is_empty() || payload.key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "message": "namespace 和 key 不能为空"})),
        ));
    }

    {
        let mut data = state.plugin_data.write().await;
        let ns = data
            .entry(plugin_id.clone())
            .or_insert_with(HashMap::new);
        let kv = ns
            .entry(payload.namespace.clone())
            .or_insert_with(HashMap::new);
        kv.insert(payload.key.clone(), payload.value);
    }

    Ok(Json(serde_json::json!({"success": true})))
}

// ── Notify ──

/// POST /api/plugins/{plugin_id}/notify
/// Body: { user_id, title, body, link? }
pub async fn plugin_notify(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<NotifyPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "插件未注册"})),
            )
        })?
        .clone();
    drop(plugins);

    require_plugin_enabled(&plugin)?;
    require_plugin_perm!(&plugin, plugin_perms::NOTIFY);

    if payload.title.is_empty() || payload.body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "message": "title 和 body 不能为空"})),
        ));
    }

    // Create notification (reuse existing notification infrastructure)
    let notif = Notification {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: payload.user_id.clone(),
        title: payload.title,
        body: payload.body,
        link: payload.link,
        read: false,
        created_at: chrono::Utc::now(),
    };

    state.insert_notification(&notif).await;

    Ok(Json(serde_json::json!({"success": true, "message": "通知已发送"})))
}
