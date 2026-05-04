use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::path::PathBuf;
use std::str::FromStr;
use toml_edit::{DocumentMut, Item, Value as TomlValue};
use chrono::Local;

use crate::state::AppState;
use crate::types::DifficultyLevel;
use crate::utils::{is_superadmin, resolve_user};

const CONFIG_PATH: &str = "/usr/share/mcguffin/config.toml";

// ============== Config Schema ==============

/// Structure returned to the frontend
#[derive(serde::Serialize)]
pub struct ConfigResponse {
    pub server: ServerSection,
    pub admin: AdminSection,
    pub site: SiteSection,
    pub oauth: OAuthSection,
    pub difficulty: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerSection {
    pub site_url: String,
    pub port: u16,
    pub data_file: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AdminSection {
    pub password: String,
    pub display_name: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SiteSection {
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OAuthSection {
    pub cp_client_id: String,
    pub cp_client_secret: String,
}

/// The full config payload from the frontend
#[derive(serde::Deserialize)]
pub struct UpdateConfigPayload {
    pub server: ServerSection,
    pub admin: AdminSection,
    pub site: SiteSection,
    pub oauth: OAuthSection,
    pub difficulty: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
}

fn read_config_raw() -> Result<String, String> {
    std::fs::read_to_string(CONFIG_PATH).map_err(|e| format!("无法读取配置文件: {}", e))
}

fn write_config_raw(content: &str) -> Result<(), String> {
    // Write to temp file first, then atomically rename
    let tmp = format!("{}.tmp", CONFIG_PATH);
    std::fs::write(&tmp, content).map_err(|e| format!("无法写入配置文件: {}", e))?;
    std::fs::rename(&tmp, CONFIG_PATH).map_err(|e| format!("无法更新配置文件: {}", e))?;
    Ok(())
}

fn parse_config(raw: &str) -> Result<ConfigResponse, String> {
    let doc = DocumentMut::from_str(raw).map_err(|e| format!("配置文件格式错误: {}", e))?;

    let get_str = |section: &str, key: &str| -> String {
        doc.get(section)
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default()
    };

    let get_u16 = |section: &str, key: &str| -> u16 {
        doc.get(section)
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_integer())
            .map(|n| n as u16)
            .unwrap_or(3000)
    };

    // Parse difficulty levels (both sub-table [difficulty.Easy] and inline Easy = { ... })
    let mut difficulty = std::collections::HashMap::new();
    if let Some(diff_table) = doc.get("difficulty").and_then(|s| s.as_table()) {
        for (key, item) in diff_table.iter() {
            let fields = if let Some(t) = item.as_table() {
                // Sub-table format: [difficulty.Easy]
                let mut f = std::collections::HashMap::new();
                if let Some(label) = t.get("label").and_then(|v| v.as_str()) {
                    f.insert("label".to_string(), label.to_string());
                }
                if let Some(color) = t.get("color").and_then(|v| v.as_str()) {
                    f.insert("color".to_string(), color.to_string());
                }
                f
            } else if let Some(v) = item.as_value() {
                // Inline table format: Easy = { label = "...", color = "..." }
                let mut f = std::collections::HashMap::new();
                if let Some(inline) = v.as_inline_table() {
                    if let Some(label) = inline.get("label").and_then(|v| v.as_str()) {
                        f.insert("label".to_string(), label.to_string());
                    }
                    if let Some(color) = inline.get("color").and_then(|v| v.as_str()) {
                        f.insert("color".to_string(), color.to_string());
                    }
                }
                f
            } else {
                std::collections::HashMap::new()
            };
            if !fields.is_empty() {
                difficulty.insert(key.to_string(), fields);
            }
        }
    }

    Ok(ConfigResponse {
        server: ServerSection {
            site_url: get_str("server", "site_url"),
            port: get_u16("server", "port"),
            data_file: get_str("server", "data_file"),
        },
        admin: AdminSection {
            password: get_str("admin", "password"),
            display_name: get_str("admin", "display_name"),
        },
        site: SiteSection {
            name: get_str("site", "name"),
        },
        oauth: OAuthSection {
            cp_client_id: get_str("oauth", "cp_client_id"),
            cp_client_secret: get_str("oauth", "cp_client_secret"),
        },
        difficulty,
    })
}

fn apply_config(raw: &str, payload: &UpdateConfigPayload) -> Result<String, String> {
    let mut doc = DocumentMut::from_str(raw).map_err(|e| format!("配置文件格式错误: {}", e))?;

    let set_str = |table: &mut toml_edit::Table, key: &str, value: &str| {
        table[key] = Item::Value(TomlValue::from(value));
    };
    let set_u16 = |table: &mut toml_edit::Table, key: &str, value: u16| {
        table[key] = Item::Value(TomlValue::from(value as i64));
    };

    if let Some(t) = doc.get_mut("server").and_then(|s| s.as_table_mut()) {
        set_str(t, "site_url", &payload.server.site_url);
        set_u16(t, "port", payload.server.port);
        set_str(t, "data_file", &payload.server.data_file);
    }
    if let Some(t) = doc.get_mut("admin").and_then(|s| s.as_table_mut()) {
        set_str(t, "password", &payload.admin.password);
        set_str(t, "display_name", &payload.admin.display_name);
    }
    if let Some(t) = doc.get_mut("site").and_then(|s| s.as_table_mut()) {
        set_str(t, "name", &payload.site.name);
    }
    if let Some(t) = doc.get_mut("oauth").and_then(|s| s.as_table_mut()) {
        set_str(t, "cp_client_id", &payload.oauth.cp_client_id);
        set_str(t, "cp_client_secret", &payload.oauth.cp_client_secret);
    }

    // Write difficulty levels — remove old ones first, then add new
    if let Some(old_diff) = doc.get_mut("difficulty").and_then(|s| s.as_table_mut()) {
        let keys: Vec<String> = old_diff.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys {
            old_diff.remove(&k);
        }
    }
    for (name, fields) in &payload.difficulty {
        let mut level = toml_edit::InlineTable::new();
        level.insert("label", toml_edit::Value::from(fields.get("label").map(|s| s.as_str()).unwrap_or(name)));
        level.insert("color", toml_edit::Value::from(fields.get("color").map(|s| s.as_str()).unwrap_or("#888888")));
        if let Some(t) = doc.get_mut("difficulty").and_then(|s| s.as_table_mut()) {
            t[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(level));
        }
    }

    Ok(doc.to_string())
}

// ============== Endpoints ==============

/// GET /api/admin/config
/// Superadmin only — returns current config as JSON
pub async fn get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let raw = match read_config_raw() {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"success": false, "message": e})),
    };

    let config = match parse_config(&raw) {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"success": false, "message": e})),
    };

    Json(serde_json::json!({"success": true, "config": config}))
}

/// PUT /api/admin/config
/// Superadmin only — updates config.toml
pub async fn update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateConfigPayload>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    // Validate
    if payload.server.site_url.trim().is_empty() {
        return Json(serde_json::json!({"success": false, "message": "站点URL不能为空"}));
    }

    let raw = match read_config_raw() {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"success": false, "message": e})),
    };

    let updated = match apply_config(&raw, &payload) {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"success": false, "message": e})),
    };

    match write_config_raw(&updated) {
        Ok(_) => {
            // Also update in-memory difficulty so it applies immediately
            let mut levels = std::collections::HashMap::new();
            for (name, fields) in &payload.difficulty {
                levels.insert(name.clone(), DifficultyLevel {
                    label: fields.get("label").cloned().unwrap_or_else(|| name.clone()),
                    color: fields.get("color").cloned().unwrap_or_else(|| "#888888".to_string()),
                });
            }
            if !levels.is_empty() {
                *state.difficulty.write().await = crate::types::DifficultyConfig { levels };
            }
            Json(serde_json::json!({"success": true, "message": "配置已保存，立即生效"}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": e})),
    }
}

/// POST /api/admin/restart
/// Superadmin only — restarts the mcguffin service
pub async fn restart_service(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    // Spawn restart in background — this will restart the service after we respond
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = std::process::Command::new("systemctl")
            .arg("restart")
            .arg("mcguffin")
            .status();
    });

    Json(serde_json::json!({"success": true, "message": "服务正在重启..."}))
}

// ============== Data Backup / Restore ==============

/// Get the backup directory path (same parent as data_file)
fn backup_dir(state: &AppState) -> PathBuf {
    let data_path = PathBuf::from(&state.data_file);
    let parent = data_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    parent.join("backups")
}

/// Get backup file entries sorted by name (newest first)
fn list_backup_files(dir: &PathBuf) -> Vec<serde_json::Value> {
    let dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut entries: Vec<_> = dir
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            Some(serde_json::json!({
                "name": name,
                "size": meta.len(),
                "modified": chrono::DateTime::<chrono::Utc>::from(meta.modified().ok()?)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            }))
        })
        .collect();
    // Sort by name descending (newest first)
    entries.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        bn.cmp(an)
    });
    entries
}

/// POST /api/admin/backup
/// Superadmin only — creates a timestamped backup of the current data file
pub async fn create_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    // Save current state to disk first so we back up the latest data
    state.save().await;

    let data_path = PathBuf::from(&state.data_file);
    if !data_path.exists() {
        return Json(serde_json::json!({"success": false, "message": "数据文件不存在，无法备份"}));
    }

    let dir = backup_dir(&state);
    if std::fs::create_dir_all(&dir).is_err() {
        return Json(serde_json::json!({"success": false, "message": "无法创建备份目录"}));
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("mcguffin_data_{}.json", timestamp);
    let backup_path = dir.join(&backup_name);

    match std::fs::copy(&data_path, &backup_path) {
        Ok(_) => {
            tracing::info!("Backup created: {:?}", backup_path);
            Json(serde_json::json!({"success": true, "message": "备份已创建", "backup": backup_name}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": format!("备份失败: {}", e)})),
    }
}

/// GET /api/admin/backups
/// Superadmin only — lists all available backups with size and date
pub async fn list_backups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let dir = backup_dir(&state);
    let backups = list_backup_files(&dir);

    Json(serde_json::json!({"success": true, "backups": backups}))
}

/// POST /api/admin/backup/restore/:name
/// Superadmin only — restores data from a named backup
pub async fn restore_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    // Validate filename — prevent path traversal
    let name_clean = name.trim();
    if name_clean.is_empty()
        || name_clean.contains('/')
        || name_clean.contains('\\')
        || name_clean.contains("..")
    {
        return Json(serde_json::json!({"success": false, "message": "无效的备份文件名"}));
    }
    if !name_clean.ends_with(".json") {
        return Json(serde_json::json!({"success": false, "message": "无效的备份文件格式"}));
    }

    let dir = backup_dir(&state);
    let backup_path = dir.join(name_clean);
    if !backup_path.exists() {
        return Json(serde_json::json!({"success": false, "message": "备份文件不存在"}));
    }

    let data_path = PathBuf::from(&state.data_file);

    // Create a safety backup of current data before overwriting
    let safety_name = format!("pre_restore_{}.json", Local::now().format("%Y%m%d_%H%M%S"));
    let safety_path = dir.join(&safety_name);
    let _ = std::fs::copy(&data_path, &safety_path);

    // Restore by copying backup -> data file
    match std::fs::copy(&backup_path, &data_path) {
        Ok(_) => {
            tracing::info!("Restored from backup: {:?}", backup_path);
            // Reload state from the restored file
            state.reload().await;
            Json(serde_json::json!({"success": true, "message": "数据已恢复，安全备份已创建"}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": format!("恢复失败: {}", e)})),
    }
}

/// DELETE /api/admin/backup/:name
/// Superadmin only — deletes a named backup
pub async fn delete_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    // Validate filename
    let name_clean = name.trim();
    if name_clean.is_empty()
        || name_clean.contains('/')
        || name_clean.contains('\\')
        || name_clean.contains("..")
    {
        return Json(serde_json::json!({"success": false, "message": "无效的备份文件名"}));
    }

    let dir = backup_dir(&state);
    let backup_path = dir.join(name_clean);
    if !backup_path.exists() {
        return Json(serde_json::json!({"success": false, "message": "备份文件不存在"}));
    }

    match std::fs::remove_file(&backup_path) {
        Ok(_) => {
            tracing::info!("Backup deleted: {:?}", backup_path);
            Json(serde_json::json!({"success": true, "message": "备份已删除"}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": format!("删除失败: {}", e)})),
    }
}

// ============== Data / Config Export ==============

/// GET /api/admin/export/data
/// Superadmin only — exports the data file (JSON) as download
pub async fn export_data(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    let data_path = PathBuf::from(&state.data_file);
    match std::fs::read_to_string(&data_path) {
        Ok(content) => {
            let filename = format!("mcguffin_data_{}.json", Local::now().format("%Y%m%d_%H%M%S"));
            Json(serde_json::json!({
                "success": true,
                "content": content,
                "filename": filename,
                "mime": "application/json",
            }))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": format!("读取数据文件失败: {}", e)})),
    }
}

/// GET /api/admin/export/config
/// Superadmin only — exports the config file (TOML) as download
pub async fn export_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (user_id, _) = match resolve_user(&state, &headers).await {
        Some(u) => u,
        None => return Json(serde_json::json!({"success": false, "message": "未登录"})),
    };
    if !is_superadmin(&state, &user_id).await {
        return Json(serde_json::json!({"success": false, "message": "权限不足"}));
    }

    match std::fs::read_to_string(CONFIG_PATH) {
        Ok(content) => {
            let filename = format!("config_{}.toml", Local::now().format("%Y%m%d_%H%M%S"));
            Json(serde_json::json!({
                "success": true,
                "content": content,
                "filename": filename,
                "mime": "text/plain",
            }))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": format!("读取配置文件失败: {}", e)})),
    }
}
