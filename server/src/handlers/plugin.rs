use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
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

/// Register or update a plugin's metadata from the frontend.
pub async fn register_plugin(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let plugin_id = body.get("id").and_then(|v| v.as_str()).unwrap_or("");
    if let Err(e) = crate::plugin::PluginManager::validate_plugin_id(plugin_id) {
        return Json(serde_json::json!({ "success": false, "message": e }));
    }

    let manifest = serde_json::from_value(body.get("manifest").cloned().unwrap_or_default())
        .unwrap_or_else(|_| PluginManifest {
            id: plugin_id.to_string(),
            name: plugin_id.to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: None,
            homepage: None,
            permissions_needed: Vec::new(),
        });

    let routes = serde_json::from_value(body.get("routes").cloned().unwrap_or_default())
        .unwrap_or_default();

    let permissions = serde_json::from_value(body.get("permissions").cloned().unwrap_or_default())
        .unwrap_or_default();

    state.plugins.register(manifest, routes, permissions).await;

    Json(serde_json::json!({ "success": true }))
}

/// Install a plugin from a .zip archive.
/// The zip is extracted and plugin.json is read for metadata.
pub async fn install_plugin_zip(
    State(state): State<AppState>,
    auth: AuthUser,
    body: axum::body::Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::MANAGE_SITE).await?;

    let manifest = state
        .plugins
        .install_from_zip(&body)
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

/// Uninstall a plugin by id.
pub async fn uninstall_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::MANAGE_SITE).await?;

    state.plugins.unregister(&plugin_id).await;

    // Also remove plugin data from SQLite
    let _ = sqlx::query("DELETE FROM plugin_data WHERE plugin_id = ?")
        .bind(&plugin_id)
        .execute(&state.db)
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Plugin '{}' uninstalled", plugin_id),
    })))
}
