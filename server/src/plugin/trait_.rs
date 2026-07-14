use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::state::AppState;

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

/// How a plugin is implemented.
pub enum PluginSource {
    BuiltIn(Box<dyn Plugin>),
    Wasm { path: PathBuf },
    Sidecar { url: String },
}

impl Clone for PluginSource {
    fn clone(&self) -> Self {
        match self {
            Self::BuiltIn(plugin) => Self::BuiltIn(plugin.clone_box()),
            Self::Wasm { path } => Self::Wasm { path: path.clone() },
            Self::Sidecar { url } => Self::Sidecar { url: url.clone() },
        }
    }
}

/// Runtime context passed to plugins.
#[derive(Clone)]
pub struct PluginContext {
    pub db: SqlitePool,
    pub http_client: Client,
    pub plugin_data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub base_url: String,
}

/// Response returned by a plugin's generic request handler.
#[derive(Clone)]
pub struct PluginResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Plugin trait.
///
/// Note: `register_routes` takes `AppState` so that built-in plugins can
/// register Axum handlers using `State<AppState>`. The original framework
/// sketch omitted this parameter, but it is required for type-safe Axum
/// router construction without a dummy state placeholder.
#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> PluginManifest;
    fn source(&self) -> PluginSource;
    fn permissions(&self) -> Vec<PermissionDef> {
        Vec::new()
    }
    fn frontend_routes(&self) -> Vec<PluginRouteDef> {
        Vec::new()
    }
    async fn on_load(&self, _ctx: &PluginContext) -> Result<(), String> {
        Ok(())
    }
    async fn on_unload(&self, _ctx: &PluginContext) -> Result<(), String> {
        Ok(())
    }
    async fn handle_request(
        &self,
        _ctx: &PluginContext,
        _method: &str,
        _path: &str,
        _query: &str,
        _body: Option<&str>,
        _headers: Vec<(String, String)>,
    ) -> Result<PluginResponse, String> {
        Err("not implemented".into())
    }
    fn register_routes(&self, state: AppState) -> Router<AppState> {
        Router::new().with_state(state)
    }
    fn clone_box(&self) -> Box<dyn Plugin>;
}
