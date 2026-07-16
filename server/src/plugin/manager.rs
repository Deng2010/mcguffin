use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock as TokioRwLock;

use crate::plugin::trait_::{PermissionDef, PluginManifest, PluginRouteDef};

/// Internal storage for a loaded plugin.
#[derive(Clone)]
struct PluginSlot {
    manifest: PluginManifest,
    routes: Vec<PluginRouteDef>,
    permissions: Vec<PermissionDef>,
    /// Where this plugin's files are stored on disk (if uploaded).
    source_dir: Option<PathBuf>,
}

/// Manages plugin listing, routes, permissions, and installation.
///
/// In the React-plugin model, plugins can be:
/// - **Local**: frontend React components registered via `definePlugin()`
/// - **Uploaded**: .zip archives uploaded via admin, stored on disk
#[derive(Clone)]
pub struct PluginManager {
    loaded_plugins: Arc<TokioRwLock<Vec<PluginSlot>>>,
    /// Directory where uploaded plugins are stored.
    pub plugins_dir: PathBuf,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&plugins_dir);
        Self {
            loaded_plugins: Arc::new(TokioRwLock::new(Vec::new())),
            plugins_dir,
        }
    }

    // ── Registration ────────────────────────────────────────

    /// Register a plugin (from frontend `definePlugin()` or backend zip install).
    pub async fn register(
        &self,
        manifest: PluginManifest,
        routes: Vec<PluginRouteDef>,
        permissions: Vec<PermissionDef>,
    ) {
        let mut slots = self.loaded_plugins.write().await;
        if let Some(pos) = slots.iter().position(|s| s.manifest.id == manifest.id) {
            slots[pos] = PluginSlot { manifest, routes, permissions, source_dir: None };
        } else {
            slots.push(PluginSlot { manifest, routes, permissions, source_dir: None });
        }
    }

    /// Register a plugin with a source directory (from zip install).
    async fn register_with_source(
        &self,
        manifest: PluginManifest,
        routes: Vec<PluginRouteDef>,
        permissions: Vec<PermissionDef>,
        source_dir: PathBuf,
    ) {
        let mut slots = self.loaded_plugins.write().await;
        if let Some(pos) = slots.iter().position(|s| s.manifest.id == manifest.id) {
            slots[pos] = PluginSlot {
                manifest, routes, permissions, source_dir: Some(source_dir),
            };
        } else {
            slots.push(PluginSlot {
                manifest, routes, permissions, source_dir: Some(source_dir),
            });
        }
    }

    /// Remove a plugin by id.
    pub async fn unregister(&self, plugin_id: &str) {
        let mut slots = self.loaded_plugins.write().await;
        // Remove source files if this was an uploaded plugin
        if let Some(pos) = slots.iter().position(|s| s.manifest.id == plugin_id) {
            if let Some(dir) = &slots[pos].source_dir {
                let _ = std::fs::remove_dir_all(dir);
            }
            slots.remove(pos);
        }
    }

    // ── Install from Zip ────────────────────────────────────

    /// Install a plugin from a .zip archive.
    ///
    /// The zip must contain a `plugin.json` at the root, which includes the
    /// plugin's `id`. The archive is extracted to `{plugins_dir}/{plugin_id}/`.
    pub async fn install_from_zip(&self, zip_bytes: &[u8]) -> Result<PluginManifest, String> {
        // Parse zip
        let reader = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| format!("无效的 ZIP 文件: {}", e))?;

        // Read plugin.json from zip without extracting
        let manifest_content = {
            let mut found: Option<String> = None;
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|_| "读取 ZIP 条目失败".to_string())?;
                if entry.name() == "plugin.json" {
                    let mut content = String::new();
                    std::io::Read::read_to_string(&mut entry, &mut content)
                        .map_err(|_| "读取 plugin.json 失败".to_string())?;
                    found = Some(content);
                    break;
                }
            }
            found.ok_or_else(|| "ZIP 文件根目录必须包含 plugin.json".to_string())?
        };

        // Parse plugin.json to get plugin id
        #[derive(serde::Deserialize)]
        struct PluginJson {
            id: String,
            name: String,
            version: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            author: Option<String>,
            #[serde(default)]
            homepage: Option<String>,
            #[serde(default)]
            permissions_needed: Vec<String>,
            #[serde(default)]
            routes: Vec<PluginRouteDef>,
            #[serde(default)]
            permissions: Vec<PermissionDef>,
        }

        let parsed: PluginJson = serde_json::from_str(&manifest_content)
            .map_err(|e| format!("解析 plugin.json 失败: {}", e))?;

        let plugin_id = &parsed.id;
        Self::validate_plugin_id(plugin_id)?;

        // Re-create archive (was consumed by read above)
        let reader = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| format!("无效的 ZIP 文件: {}", e))?;

        let target_dir = self.plugins_dir.join(plugin_id);
        if target_dir.exists() {
            return Err(format!("插件 '{}' 已安装", plugin_id));
        }

        // Extract all files
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("创建插件目录失败: {}", e))?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
            let name = entry.name().to_string();

            if name.ends_with('/') {
                std::fs::create_dir_all(target_dir.join(&name))
                    .map_err(|e| format!("创建目录 {} 失败: {}", name, e))?;
                continue;
            }

            if let Some(parent) = Path::new(&name).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(target_dir.join(parent))
                        .map_err(|e| format!("创建目录 {} 失败: {}", parent.display(), e))?;
                }
            }

            let mut out = std::fs::File::create(target_dir.join(&name))
                .map_err(|e| format!("创建文件 {} 失败: {}", name, e))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("写入文件 {} 失败: {}", name, e))?;
        }

        let manifest = PluginManifest {
            id: plugin_id.to_string(),
            name: parsed.name,
            version: parsed.version,
            description: parsed.description,
            author: parsed.author,
            homepage: parsed.homepage,
            permissions_needed: parsed.permissions_needed,
        };

        self.register_with_source(
            manifest.clone(),
            parsed.routes,
            parsed.permissions,
            target_dir,
        ).await;

        Ok(manifest)
    }

    /// Get the source directory for a plugin (for serving static assets).
    pub async fn get_plugin_dir(&self, plugin_id: &str) -> Option<PathBuf> {
        let slots = self.loaded_plugins.read().await;
        slots.iter()
            .find(|s| s.manifest.id == plugin_id)
            .and_then(|s| s.source_dir.clone())
    }

    /// Rescan the plugins directory for installed plugins and reload metadata.
    pub async fn rescan_installed(&self) -> Result<usize, String> {
        if !self.plugins_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut dir = tokio::fs::read_dir(&self.plugins_dir).await
            .map_err(|e| format!("读取插件目录失败: {}", e))?;

        let mut entries = Vec::new();
        while let Ok(Some(entry)) = dir.next_entry().await {
            let path = entry.path();
            if path.is_dir() && path.join("plugin.json").exists() {
                let id = path.file_stem().and_then(|s| s.to_str())
                    .unwrap_or("unknown").to_string();
                entries.push((id, path));
            }
        }

        for (id, path) in &entries {
            let manifest_path = path.join("plugin.json");
            let content = tokio::fs::read_to_string(&manifest_path).await
                .map_err(|e| format!("读取 {} 失败: {}", manifest_path.display(), e))?;

            #[derive(serde::Deserialize)]
            struct PluginJson {
                #[serde(default)]
                id: Option<String>,
                name: String,
                version: String,
                #[serde(default)]
                description: String,
                #[serde(default)]
                author: Option<String>,
                #[serde(default)]
                permissions_needed: Vec<String>,
                #[serde(default)]
                routes: Vec<PluginRouteDef>,
                #[serde(default)]
                permissions: Vec<PermissionDef>,
            }

            if let Ok(parsed) = serde_json::from_str::<PluginJson>(&content) {
                let actual_id = parsed.id.as_deref().unwrap_or(id);
                if !self.has_plugin_blocking(actual_id) {
                    let manifest = PluginManifest {
                        id: actual_id.to_string(),
                        name: parsed.name,
                        version: parsed.version,
                        description: parsed.description,
                        author: parsed.author,
                        homepage: None,
                        permissions_needed: parsed.permissions_needed,
                    };
                    self.loaded_plugins.write().await.push(PluginSlot {
                        manifest,
                        routes: parsed.routes,
                        permissions: parsed.permissions,
                        source_dir: Some(path.clone()),
                    });
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    fn has_plugin_blocking(&self, plugin_id: &str) -> bool {
        self.loaded_plugins.blocking_read().iter().any(|s| s.manifest.id == plugin_id)
    }

    // ── Query methods ───────────────────────────────────────

    pub async fn get_manifests(&self) -> Vec<PluginManifest> {
        let slots = self.loaded_plugins.read().await;
        slots.iter().map(|s| s.manifest.clone()).collect()
    }

    pub async fn get_frontend_routes(&self) -> Vec<(String, Vec<PluginRouteDef>)> {
        let slots = self.loaded_plugins.read().await;
        slots
            .iter()
            .map(|s| (s.manifest.id.clone(), s.routes.clone()))
            .collect()
    }

    pub fn plugin_permissions(&self) -> Vec<(String, Vec<PermissionDef>)> {
        let slots = self.loaded_plugins.blocking_read();
        slots
            .iter()
            .map(|s| (s.manifest.id.clone(), s.permissions.clone()))
            .collect()
    }

    pub fn all_plugin_permission_keys(&self) -> Vec<String> {
        let slots = self.loaded_plugins.blocking_read();
        let mut keys: Vec<String> = slots
            .iter()
            .flat_map(|s| s.permissions.iter().map(|p| p.key.clone()))
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub async fn has_plugin(&self, plugin_id: &str) -> bool {
        self.loaded_plugins.read().await.iter().any(|s| s.manifest.id == plugin_id)
    }

    /// Validate a plugin ID string.
    pub fn validate_plugin_id(id: &str) -> Result<(), String> {
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
        Ok(())
    }
}
