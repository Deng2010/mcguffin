// ============== Plugin API handlers ==============
//
// Endpoints:
//   POST   /api/plugins/register              — register/refresh plugin metadata (zip plugin entries)
//   GET    /api/plugins                        — public list (id/name/version/enabled/source)
//   GET    /api/admin/plugins                  — list registered plugins
//   DELETE /api/admin/plugins/{plugin_id}      — unregister plugin & delete data
//   POST   /api/admin/plugins/{plugin_id}/enable   — enable a plugin
//   POST   /api/admin/plugins/{plugin_id}/disable  — disable a plugin
//   GET    /api/admin/plugins/global           — global plugin feature switch
//   POST   /api/admin/plugins/global           — set global plugin feature switch
//   POST   /api/admin/plugins/install-zip      — install a plugin from a .zip package
//   GET    /api/plugins/{plugin_id}/assets/{*} — static assets of zip-installed plugins (public)
//   GET    /api/plugins/{plugin_id}/users      — list team members (needs read:team)
//   GET    /api/plugins/{plugin_id}/users/me   — current user info
//   GET    /api/plugins/{plugin_id}/users/{id} — user info (needs read:users)
//   GET    /api/plugins/{plugin_id}/data       — read KV (needs storage)
//   POST   /api/plugins/{plugin_id}/data       — write KV (needs storage)
//   POST   /api/plugins/{plugin_id}/data/add   — atomic counter add (needs storage)
//   POST   /api/plugins/{plugin_id}/data/set-add    — set add (needs storage)
//   POST   /api/plugins/{plugin_id}/data/set-remove — set remove (needs storage)
//   GET    /api/plugins/{plugin_id}/data/set-members   — set members (needs storage)
//   GET    /api/plugins/{plugin_id}/data/set-is-member — set membership (needs storage)
//   GET    /api/plugins/{plugin_id}/data/keys  — list keys in namespace (needs storage)
//   POST   /api/plugins/{plugin_id}/files/{*}  — write file (needs storage)
//   GET    /api/plugins/{plugin_id}/files/{*}  — read file (needs storage)
//   DELETE /api/plugins/{plugin_id}/files/{*}  — delete file (needs storage)
//   GET    /api/plugins/{plugin_id}/files/list — list files (needs storage)
//   POST   /api/plugins/{plugin_id}/notify     — send notification (needs notify)
//
// Permission model:
//   - Each plugin declares requested permissions in its manifest.
//   - At registration time, all requested permissions are granted (trust model;
//     admin review UI can be added later).
//   - Data APIs check the stored plugin permissions before serving.
//   - Disabled plugins are rejected from all API endpoints (except listing).
//
// Persistence:
//   - Manifests, KV data and the global switch are written through to SQLite
//     (tables `plugins` / `plugin_data`, meta key `plugins_disabled`).
//   - Files are stored on disk under `<data_dir>/plugins/{plugin_id}/files/`;
//     zip-installed plugin assets live in `<data_dir>/plugins/{plugin_id}/assets/`.

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use std::collections::HashMap;

use crate::domain::plugin::{
    plugin_perms, AddDataPayload, NotifyPayload, PluginManifest, PluginRegistration,
    PluginTogglePayload, SetDataPayload, SetMemberPayload, SetPermissionsPayload,
};
use crate::error::{json_error, ErrorCode};
use crate::state::AppState;
use crate::types::{AuditEntry, Notification, PERM_WILDCARD};
use crate::utils::AuthUser;

// ── Limits ──

/// 单个插件文件 / zip 内单文件上限（8 MiB）
const MAX_PLUGIN_FILE_SIZE: u64 = 8 * 1024 * 1024;
/// zip 插件包：最大文件数
const MAX_ZIP_FILES: usize = 512;
/// zip 插件包：解压后总大小上限（64 MiB）
const MAX_ZIP_TOTAL_UNCOMPRESSED: u64 = 64 * 1024 * 1024;
/// 从 URL 下载的插件包体积上限（64 MiB，与解压后总大小上限对齐）
const MAX_PLUGIN_ZIP_BYTES: u64 = 64 * 1024 * 1024;
/// 文件列表接口最多返回的条目数
const MAX_FILE_LIST: usize = 1000;
/// 单个 KV 值（含集合序列化结果）上限
const MAX_KV_VALUE_BYTES: usize = 64 * 1024;
/// 单插件 KV 条目总数上限（跨命名空间）
const MAX_KV_KEYS_PER_PLUGIN: usize = 2000;
/// 命名空间 / key / 集合成员的长度上限
const MAX_KV_NAMESPACE_LEN: usize = 64;
const MAX_KV_KEY_LEN: usize = 256;
const MAX_KV_MEMBER_LEN: usize = 256;

// ── Helpers ──

/// Check if a plugin has a specific permission.
fn plugin_has_perm(plugin: &PluginManifest, perm: &str) -> bool {
    plugin.permissions.iter().any(|p| p == perm)
}

/// Check if a plugin is enabled, returning 403 if disabled.
fn require_plugin_enabled(
    plugin: &PluginManifest,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !plugin.enabled {
        return Err((
            StatusCode::FORBIDDEN,
            json_error(ErrorCode::PLUGIN_DISABLED, "插件已被禁用，请联系管理员启用"),
        ));
    }
    Ok(())
}

/// Check whether the plugin feature is globally disabled, returning 403 if so.
async fn require_plugins_globally_enabled(
    state: &AppState,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if *state.plugins_disabled.read().await {
        return Err((
            StatusCode::FORBIDDEN,
            json_error(ErrorCode::PLUGIN_DISABLED, "插件功能已被管理员全局禁用"),
        ));
    }
    Ok(())
}

/// Load a plugin by id, enforcing the global switch, existence and enabled state.
async fn load_enabled_plugin(
    state: &AppState,
    plugin_id: &str,
) -> Result<PluginManifest, (StatusCode, Json<serde_json::Value>)> {
    require_plugins_globally_enabled(state).await?;
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(plugin_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                json_error(ErrorCode::PLUGIN_NOT_FOUND, "插件未注册"),
            )
        })?
        .clone();
    drop(plugins);
    require_plugin_enabled(&plugin)?;
    Ok(plugin)
}

/// Require a plugin permission, returning 403 if not granted.
macro_rules! require_plugin_perm {
    ($plugin:expr, $perm:expr) => {
        if !plugin_has_perm($plugin, $perm) {
            return Err((
                StatusCode::FORBIDDEN,
                json_error(
                    ErrorCode::PLUGIN_PERMISSION_DENIED,
                    format!("插件未申请权限: {}", $perm),
                ),
            ));
        }
    };
}

type ApiErr = (StatusCode, Json<serde_json::Value>);

/// 校验插件权限；`granted` 为假时返回 403。
/// 精确权限用 `plugin_has_perm`，蕴含权限（如 write:team ⇒ read:team）传对应 helper 的结果。
fn ensure_plugin_perm(perm: &str, granted: bool) -> Result<(), ApiErr> {
    if granted {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            json_error(
                ErrorCode::PLUGIN_PERMISSION_DENIED,
                format!("插件未申请权限: {}", perm),
            ),
        ))
    }
}

fn bad_request(code: ErrorCode, msg: impl Into<String>) -> ApiErr {
    (StatusCode::BAD_REQUEST, json_error(code, msg.into()))
}

fn not_found(msg: &str) -> ApiErr {
    (
        StatusCode::NOT_FOUND,
        json_error(ErrorCode::PLUGIN_NOT_FOUND, msg),
    )
}

// ── Filesystem helpers ──

/// 数据目录（数据库文件所在目录；`:memory:` 回退时仍为配置路径）。
fn data_dir(state: &AppState) -> std::path::PathBuf {
    std::path::Path::new(&state.db_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn plugin_root_dir(state: &AppState, plugin_id: &str) -> std::path::PathBuf {
    data_dir(state).join("plugins").join(plugin_id)
}

fn plugin_assets_dir(state: &AppState, plugin_id: &str) -> std::path::PathBuf {
    plugin_root_dir(state, plugin_id).join("assets")
}

fn plugin_files_dir(state: &AppState, plugin_id: &str) -> std::path::PathBuf {
    plugin_root_dir(state, plugin_id).join("files")
}

/// 清洗相对路径：拒绝绝对路径与 `..` 穿越，归一化分隔符。
/// 返回 None 表示路径不合法。
fn sanitize_rel_path(p: &str) -> Option<String> {
    let p = p.replace('\\', "/");
    if p.is_empty() || p.starts_with('/') || p.contains('\0') {
        return None;
    }
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => continue,
            ".." => return None,
            s => out.push(s),
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out.join("/"))
    }
}

fn is_valid_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .next()
            .map(|c| c.is_ascii_alphanumeric())
            .unwrap_or(false)
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// 读取 KV 中的 JSON 集合（字符串数组），容错为空集合。
fn read_json_set(raw: Option<&String>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

// ── KV 配额校验 ──

/// 校验 namespace / key 是否为空及长度是否超限。
fn validate_kv_target(namespace: &str, key: &str) -> Result<(), ApiErr> {
    if namespace.is_empty() || key.is_empty() {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            "namespace 和 key 不能为空",
        ));
    }
    if namespace.len() > MAX_KV_NAMESPACE_LEN {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            format!("namespace 长度不能超过 {} 字符", MAX_KV_NAMESPACE_LEN),
        ));
    }
    if key.len() > MAX_KV_KEY_LEN {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            format!("key 长度不能超过 {} 字符", MAX_KV_KEY_LEN),
        ));
    }
    Ok(())
}

/// 单值大小校验（KV 值 / 集合序列化结果）。
fn validate_kv_value(value: &str) -> Result<(), ApiErr> {
    if value.len() > MAX_KV_VALUE_BYTES {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            format!("单个值不能超过 {} KiB", MAX_KV_VALUE_BYTES / 1024),
        ));
    }
    Ok(())
}

/// 集合成员校验。
fn validate_kv_member(member: &str) -> Result<(), ApiErr> {
    if member.is_empty() {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            "member 不能为空",
        ));
    }
    if member.len() > MAX_KV_MEMBER_LEN {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            format!("member 长度不能超过 {} 字符", MAX_KV_MEMBER_LEN),
        ));
    }
    Ok(())
}

/// 写入前的条目数配额检查：新增 key 时不允许超过单插件 KV 条目上限
/// （更新已有 key 永远允许）。
fn ensure_kv_quota(
    data: &crate::state::PluginDataStore,
    plugin_id: &str,
    namespace: &str,
    key: &str,
) -> Result<(), ApiErr> {
    let plugin_ns = data.get(plugin_id);
    let is_new = !plugin_ns
        .and_then(|ns| ns.get(namespace))
        .map(|kv| kv.contains_key(key))
        .unwrap_or(false);
    if !is_new {
        return Ok(());
    }
    let total: usize = plugin_ns
        .map(|ns| ns.values().map(|kv| kv.len()).sum())
        .unwrap_or(0);
    if total >= MAX_KV_KEYS_PER_PLUGIN {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            format!(
                "插件 KV 条目数已达上限 {}，请先清理不再使用的 key",
                MAX_KV_KEYS_PER_PLUGIN
            ),
        ));
    }
    Ok(())
}

// ── 审计 ──

/// 记录插件生命周期操作的审计日志（result = "allow"，调用方在操作成功后调用）。
async fn audit_plugin(
    state: &AppState,
    auth: &AuthUser,
    action: &str,
    plugin_id: &str,
    detail: &str,
) {
    state
        .log_audit(AuditEntry {
            timestamp: chrono::Utc::now(),
            user_id: auth.user_id.clone(),
            user_name: auth.user.display_name.clone(),
            action: action.to_string(),
            resource: format!("plugin:{}", plugin_id),
            result: "allow".to_string(),
            reason: detail.to_string(),
        })
        .await;
}

// ── Register ──

/// POST /api/plugins/register
/// Called by the frontend PluginRegistry when a ZIP plugin's entry module
/// self-registers on load. Idempotent: re-registering refreshes metadata but
/// keeps `enabled` / `source` / `entry` / permissions unchanged (权限冻结，
/// 防止无鉴权的重注册被用来顶替 id 并扩权)。
pub async fn register_plugin(
    State(state): State<AppState>,
    Json(payload): Json<PluginRegistration>,
) -> Json<serde_json::Value> {
    // Validate permissions: only allow known permission strings
    let requested_perms: Vec<String> = payload
        .permissions
        .iter()
        .filter(|p| plugin_perms::ALL.contains(&p.as_str()))
        .cloned()
        .collect();

    // 已注册插件：保留 enabled / source / entry / source_url，并**冻结权限清单**。
    // 注册接口无需鉴权（zip 插件入口在页面加载时自注册），若允许重注册改权限，
    // 任何人都能顶掉某个插件 id 并扩权。权限变更只能走
    // PUT /api/admin/plugins/{id}/permissions（superadmin）。
    let (existing_enabled, existing_source, existing_entry, existing_source_url, existing_perms) = {
        let plugins = state.plugins.read().await;
        plugins
            .get(&payload.id)
            .map(|p| {
                (
                    p.enabled,
                    p.source.clone(),
                    p.entry.clone(),
                    p.source_url.clone(),
                    Some(p.permissions.clone()),
                )
            })
            // 未安装过的 id：沿用历史默认来源 "code"（无 zip 资产，前端不会加载它）。
            .unwrap_or((true, "code".to_string(), None, None, None))
    };

    let valid_perms = match &existing_perms {
        Some(existing) => {
            if *existing != requested_perms {
                tracing::warn!(
                    "plugin {} re-registered with different permissions {:?}; kept existing {:?} \
                     (use the admin permissions API to change them)",
                    payload.id,
                    requested_perms,
                    existing,
                );
            }
            existing.clone()
        }
        None => requested_perms,
    };

    let manifest = PluginManifest {
        id: payload.id.clone(),
        name: payload.manifest.name.clone(),
        version: payload.manifest.version.clone(),
        description: payload.manifest.description.clone(),
        author: payload.manifest.author.clone(),
        permissions: valid_perms.clone(),
        enabled: existing_enabled,
        source: existing_source,
        entry: existing_entry,
        source_url: existing_source_url,
    };

    {
        let mut plugins = state.plugins.write().await;
        plugins.insert(payload.id.clone(), manifest.clone());
    }
    state.persist_plugin(&manifest).await;

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
/// know which plugins are enabled/disabled so it can hide disabled ones,
/// and to discover zip-installed plugins to load dynamically.
pub async fn list_plugins_public(State(state): State<AppState>) -> Json<serde_json::Value> {
    let plugins = state.plugins.read().await;
    let globally_disabled = *state.plugins_disabled.read().await;
    let list: Vec<serde_json::Value> = plugins
        .values()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "version": p.version,
                "enabled": p.enabled,
                "source": p.source,
                "entry": p.entry,
            })
        })
        .collect();

    Json(serde_json::json!({
        "plugins": list,
        "plugins_disabled": globally_disabled,
    }))
}

// ── List plugins (admin) ──

/// GET /api/admin/plugins
pub async fn list_plugins(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let plugins = state.plugins.read().await;
    let list: Vec<&PluginManifest> = plugins.values().collect();
    let globally_disabled = *state.plugins_disabled.read().await;

    Ok(Json(serde_json::json!({
        "plugins": list,
        "plugins_disabled": globally_disabled,
        "known_permissions": plugin_perms::ALL,
    })))
}

// ── Set plugin permissions (admin) ──

/// PUT /api/admin/plugins/{plugin_id}/permissions
/// Body: { "permissions": ["storage", "read:team"] }
///
/// 权限清单在首次注册后冻结；这是唯一（superadmin）调整它的途径。
pub async fn set_plugin_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<SetPermissionsPayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    // 只接受已知权限，去重并保持稳定顺序
    let mut permissions: Vec<String> = Vec::new();
    for p in &payload.permissions {
        if !plugin_perms::ALL.contains(&p.as_str()) {
            return Err(bad_request(
                ErrorCode::PLUGIN_DATA_INVALID,
                format!("未知的插件权限: {}", p),
            ));
        }
        if !permissions.contains(p) {
            permissions.push(p.clone());
        }
    }

    let updated = {
        let mut plugins = state.plugins.write().await;
        plugins.get_mut(&plugin_id).map(|plugin| {
            plugin.permissions = permissions.clone();
            plugin.clone()
        })
    };

    match updated {
        Some(manifest) => {
            state.persist_plugin(&manifest).await;
            audit_plugin(
                &state,
                &auth,
                "plugin.set_permissions",
                &manifest.id,
                &format!("permissions={:?}", manifest.permissions),
            )
            .await;
            tracing::info!(
                "plugin permissions updated: {} → {:?}",
                manifest.id,
                manifest.permissions,
            );
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("插件「{}」权限已更新", manifest.name),
                "permissions_needed": manifest.permissions,
            })))
        }
        None => Ok(json_error(ErrorCode::PLUGIN_NOT_FOUND, "插件不存在")),
    }
}

// ── Unregister plugin (admin) ──

/// DELETE /api/admin/plugins/{plugin_id}
pub async fn unregister_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
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

    // 清理磁盘文件（zip 资产 + 插件文件存储）与 SQLite 记录
    let _ = std::fs::remove_dir_all(plugin_root_dir(&state, &plugin_id));
    state.remove_plugin_db(&plugin_id).await;

    match removed {
        Some(p) => {
            audit_plugin(&state, &auth, "plugin.uninstall", &p.id, &p.name).await;
            tracing::info!("plugin unregistered: {} ({})", p.name, p.id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("插件「{}」已卸载", p.name),
            })))
        }
        None => Ok(json_error(ErrorCode::PLUGIN_NOT_FOUND, "插件不存在")),
    }
}

// ── Enable / Disable plugin (admin) ──

/// POST /api/admin/plugins/{plugin_id}/enable
pub async fn enable_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let updated = {
        let mut plugins = state.plugins.write().await;
        plugins.get_mut(&plugin_id).map(|plugin| {
            plugin.enabled = true;
            plugin.clone()
        })
    };

    match updated {
        Some(manifest) => {
            state.persist_plugin(&manifest).await;
            audit_plugin(&state, &auth, "plugin.enable", &manifest.id, &manifest.name).await;
            tracing::info!("plugin enabled: {} ({})", manifest.name, manifest.id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("插件「{}」已启用", manifest.name),
            })))
        }
        None => Ok(json_error(ErrorCode::PLUGIN_NOT_FOUND, "插件不存在")),
    }
}

/// POST /api/admin/plugins/{plugin_id}/disable
pub async fn disable_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let updated = {
        let mut plugins = state.plugins.write().await;
        plugins.get_mut(&plugin_id).map(|plugin| {
            plugin.enabled = false;
            plugin.clone()
        })
    };

    match updated {
        Some(manifest) => {
            state.persist_plugin(&manifest).await;
            audit_plugin(
                &state,
                &auth,
                "plugin.disable",
                &manifest.id,
                &manifest.name,
            )
            .await;
            tracing::info!("plugin disabled: {} ({})", manifest.name, manifest.id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("插件「{}」已禁用", manifest.name),
            })))
        }
        None => Ok(json_error(ErrorCode::PLUGIN_NOT_FOUND, "插件不存在")),
    }
}

// ── Global plugin enable / disable (superadmin) ──

/// GET /api/admin/plugins/global
/// Returns the global plugin enable/disable state.
pub async fn get_global_plugin_state(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let disabled = *state.plugins_disabled.read().await;
    Ok(Json(serde_json::json!({
        "plugins_disabled": disabled,
    })))
}

/// POST /api/admin/plugins/global
/// Body: { "enabled": bool } — set the global plugin feature on/off.
pub async fn set_global_plugin_state(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<PluginTogglePayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    let new_disabled = !payload.enabled;
    *state.plugins_disabled.write().await = new_disabled;
    state.persist_plugins_disabled(new_disabled).await;
    audit_plugin(
        &state,
        &auth,
        "plugin.global_toggle",
        "*",
        if new_disabled {
            "plugins_disabled=true"
        } else {
            "plugins_disabled=false"
        },
    )
    .await;

    tracing::info!(
        "plugin feature globally {}",
        if new_disabled { "disabled" } else { "enabled" }
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "plugins_disabled": new_disabled,
        "message": if new_disabled {
            "插件功能已全局禁用".to_string()
        } else {
            "插件功能已全局启用".to_string()
        },
    })))
}

// ── Install / update plugins from zip or URL (superadmin) ──

/// plugin.json 的期望结构（zip 包根目录）。
#[derive(serde::Deserialize)]
struct ZipPluginManifestJson {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    permissions_needed: Vec<String>,
    /// 入口文件（assets 内相对路径），默认 index.js
    #[serde(default)]
    entry: Option<String>,
}

/// POST /api/admin/plugins/install-url 的请求体。
#[derive(serde::Deserialize)]
pub struct InstallFromUrlPayload {
    pub url: String,
}

/// 一次「安装 / 更新」的结果。
struct ZipInstallOutcome {
    manifest: PluginManifest,
    file_count: usize,
    /// true = 覆盖已有插件（更新），false = 全新安装
    updated: bool,
    /// 更新前的版本号（全新安装为 None）
    previous_version: Option<String>,
    /// 新版本声明、但当前未被授予的权限（提示管理员到后台勾选）
    new_permissions: Vec<String>,
}

impl ZipInstallOutcome {
    fn into_response(self) -> Json<serde_json::Value> {
        let message = match (&self.updated, &self.previous_version) {
            (true, Some(prev)) if *prev != self.manifest.version => format!(
                "插件「{}」已从 v{} 更新到 v{}",
                self.manifest.name, prev, self.manifest.version
            ),
            (true, _) => format!(
                "插件「{}」已重新安装 v{}",
                self.manifest.name, self.manifest.version
            ),
            (false, _) => format!("插件「{}」安装成功", self.manifest.name),
        };
        Json(serde_json::json!({
            "success": true,
            "updated": self.updated,
            "message": message,
            "previous_version": self.previous_version,
            "new_permissions": self.new_permissions,
            "plugin": {
                "id": self.manifest.id,
                "name": self.manifest.name,
                "version": self.manifest.version,
                "description": self.manifest.description,
                "author": self.manifest.author,
                "permissions_needed": self.manifest.permissions,
                "enabled": self.manifest.enabled,
                "source": self.manifest.source,
                "source_url": self.manifest.source_url,
                "entry": self.manifest.entry,
                "file_count": self.file_count,
            }
        }))
    }
}

/// 安装 / 更新核心：解析 zip、整体替换 assets、登记清单。
///
/// 更新语义（已存在同 id 插件时）：
///   - 保留 `enabled`、已授权权限与插件数据（KV / files 目录不动）
///   - 不自动授予新版本额外申请的权限，改由 `new_permissions` 提示管理员勾选
///   - `source_url` 传入 Some 时覆盖，否则保留原值
///
/// `expect_id` 为 Some 时要求 zip 内 plugin.json 的 id 与其一致（更新接口用）。
async fn install_zip_bytes(
    state: &AppState,
    bytes: &[u8],
    source_url: Option<String>,
    expect_id: Option<&str>,
) -> Result<ZipInstallOutcome, ApiErr> {
    let invalid = |msg: &str| bad_request(ErrorCode::PLUGIN_INVALID_PACKAGE, msg);

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|_| invalid("无法解析 zip 文件"))?;
    if archive.len() > MAX_ZIP_FILES {
        return Err(invalid("插件包文件数超限"));
    }

    // ── 读取 plugin.json ──
    let manifest_json: ZipPluginManifestJson = {
        let mut f = archive
            .by_name("plugin.json")
            .map_err(|_| invalid("插件包缺少 plugin.json"))?;
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut f, &mut buf)
            .map_err(|_| invalid("无法读取 plugin.json"))?;
        serde_json::from_str(&buf).map_err(|_| invalid("plugin.json 格式不合法"))?
    };

    if !is_valid_plugin_id(&manifest_json.id) {
        return Err(invalid(
            "插件 id 不合法（需以字母或数字开头，仅含字母数字、-、_，长度 ≤ 64）",
        ));
    }
    if manifest_json.name.is_empty() || manifest_json.version.is_empty() {
        return Err(invalid("plugin.json 缺少 name 或 version"));
    }
    if let Some(expected) = expect_id {
        if manifest_json.id != expected {
            return Err(invalid(&format!(
                "插件包内的 id「{}」与要更新的插件「{}」不一致",
                manifest_json.id, expected
            )));
        }
    }

    let entry = manifest_json
        .entry
        .clone()
        .unwrap_or_else(|| "index.js".to_string());
    let entry = sanitize_rel_path(&entry).ok_or_else(|| invalid("入口文件路径不合法"))?;

    // ── 解压全部文件（带限额与路径清洗） ──
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total: u64 = 0;
    let mut has_entry = false;
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|_| invalid("读取 zip 条目失败"))?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        // 不合法路径直接跳过（不中断安装）
        let rel = match sanitize_rel_path(&name) {
            Some(r) => r,
            None => continue,
        };
        let size = f.size();
        if size > MAX_PLUGIN_FILE_SIZE {
            return Err(invalid("插件包包含超过 8 MiB 的单文件"));
        }
        total += size;
        if total > MAX_ZIP_TOTAL_UNCOMPRESSED {
            return Err(invalid("插件包解压后总大小超限"));
        }
        let mut data = Vec::with_capacity(size as usize);
        std::io::Read::read_to_end(&mut f, &mut data).map_err(|_| invalid("读取 zip 条目失败"))?;
        if rel == entry {
            has_entry = true;
        }
        files.push((rel, data));
    }
    if !has_entry {
        return Err(invalid("入口文件不存在于插件包中"));
    }

    // ── 写入 assets 目录（整体替换，支持升级重装） ──
    let assets_dir = plugin_assets_dir(state, &manifest_json.id);
    let _ = std::fs::remove_dir_all(&assets_dir);
    for (rel, data) in &files {
        let dest = assets_dir.join(rel);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&dest, data).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(ErrorCode::INTERNAL_ERROR, "写入插件文件失败"),
            )
        })?;
    }

    // ── 已注册信息（决定安装还是更新） ──
    let existing = {
        let plugins = state.plugins.read().await;
        plugins.get(&manifest_json.id).cloned()
    };
    let previous_version = existing.as_ref().map(|p| p.version.clone());
    let updated = previous_version.is_some();
    let enabled = existing.as_ref().map(|p| p.enabled).unwrap_or(true);

    let declared_perms: Vec<String> = manifest_json
        .permissions_needed
        .iter()
        .filter(|p| plugin_perms::ALL.contains(&p.as_str()))
        .cloned()
        .collect();

    let (valid_perms, new_permissions) = match existing.as_ref() {
        // 更新：权限保持冻结，只把新版本额外申请的权限回给管理员提示
        Some(prev) => (
            prev.permissions.clone(),
            declared_perms
                .iter()
                .filter(|p| !prev.permissions.contains(p))
                .cloned()
                .collect(),
        ),
        // 新装：按声明授予已知权限
        None => (declared_perms, Vec::new()),
    };

    let source_url = source_url.or_else(|| existing.as_ref().and_then(|p| p.source_url.clone()));

    // ── 写入 assets 目录（整体替换；插件 KV 与 files 目录不受影响） ──
    let assets_dir = plugin_assets_dir(state, &manifest_json.id);
    let _ = std::fs::remove_dir_all(&assets_dir);
    for (rel, data) in &files {
        let dest = assets_dir.join(rel);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&dest, data).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error(ErrorCode::INTERNAL_ERROR, "写入插件文件失败"),
            )
        })?;
    }

    let manifest = PluginManifest {
        id: manifest_json.id.clone(),
        name: manifest_json.name.clone(),
        version: manifest_json.version.clone(),
        description: manifest_json.description.clone(),
        author: manifest_json.author.clone(),
        permissions: valid_perms.clone(),
        enabled,
        source: "zip".to_string(),
        entry: Some(entry),
        source_url,
    };

    {
        let mut plugins = state.plugins.write().await;
        plugins.insert(manifest.id.clone(), manifest.clone());
    }
    state.persist_plugin(&manifest).await;

    Ok(ZipInstallOutcome {
        file_count: files.len(),
        updated,
        previous_version,
        new_permissions,
        manifest,
    })
}

/// 审计 + 日志 + 统一响应（安装与更新共用）。
async fn finish_install(
    state: &AppState,
    auth: &AuthUser,
    outcome: ZipInstallOutcome,
) -> Json<serde_json::Value> {
    let action = if outcome.updated {
        "plugin.update"
    } else {
        "plugin.install"
    };
    audit_plugin(
        state,
        auth,
        action,
        &outcome.manifest.id,
        &format!("v{} files={}", outcome.manifest.version, outcome.file_count),
    )
    .await;

    tracing::info!(
        "plugin {}: {} v{} ({} files, permissions: {:?}, newly requested: {:?})",
        action,
        outcome.manifest.id,
        outcome.manifest.version,
        outcome.file_count,
        outcome.manifest.permissions,
        outcome.new_permissions,
    );

    outcome.into_response()
}

/// 更新前确认插件已注册，避免把新插件误当作「更新」装进来。
async fn ensure_plugin_registered(state: &AppState, plugin_id: &str) -> Result<(), ApiErr> {
    if state.plugins.read().await.contains_key(plugin_id) {
        Ok(())
    } else {
        Err(not_found("插件未注册"))
    }
}

/// 校验并归一化插件来源 URL：仅允许 http/https、禁止携带凭据，并拒绝云元数据地址
/// （基础 SSRF 防护）。内网镜像地址仍允许 —— 该接口仅超级管理员可用。
fn validate_plugin_source_url(raw: &str) -> Result<String, ApiErr> {
    let fail = |msg: &str| bad_request(ErrorCode::PLUGIN_DOWNLOAD_FAILED, msg);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(fail("URL 不能为空"));
    }
    let url = reqwest::Url::parse(trimmed).map_err(|_| fail("URL 格式不合法"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(fail("仅支持 http/https URL"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(fail("URL 不能包含账号密码"));
    }
    if let Some(host) = url.host_str() {
        let host = host.to_ascii_lowercase();
        if host == "metadata.google.internal" || host == "169.254.169.254" {
            return Err(fail("不允许从云元数据地址下载插件包"));
        }
    }
    Ok(url.to_string())
}

/// 插件包下载专用客户端：超时比 OAuth 用的客户端更长（允许较大的包）。
fn plugin_download_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("创建插件下载 HTTP 客户端失败")
    })
}

/// 流式下载插件包，超过 `MAX_PLUGIN_ZIP_BYTES` 立即中断。
async fn download_plugin_zip(url: &str) -> Result<Vec<u8>, ApiErr> {
    let too_large = || {
        bad_request(
            ErrorCode::PLUGIN_DOWNLOAD_FAILED,
            format!("插件包超过 {} MiB 上限", MAX_PLUGIN_ZIP_BYTES / 1024 / 1024),
        )
    };
    let fail = |msg: String| bad_request(ErrorCode::PLUGIN_DOWNLOAD_FAILED, msg);

    let mut res = plugin_download_client()
        .get(url)
        .header(
            header::ACCEPT,
            "application/zip, application/octet-stream, */*",
        )
        .header(header::USER_AGENT, "mcguffin-plugin-installer")
        .send()
        .await
        .map_err(|e| fail(format!("下载插件包失败：{e}")))?;

    if !res.status().is_success() {
        return Err(fail(format!("下载插件包失败：HTTP {}", res.status())));
    }
    if res
        .content_length()
        .is_some_and(|len| len > MAX_PLUGIN_ZIP_BYTES)
    {
        return Err(too_large());
    }

    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = res
        .chunk()
        .await
        .map_err(|e| fail(format!("下载插件包中断：{e}")))?
    {
        if buf.len() as u64 + chunk.len() as u64 > MAX_PLUGIN_ZIP_BYTES {
            return Err(too_large());
        }
        buf.extend_from_slice(&chunk);
    }
    if buf.is_empty() {
        return Err(fail("下载到的插件包为空".to_string()));
    }
    Ok(buf)
}

/// POST /api/admin/plugins/install-zip
/// Body: raw zip bytes (application/octet-stream).
///
/// zip 包约定：
///   - 根目录必须有 plugin.json（id/name/version，可选 entry/permissions_needed）
///   - 入口文件默认 index.js，须为 ESM，通过 window.__MCGUFFIN_SDK__ 调用 definePlugin()
///   - 全部文件解压到 <data_dir>/plugins/{id}/assets/，经 /api/plugins/{id}/assets/* 公开访问
///   - 若该 id 已安装，则按「更新」处理（保留插件数据、启用状态与已授权权限）
pub async fn install_plugin_zip(
    State(state): State<AppState>,
    auth: AuthUser,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let outcome = install_zip_bytes(&state, body.as_ref(), None, None).await?;
    Ok(finish_install(&state, &auth, outcome).await)
}

/// POST /api/admin/plugins/install-url
/// Body: { "url": "https://example.com/my-plugin.zip" }
///
/// 服务端下载 zip 后按上述约定安装 / 更新，并记录来源 URL 供「从原 URL 更新」。
pub async fn install_plugin_from_url(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<InstallFromUrlPayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let url = validate_plugin_source_url(&payload.url)?;
    let bytes = download_plugin_zip(&url).await?;
    let outcome = install_zip_bytes(&state, &bytes, Some(url), None).await?;
    Ok(finish_install(&state, &auth, outcome).await)
}

/// POST /api/admin/plugins/{plugin_id}/update-zip
/// Body: raw zip bytes (application/octet-stream)；包内 plugin.json 的 id 必须与 {plugin_id} 一致。
pub async fn update_plugin_zip(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    ensure_plugin_registered(&state, &plugin_id).await?;
    let outcome = install_zip_bytes(&state, body.as_ref(), None, Some(&plugin_id)).await?;
    Ok(finish_install(&state, &auth, outcome).await)
}

/// POST /api/admin/plugins/{plugin_id}/update
/// 从安装时记录的来源 URL 重新下载并更新（保留插件数据、启用状态与已授权权限）。
pub async fn update_plugin_from_url(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let source_url = {
        let plugins = state.plugins.read().await;
        let plugin = plugins
            .get(&plugin_id)
            .ok_or_else(|| not_found("插件未注册"))?;
        plugin.source_url.clone()
    };
    let url = source_url.ok_or_else(|| {
        (
            ErrorCode::PLUGIN_UPDATE_UNAVAILABLE.status(),
            json_error(
                ErrorCode::PLUGIN_UPDATE_UNAVAILABLE,
                "该插件不是从 URL 安装的，无法从原 URL 更新",
            ),
        )
    })?;
    let bytes = download_plugin_zip(&url).await?;
    let outcome = install_zip_bytes(&state, &bytes, Some(url), Some(&plugin_id)).await?;
    Ok(finish_install(&state, &auth, outcome).await)
}

// ── Static assets of zip-installed plugins (public) ──

/// GET /api/plugins/{plugin_id}/assets/{*path}
/// Serves static files of zip-installed plugins. No auth required — assets are
/// static frontend code; all data APIs remain permission-checked.
pub async fn plugin_asset(
    State(state): State<AppState>,
    Path((plugin_id, path)): Path<(String, String)>,
) -> Result<Response, ApiErr> {
    require_plugins_globally_enabled(&state).await?;
    let plugins = state.plugins.read().await;
    let plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| not_found("插件未注册"))?
        .clone();
    drop(plugins);
    require_plugin_enabled(&plugin)?;
    if plugin.source != "zip" {
        return Err(not_found("插件无静态资源"));
    }

    let rel = sanitize_rel_path(&path)
        .ok_or_else(|| bad_request(ErrorCode::PLUGIN_DATA_INVALID, "非法路径"))?;
    let full = plugin_assets_dir(&state, &plugin_id).join(&rel);
    let content = tokio::fs::read(&full)
        .await
        .map_err(|_| not_found("资源不存在"))?;
    let mime = mime_guess::from_path(&full).first_or_octet_stream();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(content))
        .unwrap())
}

// ── Users: list team members ──

/// GET /api/plugins/{plugin_id}/users
/// Returns team members. Requires read:team plugin permission.
pub async fn plugin_list_users(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    // write:team / write:team_roles 蕴含 read:team
    ensure_plugin_perm(
        plugin_perms::READ_TEAM,
        plugin_perms::implies_read_team(&plugin.permissions),
    )?;

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
    if *state.plugins_disabled.read().await {
        return json_error(ErrorCode::PLUGIN_DISABLED, "插件功能已被管理员全局禁用");
    }
    let plugins = state.plugins.read().await;
    let plugin = match plugins.get(&plugin_id) {
        Some(p) => p.clone(),
        None => return json_error(ErrorCode::PLUGIN_NOT_FOUND, "插件未注册"),
    };
    drop(plugins);

    if !plugin.enabled {
        return json_error(ErrorCode::PLUGIN_DISABLED, "插件已被禁用");
    }

    let users = state.users.read().await;
    let user = match users.get(&auth.user_id) {
        Some(u) => u.clone(),
        None => return json_error(ErrorCode::USER_NOT_FOUND, "用户不存在"),
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
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    // read:users:email 蕴含 read:users
    ensure_plugin_perm(
        plugin_perms::READ_USERS,
        plugin_perms::implies_read_users(&plugin.permissions),
    )?;

    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json_error(ErrorCode::USER_NOT_FOUND, "用户不存在"),
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
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let namespace = params.get("namespace").cloned().unwrap_or_default();
    let key = params.get("key").cloned().unwrap_or_default();
    validate_kv_target(&namespace, &key)?;

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
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    validate_kv_target(&payload.namespace, &payload.key)?;
    validate_kv_value(&payload.value)?;

    {
        let mut data = state.plugin_data.write().await;
        ensure_kv_quota(&data, &plugin_id, &payload.namespace, &payload.key)?;
        let ns = data.entry(plugin_id.clone()).or_insert_with(HashMap::new);
        let kv = ns
            .entry(payload.namespace.clone())
            .or_insert_with(HashMap::new);
        kv.insert(payload.key.clone(), payload.value.clone());
    }
    state
        .persist_plugin_data_value(&plugin_id, &payload.namespace, &payload.key, &payload.value)
        .await;

    Ok(Json(serde_json::json!({"success": true})))
}

// ── Data: atomic counters ──

/// POST /api/plugins/{plugin_id}/data/add
/// Body: { namespace, key, delta }
/// Atomically adds `delta` to an integer counter (default 0) and returns the new value.
pub async fn plugin_add_data(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<AddDataPayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    validate_kv_target(&payload.namespace, &payload.key)?;

    let new_val = {
        let mut data = state.plugin_data.write().await;
        ensure_kv_quota(&data, &plugin_id, &payload.namespace, &payload.key)?;
        let kv = data
            .entry(plugin_id.clone())
            .or_insert_with(HashMap::new)
            .entry(payload.namespace.clone())
            .or_insert_with(HashMap::new);
        let cur: i64 = kv
            .get(&payload.key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let new = cur.saturating_add(payload.delta);
        kv.insert(payload.key.clone(), new.to_string());
        new
    };
    state
        .persist_plugin_data_value(
            &plugin_id,
            &payload.namespace,
            &payload.key,
            &new_val.to_string(),
        )
        .await;

    Ok(Json(serde_json::json!({"value": new_val})))
}

// ── Data: string sets ──

/// POST /api/plugins/{plugin_id}/data/set-add
/// Body: { namespace, key, member } — adds member to a JSON string set.
pub async fn plugin_set_add(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<SetMemberPayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    validate_kv_target(&payload.namespace, &payload.key)?;
    validate_kv_member(&payload.member)?;

    let (added, serialized) = {
        let mut data = state.plugin_data.write().await;
        ensure_kv_quota(&data, &plugin_id, &payload.namespace, &payload.key)?;
        let kv = data
            .entry(plugin_id.clone())
            .or_insert_with(HashMap::new)
            .entry(payload.namespace.clone())
            .or_insert_with(HashMap::new);
        let mut set = read_json_set(kv.get(&payload.key));
        let added = if set.contains(&payload.member) {
            false
        } else {
            set.push(payload.member.clone());
            true
        };
        let serialized = serde_json::to_string(&set).unwrap_or_default();
        validate_kv_value(&serialized)?;
        kv.insert(payload.key.clone(), serialized.clone());
        (added, serialized)
    };
    state
        .persist_plugin_data_value(&plugin_id, &payload.namespace, &payload.key, &serialized)
        .await;

    Ok(Json(serde_json::json!({"added": added})))
}

/// POST /api/plugins/{plugin_id}/data/set-remove
/// Body: { namespace, key, member } — removes member from a JSON string set.
pub async fn plugin_set_remove(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<SetMemberPayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    validate_kv_target(&payload.namespace, &payload.key)?;
    validate_kv_member(&payload.member)?;

    let (removed, serialized) = {
        let mut data = state.plugin_data.write().await;
        ensure_kv_quota(&data, &plugin_id, &payload.namespace, &payload.key)?;
        let kv = data
            .entry(plugin_id.clone())
            .or_insert_with(HashMap::new)
            .entry(payload.namespace.clone())
            .or_insert_with(HashMap::new);
        let mut set = read_json_set(kv.get(&payload.key));
        let before = set.len();
        set.retain(|m| m != &payload.member);
        let removed = set.len() != before;
        let serialized = serde_json::to_string(&set).unwrap_or_default();
        kv.insert(payload.key.clone(), serialized.clone());
        (removed, serialized)
    };
    state
        .persist_plugin_data_value(&plugin_id, &payload.namespace, &payload.key, &serialized)
        .await;

    Ok(Json(serde_json::json!({"removed": removed})))
}

/// GET /api/plugins/{plugin_id}/data/set-members?namespace=X&key=Y
pub async fn plugin_set_members(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let namespace = params.get("namespace").cloned().unwrap_or_default();
    let key = params.get("key").cloned().unwrap_or_default();
    validate_kv_target(&namespace, &key)?;

    let data = state.plugin_data.read().await;
    let members = read_json_set(
        data.get(&plugin_id)
            .and_then(|ns| ns.get(&namespace))
            .and_then(|kv| kv.get(&key)),
    );

    Ok(Json(serde_json::json!({
        "members": members,
        "count": members.len(),
    })))
}

/// GET /api/plugins/{plugin_id}/data/set-is-member?namespace=X&key=Y&member=Z
pub async fn plugin_set_is_member(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let namespace = params.get("namespace").cloned().unwrap_or_default();
    let key = params.get("key").cloned().unwrap_or_default();
    let member = params.get("member").cloned().unwrap_or_default();
    validate_kv_target(&namespace, &key)?;
    validate_kv_member(&member)?;

    let data = state.plugin_data.read().await;
    let set = read_json_set(
        data.get(&plugin_id)
            .and_then(|ns| ns.get(&namespace))
            .and_then(|kv| kv.get(&key)),
    );

    Ok(Json(serde_json::json!({
        "is_member": set.contains(&member),
    })))
}

// ── Data: key listing ──

/// GET /api/plugins/{plugin_id}/data/keys?namespace=X&prefix=Y
/// Lists keys in a namespace, optionally filtered by prefix.
pub async fn plugin_data_keys(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let namespace = params.get("namespace").cloned().unwrap_or_default();
    if namespace.is_empty() {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            "namespace 不能为空",
        ));
    }
    if namespace.len() > MAX_KV_NAMESPACE_LEN {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            format!("namespace 长度不能超过 {} 字符", MAX_KV_NAMESPACE_LEN),
        ));
    }
    let prefix = params.get("prefix").cloned();

    let data = state.plugin_data.read().await;
    let mut keys: Vec<String> = data
        .get(&plugin_id)
        .and_then(|ns| ns.get(&namespace))
        .map(|kv| {
            kv.keys()
                .filter(|k| prefix.as_deref().map(|p| k.starts_with(p)).unwrap_or(true))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    keys.sort();

    Ok(Json(serde_json::json!({"keys": keys})))
}

// ── Files ──

/// POST /api/plugins/{plugin_id}/files/{*path}
/// Body: raw file bytes. Writes a file into the plugin's private file storage.
pub async fn plugin_write_file(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((plugin_id, path)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    if body.len() as u64 > MAX_PLUGIN_FILE_SIZE {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            "文件大小超过 8 MiB 上限",
        ));
    }
    let rel = sanitize_rel_path(&path)
        .ok_or_else(|| bad_request(ErrorCode::PLUGIN_DATA_INVALID, "非法文件路径"))?;
    let full = plugin_files_dir(&state, &plugin_id).join(&rel);
    if let Some(parent) = full.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&full, &body).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(ErrorCode::INTERNAL_ERROR, "写入文件失败"),
        )
    })?;

    Ok(Json(serde_json::json!({
        "path": rel,
        "size": body.len(),
    })))
}

/// GET /api/plugins/{plugin_id}/files/{*path}
/// Reads a file from the plugin's private file storage.
pub async fn plugin_read_file(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((plugin_id, path)): Path<(String, String)>,
) -> Result<Response, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let rel = sanitize_rel_path(&path)
        .ok_or_else(|| bad_request(ErrorCode::PLUGIN_DATA_INVALID, "非法文件路径"))?;
    let full = plugin_files_dir(&state, &plugin_id).join(&rel);
    let content = tokio::fs::read(&full)
        .await
        .map_err(|_| not_found("文件不存在"))?;
    let mime = mime_guess::from_path(&full).first_or_octet_stream();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(content))
        .unwrap())
}

/// DELETE /api/plugins/{plugin_id}/files/{*path}
pub async fn plugin_delete_file(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((plugin_id, path)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let rel = sanitize_rel_path(&path)
        .ok_or_else(|| bad_request(ErrorCode::PLUGIN_DATA_INVALID, "非法文件路径"))?;
    let full = plugin_files_dir(&state, &plugin_id).join(&rel);
    if !full.exists() {
        return Err(not_found("文件不存在"));
    }
    tokio::fs::remove_file(&full).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error(ErrorCode::INTERNAL_ERROR, "删除文件失败"),
        )
    })?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// GET /api/plugins/{plugin_id}/files/list?prefix=X
/// Lists files in the plugin's private file storage, optionally filtered by prefix.
pub async fn plugin_list_files(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::STORAGE);

    let prefix = params.get("prefix").cloned().unwrap_or_default();
    let base = plugin_files_dir(&state, &plugin_id);
    let mut files: Vec<String> = Vec::new();
    walk_files(&base, &base, &prefix, &mut files);
    files.sort();
    files.truncate(MAX_FILE_LIST);

    Ok(Json(serde_json::json!({
        "files": files,
        "count": files.len(),
    })))
}

fn walk_files(base: &std::path::Path, dir: &std::path::Path, prefix: &str, out: &mut Vec<String>) {
    if out.len() >= MAX_FILE_LIST {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_files(base, &p, prefix, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            if prefix.is_empty() || rel.starts_with(prefix) {
                out.push(rel);
            }
        }
    }
}

// ── Notify ──

/// POST /api/plugins/{plugin_id}/notify
/// Body: { user_id, title, body, link? }
pub async fn plugin_notify(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(payload): Json<NotifyPayload>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let plugin = load_enabled_plugin(&state, &plugin_id).await?;
    require_plugin_perm!(&plugin, plugin_perms::NOTIFY);

    if payload.title.is_empty() || payload.body.is_empty() {
        return Err(bad_request(
            ErrorCode::PLUGIN_DATA_INVALID,
            "title 和 body 不能为空",
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

    Ok(Json(
        serde_json::json!({"success": true, "message": "通知已发送"}),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_kv_quota, is_valid_plugin_id, read_json_set, sanitize_rel_path, validate_kv_member,
        validate_kv_target, validate_kv_value, validate_plugin_source_url, MAX_KV_KEYS_PER_PLUGIN,
        MAX_KV_KEY_LEN, MAX_KV_NAMESPACE_LEN, MAX_KV_VALUE_BYTES,
    };

    #[test]
    fn plugin_source_url_accepts_http_and_https() {
        assert_eq!(
            validate_plugin_source_url("https://example.com/my-plugin.zip").unwrap(),
            "https://example.com/my-plugin.zip"
        );
        // 前后空白被裁剪
        assert_eq!(
            validate_plugin_source_url("  http://mirror.local/p.zip  ").unwrap(),
            "http://mirror.local/p.zip"
        );
        // 带查询串的发布地址（如 GitHub releases/latest/download）
        assert_eq!(
            validate_plugin_source_url("https://github.com/o/r/releases/latest/download/p.zip")
                .unwrap(),
            "https://github.com/o/r/releases/latest/download/p.zip"
        );
    }

    #[test]
    fn plugin_source_url_rejects_unsafe_inputs() {
        assert!(validate_plugin_source_url("").is_err());
        assert!(validate_plugin_source_url("   ").is_err());
        assert!(validate_plugin_source_url("not a url").is_err());
        assert!(validate_plugin_source_url("ftp://example.com/p.zip").is_err());
        assert!(validate_plugin_source_url("file:///etc/passwd").is_err());
        assert!(validate_plugin_source_url("https://user:pw@example.com/p.zip").is_err());
        assert!(validate_plugin_source_url("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn kv_target_and_value_limits() {
        assert!(validate_kv_target("ns", "key").is_ok());
        assert!(validate_kv_target("", "key").is_err());
        assert!(validate_kv_target("ns", "").is_err());
        assert!(validate_kv_target(&"n".repeat(MAX_KV_NAMESPACE_LEN + 1), "key").is_err());
        assert!(validate_kv_target("ns", &"k".repeat(MAX_KV_KEY_LEN + 1)).is_err());

        assert!(validate_kv_value("v").is_ok());
        assert!(validate_kv_value(&"v".repeat(MAX_KV_VALUE_BYTES)).is_ok());
        assert!(validate_kv_value(&"v".repeat(MAX_KV_VALUE_BYTES + 1)).is_err());

        assert!(validate_kv_member("u1").is_ok());
        assert!(validate_kv_member("").is_err());
        assert!(validate_kv_member(&"u".repeat(257)).is_err());
    }

    #[test]
    fn kv_quota_blocks_new_keys_but_allows_updates() {
        use std::collections::HashMap;

        let mut data: crate::state::PluginDataStore = HashMap::new();
        // 已有 key 的更新永远放行（即使已达上限）
        data.entry("p1".to_string())
            .or_default()
            .entry("ns".to_string())
            .or_default()
            .insert("existing".to_string(), "1".to_string());
        assert!(ensure_kv_quota(&data, "p1", "ns", "existing").is_ok());

        // 填满配额
        let ns = data
            .get_mut("p1")
            .unwrap()
            .entry("ns".to_string())
            .or_default();
        for i in 0..(MAX_KV_KEYS_PER_PLUGIN - 1) {
            ns.insert(format!("k{}", i), "1".to_string());
        }
        assert_eq!(
            data.get("p1")
                .map(|ns| ns.values().map(|kv| kv.len()).sum::<usize>()),
            Some(MAX_KV_KEYS_PER_PLUGIN)
        );
        // 新增 key 被拒
        assert!(ensure_kv_quota(&data, "p1", "ns", "brand-new").is_err());
        // 更新已有 key 仍放行
        assert!(ensure_kv_quota(&data, "p1", "ns", "existing").is_ok());
        // 其它插件不受影响
        assert!(ensure_kv_quota(&data, "p2", "ns", "anything").is_ok());
        // 新命名空间同样计入总额
        assert!(ensure_kv_quota(&data, "p1", "other-ns", "k").is_err());
    }

    #[test]
    fn sanitize_rel_path_rejects_traversal() {
        assert_eq!(sanitize_rel_path("a/b/c.js"), Some("a/b/c.js".to_string()));
        assert_eq!(sanitize_rel_path("./a//b.js"), Some("a/b.js".to_string()));
        assert_eq!(sanitize_rel_path("a\\b.js"), Some("a/b.js".to_string()));
        assert!(sanitize_rel_path("../evil.js").is_none());
        assert!(sanitize_rel_path("a/../../evil.js").is_none());
        assert!(sanitize_rel_path("/abs.js").is_none());
        assert!(sanitize_rel_path("").is_none());
        assert!(sanitize_rel_path("/").is_none());
    }

    #[test]
    fn plugin_id_validation() {
        assert!(is_valid_plugin_id("lollipop-rank"));
        assert!(is_valid_plugin_id("team_members2"));
        assert!(!is_valid_plugin_id(""));
        assert!(!is_valid_plugin_id("-bad"));
        assert!(!is_valid_plugin_id("has space"));
        assert!(!is_valid_plugin_id("has/slash"));
        assert!(!is_valid_plugin_id(&"x".repeat(65)));
    }

    #[test]
    fn json_set_tolerates_garbage() {
        assert_eq!(read_json_set(None), Vec::<String>::new());
        assert_eq!(
            read_json_set(Some(&"not json".to_string())),
            Vec::<String>::new()
        );
        assert_eq!(
            read_json_set(Some(&"[\"a\",\"b\"]".to_string())),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
