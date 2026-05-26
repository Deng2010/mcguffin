use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::path::PathBuf;
use std::str::FromStr;
use toml_edit::{DocumentMut, Item, Value as TomlValue};
use chrono::Local;

use crate::req_perm;
use crate::state::AppState;
use crate::types::{AuditEntry, ChangeRolePayload, DifficultyLevel, ShowcaseConfigPayload, PERM_WILDCARD};

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
    #[serde(default)]
    pub discussion_tags: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub discussion_emojis: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub permissions: std::collections::HashMap<String, Vec<String>>,
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
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub difficulty_order: Vec<String>,
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
    #[serde(default)]
    pub discussion_tags: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub discussion_emojis: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub permissions: std::collections::HashMap<String, Vec<String>>,
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

    let get_array = |section: &str, key: &str| -> Vec<String> {
        doc.get(section)
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default()
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

    // Parse discussion_tags
    let mut discussion_tags = std::collections::HashMap::new();
    if let Some(t) = doc.get("discussion_tags").and_then(|s| s.as_table()) {
        for (key, item) in t.iter() {
            let mut fields = std::collections::HashMap::new();
            if let Some(tbl) = item.as_table() {
                if let Some(v) = tbl.get("color").and_then(|v| v.as_str()) { fields.insert("color".to_string(), v.to_string()); }
                if let Some(v) = tbl.get("description").and_then(|v| v.as_str()) { fields.insert("description".to_string(), v.to_string()); }
            } else if let Some(v) = item.as_value() {
                if let Some(inline) = v.as_inline_table() {
                    if let Some(v) = inline.get("color").and_then(|v| v.as_str()) { fields.insert("color".to_string(), v.to_string()); }
                    if let Some(v) = inline.get("description").and_then(|v| v.as_str()) { fields.insert("description".to_string(), v.to_string()); }
                }
            }
            if !fields.is_empty() {
                discussion_tags.insert(key.to_string(), fields);
            }
        }
    }

    // Parse discussion_emojis
    let mut discussion_emojis = std::collections::HashMap::new();
    if let Some(t) = doc.get("discussion_emojis").and_then(|s| s.as_table()) {
        for (key, item) in t.iter() {
            let mut fields = std::collections::HashMap::new();
            if let Some(tbl) = item.as_table() {
                if let Some(v) = tbl.get("char").and_then(|v| v.as_str()) { fields.insert("char".to_string(), v.to_string()); }
            } else if let Some(v) = item.as_value() {
                if let Some(inline) = v.as_inline_table() {
                    if let Some(v) = inline.get("char").and_then(|v| v.as_str()) { fields.insert("char".to_string(), v.to_string()); }
                }
            }
            if !fields.is_empty() {
                discussion_emojis.insert(key.to_string(), fields);
            }
        }
    }

    // Parse permissions (role_name → [permissions])
    let mut permissions = std::collections::HashMap::new();
    if let Some(t) = doc.get("permissions").and_then(|s| s.as_table()) {
        for (role, item) in t.iter() {
            let perms: Vec<String> = if let Some(arr) = item.as_array() {
                arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
            } else if let Some(v) = item.as_value() {
                // Single string value (e.g., "*")
                v.as_str().map(|s| vec![s.to_string()]).unwrap_or_default()
            } else {
                continue;
            };
            if !perms.is_empty() {
                permissions.insert(role.to_string(), perms);
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
            title: {
                let t = get_str("site", "title");
                if t.is_empty() { None } else { Some(t) }
            },
            difficulty_order: get_array("site", "difficulty_order"),
        },
        oauth: OAuthSection {
            cp_client_id: get_str("oauth", "cp_client_id"),
            cp_client_secret: get_str("oauth", "cp_client_secret"),
        },
        difficulty,
        discussion_tags,
        discussion_emojis,
        permissions,
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
        match &payload.site.title {
            Some(title) if !title.trim().is_empty() => set_str(t, "title", title.trim()),
            _ => { t.remove("title"); }
        }
        // Write difficulty_order array
        if payload.site.difficulty_order.is_empty() {
            t.remove("difficulty_order");
        } else {
            let arr = toml_edit::Array::from_iter(payload.site.difficulty_order.iter().map(|s| toml_edit::Value::from(s.as_str())));
            t["difficulty_order"] = Item::Value(toml_edit::Value::Array(arr));
        }
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

    // Write discussion_tags — remove old, add new
    if let Some(t) = doc.get_mut("discussion_tags").and_then(|s| s.as_table_mut()) {
        let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys { t.remove(&k); }
    }
    for (name, fields) in &payload.discussion_tags {
        let mut it = toml_edit::InlineTable::new();
        if let Some(v) = fields.get("color") { it.insert("color", toml_edit::Value::from(v.as_str())); }
        if let Some(v) = fields.get("description") { it.insert("description", toml_edit::Value::from(v.as_str())); }
        if let Some(t) = doc.get_mut("discussion_tags").and_then(|s| s.as_table_mut()) {
            t[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(it));
        }
    }

    // Write discussion_emojis — remove old, add new
    if let Some(t) = doc.get_mut("discussion_emojis").and_then(|s| s.as_table_mut()) {
        let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys { t.remove(&k); }
    }
    for (name, fields) in &payload.discussion_emojis {
        let mut it = toml_edit::InlineTable::new();
        if let Some(v) = fields.get("char") { it.insert("char", toml_edit::Value::from(v.as_str())); }
        if let Some(t) = doc.get_mut("discussion_emojis").and_then(|s| s.as_table_mut()) {
            t[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(it));
        }
    }

    // Write permissions — remove old, add new
    if let Some(t) = doc.get_mut("permissions").and_then(|s| s.as_table_mut()) {
        let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys { t.remove(&k); }
    }
    for (role, perms) in &payload.permissions {
        if !perms.is_empty() {
            let arr = toml_edit::Array::from_iter(perms.iter().map(|p| toml_edit::Value::from(p.as_str())));
            if let Some(t) = doc.get_mut("permissions").and_then(|s| s.as_table_mut()) {
                t[role] = toml_edit::Item::Value(toml_edit::Value::Array(arr));
            }
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
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

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
/// manage_site permission required — updates config.toml
pub async fn update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateConfigPayload>,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

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
            // Also update difficulty_order in-memory so it applies immediately
            if !payload.site.difficulty_order.is_empty() {
                *state.difficulty_order.write().await = payload.site.difficulty_order.clone();
            }
            // Update in-memory discussion_tags and discussion_emojis immediately
            {
                let mut tags = state.discussion_tags.write().await;
                tags.clear();
                for (name, fields) in &payload.discussion_tags {
            tags.insert(name.clone(), crate::types::DiscussionTag {
                id: name.clone(),
                name: name.clone(),
                color: fields.get("color").cloned().unwrap_or_else(|| "#888888".to_string()),
                description: fields.get("description").cloned().unwrap_or_default(),
                admin_only: fields.get("admin_only").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false),
            });
                }
            }
            {
                let mut emojis = state.discussion_emojis.write().await;
                emojis.clear();
                for (name, fields) in &payload.discussion_emojis {
                    if let Some(ch) = fields.get("char") {
                        emojis.insert(name.clone(), crate::types::DiscussionEmoji {
                            id: name.clone(),
                            name: name.clone(),
                            char: ch.clone(),
                        });
                    }
                }
            }
            // Reload role→permissions in-memory
            if !payload.permissions.is_empty() {
                *state.role_permissions.write().await = payload.permissions.clone();
            }
            Json(serde_json::json!({"success": true, "message": "配置已保存，立即生效"}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": e})),
    }
}

/// POST /api/admin/restart
/// manage_site permission required — restarts the mcguffin service
pub async fn restart_service(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

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
/// manage_backups permission required — creates a timestamped backup of the current data file
pub async fn create_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_BACKUPS);

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
/// manage_backups permission required — lists all available backups with size and date
pub async fn list_backups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_BACKUPS);

    let dir = backup_dir(&state);
    let backups = list_backup_files(&dir);

    Json(serde_json::json!({"success": true, "backups": backups}))
}

/// POST /api/admin/backup/restore/:name
/// manage_backups permission required — restores data from a named backup
pub async fn restore_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_BACKUPS);

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
/// manage_backups permission required — deletes a named backup
pub async fn delete_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_BACKUPS);

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
/// manage_site permission required — exports the data file (JSON) as download
pub async fn export_data(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

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
/// manage_site permission required — exports the config file (TOML) as download
pub async fn export_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

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

// ============== Showcase Configuration ==============

/// GET /api/admin/showcase
/// manage_site permission required — returns current showcase selections
pub async fn get_showcase_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

    Json(serde_json::json!({
        "success": true,
        "problem_ids": state.showcase_problem_ids.read().await.clone(),
        "contest_ids": state.showcase_contest_ids.read().await.clone(),
    }))
}

/// PUT /api/admin/showcase
/// manage_site permission required — updates which problems/contests appear on the showcase
pub async fn update_showcase_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ShowcaseConfigPayload>,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::MANAGE_SITE);

    *state.showcase_problem_ids.write().await = payload.problem_ids;
    *state.showcase_contest_ids.write().await = payload.contest_ids;
    state.save().await;

    Json(serde_json::json!({"success": true, "message": "展板配置已保存"}))
}

// ============== Audit Log ==============

/// GET /api/admin/audit-log
/// view_stats permission required — returns recent permission audit entries
pub async fn get_audit_log(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, crate::types::perms::VIEW_STATS);

    let log = state.audit_log.read().await;
    let entries: Vec<&AuditEntry> = log.iter().rev().take(200).collect();
    Json(serde_json::json!(entries))
}

// ============== User Management ==============

use crate::state::ADMIN_USER_ID;
/// GET /api/admin/users
/// List all users (superadmin only)
pub async fn admin_list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    let (_user_id, _) = req_perm!(&state, &headers, PERM_WILDCARD);
    let users = state.users.read().await;
    let members = state.team_members.read().await;
    let result: Vec<serde_json::Value> = users.values().map(|u| {
        let is_team_member = members.values().any(|m| m.user_id == u.id);
        serde_json::json!({
            "id": u.id,
            "username": u.username,
            "display_name": u.display_name,
            "email": u.email,
            "role": u.role,
            "team_status": u.team_status,
            "is_team_member": is_team_member,
            "created_at": u.created_at,
        })
    }).collect();
    Json(serde_json::json!(result))
}

/// POST /api/admin/users/{user_id}/role
/// Change a user's role (superadmin only)
pub async fn admin_change_user_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<ChangeRolePayload>,
) -> Json<serde_json::Value> {
    let (_admin_id, _) = req_perm!(&state, &headers, PERM_WILDCARD);
    if user_id == ADMIN_USER_ID {
        return Json(serde_json::json!({"success": false, "message": "不能修改系统管理员的角色"}));
    }
    if payload.role != "admin" && payload.role != "member" && payload.role != "guest" {
        return Json(serde_json::json!({"success": false, "message": "无效角色"}));
    }
    let mut users = state.users.write().await;
    if let Some(u) = users.get_mut(&user_id) {
        u.role = payload.role.clone();
        drop(users);
        state.save().await;
        Json(serde_json::json!({"success": true, "message": "角色已更新"}))
    } else {
        Json(serde_json::json!({"success": false, "message": "用户不存在"}))
    }
}

/// POST /api/admin/users/{user_id}/remove
/// Remove (delete) a user (superadmin only)
pub async fn admin_remove_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Json<serde_json::Value> {
    let (_admin_id, _) = req_perm!(&state, &headers, PERM_WILDCARD);
    if user_id == ADMIN_USER_ID {
        return Json(serde_json::json!({"success": false, "message": "不能删除系统管理员"}));
    }
    if user_id == _admin_id {
        return Json(serde_json::json!({"success": false, "message": "不能删除自己"}));
    }
    state.users.write().await.remove(&user_id);
    state.team_members.write().await.retain(|_, m| m.user_id != user_id);
    state.save().await;
    Json(serde_json::json!({"success": true, "message": "用户已删除"}))
}
