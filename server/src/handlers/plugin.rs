use axum::{extract::State, http::StatusCode, Json};
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

/// Hot-reload plugins from the plugins directory.
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
