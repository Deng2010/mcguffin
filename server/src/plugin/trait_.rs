use serde::{Deserialize, Serialize};

/// Metadata describing a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub permissions_needed: Vec<String>,
}

/// A permission declared by a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionDef {
    pub key: String,
    pub label: String,
    pub description: String,
}

/// Where a plugin frontend route should be surfaced.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavPlacement {
    Main,
    Admin,
    Hidden,
}

/// A frontend route contributed by a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginRouteDef {
    pub path: String,
    pub label: String,
    pub icon: Option<String>,
    pub required_permission: Option<String>,
    pub nav_placement: NavPlacement,
}

/// How a plugin is loaded.
#[derive(Clone, Debug)]
pub enum PluginSource {
    Wasm { path: std::path::PathBuf },
    Sidecar { url: String },
}

/// Runtime context passed to WASM plugin calls.
///
/// Serialized to JSON and passed as a string to the WASM guest's
/// `_plugin_on_load`, `_plugin_on_unload`, and `_plugin_handle_request` exports.
#[derive(Clone, Serialize)]
pub struct PluginContext {
    pub base_url: String,
    pub user_id: String,
    pub user_role: String,
    pub request_method: String,
    pub request_path: String,
    pub request_body: serde_json::Value,
}

/// Response returned by a plugin's request handler.
/// Serialized from the JSON string returned by the WASM guest's
/// `_plugin_handle_request` export.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    pub body: String,
}
