use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use tokio::sync::{mpsc, Mutex as TokioMutex, RwLock as TokioRwLock};

use crate::plugin::trait_::{
    PermissionDef, PluginManifest, PluginRouteDef, PluginSource,
};
use crate::state::AppState;

/// Internal storage for a loaded plugin.
struct PluginSlot {
    manifest: PluginManifest,
    source: PluginSource,
    routes: Vec<PluginRouteDef>,
    permissions: Vec<PermissionDef>,
}

/// Manages the lifecycle and routing of WASM plugins.
#[derive(Clone)]
pub struct PluginManager {
    loaded_plugins: Arc<TokioRwLock<Vec<PluginSlot>>>,
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
            loaded_plugins: Arc::new(TokioRwLock::new(Vec::new())),
            plugin_data: Arc::new(TokioRwLock::new(HashMap::new())),
            plugins_dir,
            hot_reload_tx: tx,
            hot_reload_rx: Arc::new(TokioMutex::new(Some(rx))),
        }
    }

    // ── Validation ──────────────────────────────────────────

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
        if matches!(id, "routes" | "reload" | "install" | "install-url") {
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

    fn validate_wasm_file(path: &Path) -> Result<(), String> {
        if path.extension().and_then(|s| s.to_str()) != Some("wasm") {
            return Err("File must have .wasm extension".into());
        }
        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        Self::validate_plugin_id(id)?;
        Ok(())
    }

    // ── WASM loading ────────────────────────────────────────

    /// Load a single WASM plugin from disk into memory.
    /// Parses the manifest from the WASM binary (via wasmtime) and registers
    /// the plugin slot so it appears in listings and hot-reload.
    async fn load_wasm_plugin(&self, wasm_path: &Path) -> Result<PluginSlot, String> {
        Self::validate_wasm_file(wasm_path)?;
        let id = wasm_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // Read the WASM binary
        let wasm_bytes = tokio::fs::read(wasm_path)
            .await
            .map_err(|e| format!("Failed to read WASM file '{}': {}", wasm_path.display(), e))?;

        // Attempt to parse the manifest from the WASM binary
        // The plugin guest exports `_plugin_manifest()` returning a JSON string
        let manifest = Self::extract_manifest(&wasm_bytes, &id).await?;

        // For now, WASM plugins contribute no frontend routes or Axum routes
        // (they use handle_request via a generic proxy endpoint)
        let plugin_routes = Vec::new();
        let permissions = Vec::new();

        Ok(PluginSlot {
            manifest,
            source: PluginSource::Wasm { path: wasm_path.to_path_buf() },
            routes: plugin_routes,
            permissions,
        })
    }

    /// Extract the plugin manifest by calling `_plugin_manifest()` in the WASM module.
    /// If the function is not exported, fall back to a basic manifest using the file stem.
    async fn extract_manifest(wasm_bytes: &[u8], fallback_id: &str) -> Result<PluginManifest, String> {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Invalid WASM module: {}", e))?;

        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[])
            .map_err(|e| format!("Failed to instantiate WASM module: {}", e))?;

        // Try to call _plugin_manifest() via the generic Func API
        if let Some(func) = instance.get_func(&mut store, "_plugin_manifest") {
            let mut results = vec![wasmtime::Val::I32(0)];
            match func.call(&mut store, &[], &mut results) {
                Ok(()) => {
                    if let wasmtime::Val::I32(ptr_val) = results[0] {
                        let memory = instance
                            .get_memory(&mut store, "memory")
                            .ok_or("WASM module has no exported memory")?;
                        let manifest_str = read_null_terminated_string(memory.data_mut(&mut store), ptr_val as usize);
                        let manifest: PluginManifest = serde_json::from_str(&manifest_str)
                            .map_err(|e| format!("Invalid plugin manifest JSON: {}", e))?;
                        return Ok(manifest);
                    }
                }
                Err(e) => {
                    tracing::warn!("_plugin_manifest() call failed: {}", e);
                }
            }
        }

        // Fallback: use file stem as id
        tracing::info!(
            "WASM plugin '{}' has no _plugin_manifest export; using fallback manifest",
            fallback_id
        );
        Ok(PluginManifest {
            id: fallback_id.to_string(),
            name: fallback_id.to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: None,
            homepage: None,
            permissions_needed: Vec::new(),
        })
    }

    // ── Install / Uninstall ─────────────────────────────────

    /// Install a WASM plugin from raw bytes (e.g., uploaded via API).
    /// The bytes are written to the plugins directory and the plugin is loaded.
    pub async fn install_plugin_from_bytes(
        &self,
        plugin_id: &str,
        wasm_bytes: &[u8],
    ) -> Result<PluginManifest, String> {
        Self::validate_plugin_id(plugin_id)?;

        let dest = self.plugins_dir.join(format!("{}.wasm", plugin_id));
        if dest.exists() {
            return Err(format!("Plugin '{}' is already installed", plugin_id));
        }

        // Validate by attempting to extract manifest first
        let manifest = Self::extract_manifest(wasm_bytes, plugin_id).await?;

        // Write the WASM file
        tokio::fs::write(&dest, wasm_bytes)
            .await
            .map_err(|e| format!("Failed to write plugin file: {}", e))?;

        // Load into memory
        let slot = self.load_wasm_plugin(&dest).await?;
        self.loaded_plugins.write().await.push(slot);

        tracing::info!("Installed WASM plugin: {} ({})", manifest.id, manifest.version);
        Ok(manifest)
    }

    /// Install a WASM plugin by downloading from a URL.
    pub async fn install_plugin_from_url(
        &self,
        plugin_id: &str,
        url: &str,
    ) -> Result<PluginManifest, String> {
        let resp = reqwest::get(url)
            .await
            .map_err(|e| format!("Failed to download from URL '{}': {}", url, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Download failed with HTTP {} from '{}'",
                resp.status(),
                url
            ));
        }

        let bytes = resp.bytes().await.map_err(|e| format!("Failed to read response body: {}", e))?;
        self.install_plugin_from_bytes(plugin_id, &bytes).await
    }

    /// Uninstall (delete) a WASM plugin by id.
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let mut slots = self.loaded_plugins.write().await;
        if let Some(pos) = slots.iter().position(|s| s.manifest.id == plugin_id) {
            // Check that it's a WASM plugin (we only support uninstalling WASM)
            match &slots[pos].source {
                PluginSource::Wasm { path } => {
                    tokio::fs::remove_file(path)
                        .await
                        .map_err(|e| format!("Failed to delete plugin file: {}", e))?;
                    slots.remove(pos);
                    tracing::info!("Uninstalled WASM plugin: {}", plugin_id);
                    Ok(())
                }
                _ => Err(format!("Plugin '{}' cannot be uninstalled (not a WASM plugin)", plugin_id)),
            }
        } else {
            Err(format!("Plugin '{}' is not installed", plugin_id))
        }
    }

    // ── Scanning & Hot-reload ───────────────────────────────

    /// Scan the plugins directory for `.wasm` files and load any new ones.
    /// Returns counts of newly-loaded plugins.
    pub async fn scan_wasm_plugins(&self) -> Result<usize, String> {
        if !self.plugins_dir.exists() {
            return Ok(0);
        }

        let mut dir = tokio::fs::read_dir(&self.plugins_dir)
            .await
            .map_err(|e| e.to_string())?;

        let mut loaded = 0usize;
        let mut ids_to_load: Vec<(String, PathBuf)> = Vec::new();

        loop {
            match dir.next_entry().await {
                Ok(None) => break,
                Ok(Some(entry)) => {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                        ids_to_load.push((id, path));
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        let mut slots = self.loaded_plugins.read().await;
        for (id, path) in &ids_to_load {
            if !slots.iter().any(|s| s.manifest.id == *id) {
                drop(slots);
                match self.load_wasm_plugin(path).await {
                    Ok(slot) => {
                        self.loaded_plugins.write().await.push(slot);
                        loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load WASM plugin '{}': {}", id, e);
                    }
                }
                slots = self.loaded_plugins.read().await;
            }
        }

        Ok(loaded)
    }

    /// Hot-reload: scan for new/removed WASM plugins.
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
                        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                        current_files.insert(id, path);
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        // Remove plugins whose files have disappeared
        let mut slots = self.loaded_plugins.write().await;
        let mut plugins_removed = 0usize;
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

        for id in &removed_ids {
            if let Some(pos) = slots.iter().position(|s| s.manifest.id == *id) {
                slots.remove(pos);
                tracing::info!("Unloaded removed WASM plugin: {}", id);
                plugins_removed += 1;
            }
        }

        // Load new plugins
        let mut plugins_loaded = 0usize;
        for (id, path) in &current_files {
            if !slots.iter().any(|s| s.manifest.id == *id) {
                drop(slots);
                match self.load_wasm_plugin(path).await {
                    Ok(slot) => {
                        self.loaded_plugins.write().await.push(slot);
                        plugins_loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load WASM plugin '{}': {}", id, e);
                    }
                }
                slots = self.loaded_plugins.write().await;
            }
        }

        Ok((plugins_loaded, plugins_removed))
    }

    // ── Query methods ───────────────────────────────────────

    pub async fn get_manifests(&self) -> Vec<PluginManifest> {
        let slots = self.loaded_plugins.read().await;
        slots.iter().map(|s| s.manifest.clone()).collect()
    }

    pub fn plugin_permissions(&self) -> Vec<(String, Vec<PermissionDef>)> {
        let slots = self.loaded_plugins.blocking_read();
        slots
            .iter()
            .map(|s| (s.manifest.id.clone(), s.permissions.clone()))
            .collect()
    }

    pub fn trigger_hot_reload(&self) -> Result<(), mpsc::error::TrySendError<()>> {
        self.hot_reload_tx.try_send(())
    }

    pub async fn get_frontend_routes(&self) -> Vec<(String, Vec<PluginRouteDef>)> {
        let slots = self.loaded_plugins.read().await;
        slots
            .iter()
            .map(|s| (s.manifest.id.clone(), s.routes.clone()))
            .collect()
    }

    /// Build an Axum router containing all built-in plugin routes
    /// nested under `/{plugin_id}`.
    /// WASM plugins currently do not register Axum routes; they use
    /// the generic handle_request proxy instead.
    pub fn build_plugin_router(&self, _state: AppState) -> Router<AppState> {
        Router::new()
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

/// Read a null-terminated UTF-8 string from WASM linear memory at the given offset.
fn read_null_terminated_string(memory: &[u8], offset: usize) -> String {
    let mut end = offset;
    while end < memory.len() && memory[end] != 0 {
        end += 1;
    }
    std::str::from_utf8(&memory[offset..end])
        .unwrap_or("")
        .to_string()
}
