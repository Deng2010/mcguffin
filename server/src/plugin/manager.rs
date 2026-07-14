use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock as StdRwLock};

use axum::Router;
use tokio::sync::{mpsc, Mutex as TokioMutex, RwLock as TokioRwLock};

use crate::plugin::trait_::{PermissionDef, Plugin, PluginContext, PluginManifest, PluginRouteDef, PluginSource};
use crate::state::AppState;

/// Internal storage for a loaded plugin.
struct PluginSlot {
    manifest: PluginManifest,
    source: PluginSource,
    routes: Vec<PluginRouteDef>,
    permissions: Vec<PermissionDef>,
}

/// Manages the lifecycle and routing of plugins.
#[derive(Clone)]
pub struct PluginManager {
    loaded_plugins: Arc<StdRwLock<Vec<PluginSlot>>>,
    pub plugin_data: Arc<TokioRwLock<HashMap<String, serde_json::Value>>>,
    plugins_dir: PathBuf,
    hot_reload_tx: mpsc::Sender<()>,
    hot_reload_rx: Arc<TokioMutex<Option<mpsc::Receiver<()>>>>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&plugins_dir) {
            tracing::warn!("Failed to create plugins directory {}: {}", plugins_dir.display(), e);
        }
        let (tx, rx) = mpsc::channel(8);
        Self {
            loaded_plugins: Arc::new(StdRwLock::new(Vec::new())),
            plugin_data: Arc::new(TokioRwLock::new(HashMap::new())),
            plugins_dir,
            hot_reload_tx: tx,
            hot_reload_rx: Arc::new(TokioMutex::new(Some(rx))),
        }
    }

    /// Register a built-in plugin and run its load hook.
    pub async fn register_builtin(
        &self,
        plugin: Box<dyn Plugin>,
        ctx: PluginContext,
    ) -> Result<(), String> {
        let manifest = plugin.manifest();
        Self::validate_plugin_id(&manifest.id)?;
        for route in plugin.frontend_routes() {
            Self::validate_route_path(&route.path)?;
        }

        plugin
            .on_load(&ctx)
            .await
            .map_err(|e| format!("Plugin on_load failed: {}", e))?;
        let routes = plugin.frontend_routes();
        let permissions = plugin.permissions();
        let slot = PluginSlot {
            manifest,
            source: PluginSource::BuiltIn(plugin),
            routes,
            permissions,
        };
        self.loaded_plugins.write().unwrap().push(slot);
        Ok(())
    }

    fn validate_plugin_id(id: &str) -> Result<(), String> {
        if id.is_empty() {
            return Err("Plugin id must not be empty".into());
        }
        if id.len() > 64 {
            return Err("Plugin id must be 64 characters or fewer".into());
        }
        if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(format!(
                "Plugin id '{}' contains invalid characters; allowed: a-z, 0-9, '-'",
                id
            ));
        }
        if matches!(id, "routes" | "reload") {
            return Err(format!("Plugin id '{}' is reserved", id));
        }
        Ok(())
    }

    fn validate_route_path(path: &str) -> Result<(), String> {
        if !path.starts_with("/plugins/") {
            return Err(format!(
                "Plugin route path '{}' must start with '/plugins/'",
                path
            ));
        }
        if path.contains("..") || path.contains("//") {
            return Err(format!("Plugin route path '{}' contains invalid segments", path));
        }
        Ok(())
    }

    /// Scan the plugins directory for `.wasm` files and log their presence.
    /// Actual WASM loading is not yet implemented.
    pub fn scan_wasm_plugins(&self) -> Result<(), String> {
        if !self.plugins_dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(&self.plugins_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                tracing::info!(
                    "WASM plugin loading not yet implemented — found file: {}",
                    path.display()
                );
            }
        }
        Ok(())
    }

    /// Rescan the plugins directory, detect new/removed WASM plugins, and
    /// return the counts of newly-discovered and removed plugins.
    pub async fn hot_reload(&self) -> Result<(usize, usize), String> {
        if !self.plugins_dir.exists() {
            return Ok((0, 0));
        }

        let mut dir = tokio::fs::read_dir(&self.plugins_dir)
            .await
            .map_err(|e| e.to_string())?;
        let mut current_files: HashMap<String, PathBuf> = HashMap::new();
        loop {
            match dir.next_entry().await {
                Ok(None) => break,
                Ok(Some(entry)) => {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        let id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        current_files.insert(id, path);
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        let mut slots = self.loaded_plugins.write().unwrap();

        let removed_ids: Vec<String> = slots
            .iter()
            .filter_map(|s| {
                if matches!(s.source, PluginSource::Wasm { .. })
                    && !current_files.contains_key(&s.manifest.id)
                {
                    Some(s.manifest.id.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut plugins_removed = 0usize;
        for id in removed_ids {
            if let Some(pos) = slots.iter().position(|s| s.manifest.id == id) {
                slots.remove(pos);
                tracing::info!("Unloaded removed WASM plugin: {}", id);
                plugins_removed += 1;
            }
        }

        let mut plugins_loaded = 0usize;
        for (id, path) in current_files {
            if !slots.iter().any(|s| s.manifest.id == id) {
                tracing::info!(
                    "WASM plugin loading not yet implemented — found file: {}",
                    path.display()
                );
                plugins_loaded += 1;
            }
        }

        Ok((plugins_loaded, plugins_removed))
    }

    pub async fn get_manifests(&self) -> Vec<PluginManifest> {
        let slots = self.loaded_plugins.read().unwrap();
        slots.iter().map(|s| s.manifest.clone()).collect()
    }

    pub fn plugin_permissions(&self) -> Vec<(String, Vec<PermissionDef>)> {
        let slots = self.loaded_plugins.read().unwrap();
        slots
            .iter()
            .map(|s| (s.manifest.id.clone(), s.permissions.clone()))
            .collect()
    }

    pub fn trigger_hot_reload(&self) -> Result<(), mpsc::error::TrySendError<()>> {
        self.hot_reload_tx.try_send(())
    }

    /// Return frontend routes grouped by plugin id.
    pub async fn get_frontend_routes(&self) -> Vec<(String, Vec<PluginRouteDef>)> {
        let slots = self.loaded_plugins.read().unwrap();
        slots
            .iter()
            .map(|s| (s.manifest.id.clone(), s.routes.clone()))
            .collect()
    }

    /// Build an Axum router containing all built-in plugin routes nested
    /// under `/{plugin_id}`.
    ///
    /// `AppState` is passed explicitly because Axum routers must know their
    /// state type at construction time.
    pub fn build_plugin_router(&self, state: AppState) -> Router<AppState> {
        let slots = self.loaded_plugins.read().unwrap();
        let mut router = Router::new();
        for slot in slots.iter() {
            if let PluginSource::BuiltIn(plugin) = &slot.source {
                let plugin_router = plugin.register_routes(state.clone());
                let nested =
                    Router::new().nest(&format!("/plugins/{}", slot.manifest.id), plugin_router);
                router = router.merge(nested);
            }
        }
        router
    }

    /// Start a background task that reloads plugins when signaled.
    pub async fn start_hot_reload_task(&self) {
        let rx = self.hot_reload_rx.lock().await.take();
        let manager = self.clone();
        if let Some(mut rx) = rx {
            tokio::spawn(async move {
                while rx.recv().await.is_some() {
                    if let Err(e) = manager.hot_reload().await {
                        tracing::warn!("Plugin hot reload failed: {}", e);
                    }
                }
            });
        }
    }
}
