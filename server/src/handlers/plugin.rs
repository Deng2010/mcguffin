use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::plugin::trait_::PluginManifest;
use crate::state::AppState;
use crate::types::perms;
use crate::utils::AuthUser;

/// List all loaded plugins.
pub async fn get_plugins(State(state): State<AppState>) -> Json<Value> {
    let plugins = state.plugins.get_manifests().await;
    Json(serde_json::json!({ "plugins": plugins }))
}

/// Return all permission keys declared by loaded plugins.
pub async fn get_plugin_permissions(State(state): State<AppState>) -> Json<Value> {
    let per_plugin = state.plugins.plugin_permissions();
    let all_keys = state.plugins.all_plugin_permission_keys();
    Json(serde_json::json!({
        "permissions": per_plugin,
        "all_keys": all_keys,
    }))
}

/// List frontend routes contributed by plugins.
pub async fn get_plugin_routes(State(state): State<AppState>) -> Json<Value> {
    let routes = state.plugins.get_frontend_routes().await;
    let manifests = state.plugins.get_manifests().await;
    let manifest_by_id: std::collections::HashMap<String, PluginManifest> = manifests
        .into_iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    let plugins: Vec<Value> = routes
        .into_iter()
        .map(|(id, routes)| {
            serde_json::json!({
                "id": id,
                "manifest": manifest_by_id.get(&id),
                "routes": routes,
            })
        })
        .collect();
    Json(serde_json::json!({ "plugins": plugins }))
}

/// Hot-reload plugins from the plugins directory (rescan for new/removed WASM files).
pub async fn reload_plugins(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::MANAGE_SITE).await?;
    let (loaded, removed) = state.plugins.hot_reload().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": e,
            })),
        )
    })?;
    Ok(Json(serde_json::json!({
        "reloaded": true,
        "plugins_loaded": loaded,
        "plugins_removed": removed,
    })))
}

// ── Install / Uninstall ───────────────────────────────────

#[derive(Deserialize)]
pub struct InstallUrlQuery {
    pub id: String,
    pub url: String,
}

/// Install a plugin by downloading from a URL.
pub async fn install_plugin_from_url(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<InstallUrlQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::MANAGE_SITE).await?;

    let manifest = state
        .plugins
        .install_plugin_from_url(&query.id, &query.url)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "message": e,
                })),
            )
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "plugin": manifest,
    })))
}

/// Upload and install a WASM plugin.
/// The request body should be the raw WASM bytes; the plugin id is derived
/// from the Content-Disposition filename or passed as a query param.
pub async fn install_plugin_upload(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<InstallUrlQuery>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::MANAGE_SITE).await?;

    let manifest = state
        .plugins
        .install_plugin_from_bytes(&query.id, &body)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "message": e,
                })),
            )
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "plugin": manifest,
    })))
}

/// Uninstall (delete) a plugin by id.
pub async fn uninstall_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::MANAGE_SITE).await?;

    state.plugins.uninstall_plugin(&plugin_id).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": e,
            })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Plugin '{}' uninstalled", plugin_id),
    })))
}
