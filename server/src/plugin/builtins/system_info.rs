use std::sync::OnceLock;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::Value;

use crate::plugin::trait_::{NavPlacement, Plugin, PluginContext, PluginManifest, PluginRouteDef, PluginSource};
use crate::state::AppState;
use crate::types::perms;
use crate::utils::AuthUser;

/// Built-in plugin exposing basic system statistics.
#[derive(Clone)]
pub struct SystemInfoPlugin;

static START_TIME: OnceLock<Instant> = OnceLock::new();

#[async_trait::async_trait]
impl Plugin for SystemInfoPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "system-info".to_string(),
            name: "系统信息".to_string(),
            version: "0.3.0".to_string(),
            description: "System statistics".to_string(),
            author: None,
            homepage: None,
            permissions_needed: Vec::new(),
        }
    }

    fn source(&self) -> PluginSource {
        PluginSource::BuiltIn(Box::new(self.clone()))
    }

    fn frontend_routes(&self) -> Vec<PluginRouteDef> {
        vec![PluginRouteDef {
            path: "/plugins/system-info".to_string(),
            label: "系统信息".to_string(),
            icon: Some("📊".to_string()),
            required_permission: None,
            nav_placement: NavPlacement::Main,
        }]
    }

    async fn on_load(&self, _ctx: &PluginContext) -> Result<(), String> {
        let _ = START_TIME.set(Instant::now());
        Ok(())
    }

    fn register_routes(&self, state: AppState) -> Router<AppState> {
        Router::new()
            .route("/stats", get(get_system_stats))
            .with_state(state)
    }

    fn clone_box(&self) -> Box<dyn Plugin> {
        Box::new(self.clone())
    }
}

async fn get_system_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::VIEW_SHOWCASE).await?;
    let users = state.users.lock().await.len();
    let problems = state.problems.read().await.len();
    let contests = state.contests.read().await.len();
    let posts = state.posts.read().await.len();
    let uptime_secs = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);

    Ok(Json(serde_json::json!({
        "users": users,
        "problems": problems,
        "contests": contests,
        "posts": posts,
        "version": state.site_version,
        "uptime_secs": uptime_secs,
    })))
}
