use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::str::FromStr;
use toml_edit::{DocumentMut, Item, Value as TomlValue};

use crate::state::resolve_config_path;
use crate::state::AppState;
use crate::types::DifficultyLevel;
use crate::utils::AuthUser;

// ============== Config Schema ==============

/// Structure returned to the frontend
#[derive(serde::Serialize)]
pub struct ConfigResponse {
    pub server: ServerSection,
    pub admin: AdminSection,
    pub site: SiteSection,
    pub oauth: OAuthSection,
    pub backup: BackupSection,
    pub difficulty: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub discussion_tags:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub discussion_emojis:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub permissions: std::collections::HashMap<String, Vec<String>>,
    #[serde(default)]
    pub groups: Vec<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerSection {
    pub site_url: String,
    pub port: u16,
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BackupSection {
    pub interval_minutes: u64,
    pub retention_count: u64,
    #[serde(default)]
    pub backup_directory: Option<String>,
}

/// The full config payload from the frontend
#[derive(serde::Deserialize)]
pub struct UpdateConfigPayload {
    pub server: ServerSection,
    pub admin: AdminSection,
    pub site: SiteSection,
    pub oauth: OAuthSection,
    pub backup: BackupSection,
    pub difficulty: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub discussion_tags:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub discussion_emojis:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub permissions: std::collections::HashMap<String, Vec<String>>,
    #[serde(default)]
    pub groups: Vec<serde_json::Value>,
}

fn read_config_raw() -> Result<String, String> {
    let path = resolve_config_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => Ok(s),
        Err(_) => {
            // 文件不存在时返回最小默认配置，而不是报错
            Ok(r#"[server]
site_url = "http://localhost:3000"
port = 3000

[admin]
password = "admin123"
display_name = "管理员"

[oauth]
cp_client_id = ""
cp_client_secret = ""
"#
            .to_string())
        }
    }
}

fn write_config_raw(content: &str) -> Result<(), String> {
    let path = resolve_config_path();
    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建配置目录: {}", e))?;
    }
    // Write to temp file first, then atomically rename
    let tmp = format!("{}.tmp", path.display());
    std::fs::write(&tmp, content).map_err(|e| format!("无法写入配置文件: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("无法更新配置文件: {}", e))?;
    Ok(())
}

/// Sync in-memory member_groups to [permissions.groups] in config.toml
pub(super) async fn sync_groups_to_config(state: &AppState) {
    let groups = {
        let mg = state.member_groups.read().await;
        mg.values()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "name": g.name,
                    "permissions": g.permissions,
                })
            })
            .collect::<Vec<_>>()
    };
    if let Ok(raw) = read_config_raw() {
        let mut doc = match DocumentMut::from_str(&raw) {
            Ok(d) => d,
            Err(_) => return,
        };
        // Build a minimal payload with only groups
        let payload = UpdateConfigPayload {
            server: ServerSection {
                site_url: String::new(),
                port: 0,
            },
            admin: AdminSection {
                password: String::new(),
                display_name: String::new(),
            },
            site: SiteSection {
                name: String::new(),
                title: None,
                difficulty_order: vec![],
            },
            oauth: OAuthSection {
                cp_client_id: String::new(),
                cp_client_secret: String::new(),
            },
            backup: BackupSection {
                interval_minutes: 0,
                retention_count: 0,
                backup_directory: None,
            },
            difficulty: std::collections::HashMap::new(),
            discussion_tags: std::collections::HashMap::new(),
            discussion_emojis: std::collections::HashMap::new(),
            permissions: std::collections::HashMap::new(),
            groups,
        };
        // Use existing permissions writing (preserves existing roles)
        if let Some(perms_root) = doc.get_mut("permissions").and_then(|s| s.as_table_mut()) {
            // Clear old groups
            if let Some(groups_t) = perms_root.get_mut("groups").and_then(|s| s.as_table_mut()) {
                let keys: Vec<String> = groups_t.iter().map(|(k, _)| k.to_string()).collect();
                for k in keys {
                    groups_t.remove(&k);
                }
            }
            // Write new groups
            if !payload.groups.is_empty() {
                if perms_root.get("groups").is_none() {
                    perms_root["groups"] = Item::Table(toml_edit::Table::new());
                }
                if let Some(groups_t) = perms_root.get_mut("groups").and_then(|s| s.as_table_mut())
                {
                    for g in &payload.groups {
                        let id = g.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let name = g.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let perms = g.get("permissions").and_then(|v| v.as_array());
                        if id.is_empty() || name.is_empty() {
                            continue;
                        }
                        let mut it = toml_edit::InlineTable::new();
                        it.insert("name", toml_edit::Value::from(name));
                        if let Some(arr) = perms {
                            let t_arr = toml_edit::Array::from_iter(
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(toml_edit::Value::from)),
                            );
                            it.insert("permissions", toml_edit::Value::Array(t_arr));
                        } else {
                            it.insert(
                                "permissions",
                                toml_edit::Value::Array(toml_edit::Array::new()),
                            );
                        }
                        groups_t[id] = Item::Value(toml_edit::Value::InlineTable(it));
                    }
                }
            }
        }
        let _ = write_config_raw(&doc.to_string());
    }
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

    let get_u64 = |section: &str, key: &str, default_val: u64| -> u64 {
        doc.get(section)
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_integer())
            .map(|n| n as u64)
            .unwrap_or(default_val)
    };

    let get_array = |section: &str, key: &str| -> Vec<String> {
        doc.get(section)
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
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
                if let Some(v) = tbl.get("color").and_then(|v| v.as_str()) {
                    fields.insert("color".to_string(), v.to_string());
                }
                if let Some(v) = tbl.get("description").and_then(|v| v.as_str()) {
                    fields.insert("description".to_string(), v.to_string());
                }
            } else if let Some(v) = item.as_value() {
                if let Some(inline) = v.as_inline_table() {
                    if let Some(v) = inline.get("color").and_then(|v| v.as_str()) {
                        fields.insert("color".to_string(), v.to_string());
                    }
                    if let Some(v) = inline.get("description").and_then(|v| v.as_str()) {
                        fields.insert("description".to_string(), v.to_string());
                    }
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
                if let Some(v) = tbl.get("char").and_then(|v| v.as_str()) {
                    fields.insert("char".to_string(), v.to_string());
                }
            } else if let Some(v) = item.as_value() {
                if let Some(inline) = v.as_inline_table() {
                    if let Some(v) = inline.get("char").and_then(|v| v.as_str()) {
                        fields.insert("char".to_string(), v.to_string());
                    }
                }
            }
            if !fields.is_empty() {
                discussion_emojis.insert(key.to_string(), fields);
            }
        }
    }

    // Parse permissions (nested format: [permissions.roles], [permissions.groups])
    // with fallback to flat [permissions] for backward compat
    let mut permissions = std::collections::HashMap::new();
    let mut groups: Vec<serde_json::Value> = Vec::new();

    if let Some(perms_table) = doc.get("permissions").and_then(|s| s.as_table()) {
        // Check if we have nested [permissions.roles] format
        if let Some(roles_table) = perms_table.get("roles").and_then(|s| s.as_table()) {
            for (role, item) in roles_table.iter() {
                let perms: Vec<String> = if let Some(arr) = item.as_array() {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else if let Some(v) = item.as_value() {
                    v.as_str().map(|s| vec![s.to_string()]).unwrap_or_default()
                } else {
                    continue;
                };
                if !perms.is_empty() {
                    permissions.insert(role.to_string(), perms);
                }
            }
        } else {
            // Fallback: flat [permissions] format (old)
            for (role, item) in perms_table.iter() {
                if role == "roles" || role == "groups" {
                    continue;
                }
                let perms: Vec<String> = if let Some(arr) = item.as_array() {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else if let Some(v) = item.as_value() {
                    v.as_str().map(|s| vec![s.to_string()]).unwrap_or_default()
                } else {
                    continue;
                };
                if !perms.is_empty() {
                    permissions.insert(role.to_string(), perms);
                }
            }
        }

        // Parse groups
        if let Some(groups_table) = perms_table.get("groups").and_then(|s| s.as_table()) {
            for (uuid, item) in groups_table.iter() {
                let mut name = String::new();
                let mut group_perms: Vec<String> = Vec::new();
                if let Some(tbl) = item.as_table() {
                    if let Some(n) = tbl.get("name").and_then(|v| v.as_str()) {
                        name = n.to_string();
                    }
                    if let Some(arr) = tbl.get("permissions").and_then(|v| v.as_array()) {
                        group_perms = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                } else if let Some(v) = item.as_value() {
                    if let Some(inline) = v.as_inline_table() {
                        if let Some(n) = inline.get("name").and_then(|v| v.as_str()) {
                            name = n.to_string();
                        }
                        if let Some(arr) = inline.get("permissions").and_then(|v| v.as_array()) {
                            group_perms = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }
                }
                if !name.is_empty() {
                    groups.push(serde_json::json!({
                        "id": uuid,
                        "name": name,
                        "permissions": group_perms,
                    }));
                }
            }
        }
    }

    Ok(ConfigResponse {
        server: ServerSection {
            site_url: get_str("server", "site_url"),
            port: get_u16("server", "port"),
        },
        admin: AdminSection {
            password: get_str("admin", "password"),
            display_name: get_str("admin", "display_name"),
        },
        site: SiteSection {
            name: get_str("site", "name"),
            title: {
                let t = get_str("site", "title");
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            },
            difficulty_order: get_array("site", "difficulty_order"),
        },
        oauth: OAuthSection {
            cp_client_id: get_str("oauth", "cp_client_id"),
            cp_client_secret: get_str("oauth", "cp_client_secret"),
        },
        backup: BackupSection {
            interval_minutes: get_u64("backup", "interval_minutes", 60),
            retention_count: get_u64("backup", "retention_count", 48),
            backup_directory: {
                let v = get_str("backup", "backup_directory");
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
        },
        difficulty,
        discussion_tags,
        discussion_emojis,
        permissions,
        groups,
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
    let set_u64 = |table: &mut toml_edit::Table, key: &str, value: u64| {
        table[key] = Item::Value(TomlValue::from(value as i64));
    };

    if let Some(t) = doc.get_mut("server").and_then(|s| s.as_table_mut()) {
        set_str(t, "site_url", &payload.server.site_url);
        set_u16(t, "port", payload.server.port);
    }
    if let Some(t) = doc.get_mut("admin").and_then(|s| s.as_table_mut()) {
        set_str(t, "password", &payload.admin.password);
        set_str(t, "display_name", &payload.admin.display_name);
    }
    if let Some(t) = doc.get_mut("site").and_then(|s| s.as_table_mut()) {
        set_str(t, "name", &payload.site.name);
        match &payload.site.title {
            Some(title) if !title.trim().is_empty() => set_str(t, "title", title.trim()),
            _ => {
                t.remove("title");
            }
        }
        // Write difficulty_order array
        if payload.site.difficulty_order.is_empty() {
            t.remove("difficulty_order");
        } else {
            let arr = toml_edit::Array::from_iter(
                payload
                    .site
                    .difficulty_order
                    .iter()
                    .map(|s| toml_edit::Value::from(s.as_str())),
            );
            t["difficulty_order"] = Item::Value(toml_edit::Value::Array(arr));
        }
    }
    if let Some(t) = doc.get_mut("oauth").and_then(|s| s.as_table_mut()) {
        set_str(t, "cp_client_id", &payload.oauth.cp_client_id);
        set_str(t, "cp_client_secret", &payload.oauth.cp_client_secret);
    }

    // Write backup section
    if !doc.contains_key("backup") {
        doc["backup"] = Item::Table(toml_edit::Table::new());
    }
    if let Some(t) = doc.get_mut("backup").and_then(|s| s.as_table_mut()) {
        set_u64(t, "interval_minutes", payload.backup.interval_minutes);
        set_u64(t, "retention_count", payload.backup.retention_count);
        if let Some(dir) = &payload.backup.backup_directory {
            if !dir.trim().is_empty() {
                set_str(t, "backup_directory", dir.trim());
            } else {
                t.remove("backup_directory");
            }
        } else {
            t.remove("backup_directory");
        }
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
        level.insert(
            "label",
            toml_edit::Value::from(fields.get("label").map(|s| s.as_str()).unwrap_or(name)),
        );
        level.insert(
            "color",
            toml_edit::Value::from(fields.get("color").map(|s| s.as_str()).unwrap_or("#888888")),
        );
        if let Some(t) = doc.get_mut("difficulty").and_then(|s| s.as_table_mut()) {
            t[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(level));
        }
    }

    // Write discussion_tags — remove old, add new
    // 确保段存在（默认配置模板可能没有此段）
    if !doc.contains_key("discussion_tags") {
        doc["discussion_tags"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    if let Some(t) = doc
        .get_mut("discussion_tags")
        .and_then(|s| s.as_table_mut())
    {
        let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys {
            t.remove(&k);
        }
    }
    for (name, fields) in &payload.discussion_tags {
        let mut it = toml_edit::InlineTable::new();
        if let Some(v) = fields.get("color") {
            it.insert("color", toml_edit::Value::from(v.as_str()));
        }
        if let Some(v) = fields.get("description") {
            it.insert("description", toml_edit::Value::from(v.as_str()));
        }
        if let Some(t) = doc
            .get_mut("discussion_tags")
            .and_then(|s| s.as_table_mut())
        {
            t[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(it));
        }
    }

    // Write discussion_emojis — remove old, add new
    // 确保段存在
    if !doc.contains_key("discussion_emojis") {
        doc["discussion_emojis"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    if let Some(t) = doc
        .get_mut("discussion_emojis")
        .and_then(|s| s.as_table_mut())
    {
        let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys {
            t.remove(&k);
        }
    }
    for (name, fields) in &payload.discussion_emojis {
        let mut it = toml_edit::InlineTable::new();
        if let Some(v) = fields.get("char") {
            it.insert("char", toml_edit::Value::from(v.as_str()));
        }
        if let Some(t) = doc
            .get_mut("discussion_emojis")
            .and_then(|s| s.as_table_mut())
        {
            t[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(it));
        }
    }

    // Write permissions — remove old, add new in nested [permissions.roles] format
    // Ensure [permissions] table exists
    if doc.get("permissions").is_none() {
        doc["permissions"] = Item::Table(toml_edit::Table::new());
    }
    let perms_root = doc["permissions"].as_table_mut().unwrap();
    {
        // Clear old roles
        if let Some(roles_t) = perms_root.get_mut("roles").and_then(|s| s.as_table_mut()) {
            let keys: Vec<String> = roles_t.iter().map(|(k, _)| k.to_string()).collect();
            for k in keys {
                roles_t.remove(&k);
            }
        }
        // Write roles
        if !payload.permissions.is_empty() {
            // Ensure "roles" sub-table exists
            if perms_root.get("roles").is_none() {
                perms_root["roles"] = Item::Table(toml_edit::Table::new());
            }
            if let Some(roles_t) = perms_root.get_mut("roles").and_then(|s| s.as_table_mut()) {
                for (role, perms) in &payload.permissions {
                    if !perms.is_empty() {
                        let arr = toml_edit::Array::from_iter(
                            perms.iter().map(|p| toml_edit::Value::from(p.as_str())),
                        );
                        roles_t[role] = Item::Value(toml_edit::Value::Array(arr));
                    }
                }
            }
        }

        // Clear old groups
        if let Some(groups_t) = perms_root.get_mut("groups").and_then(|s| s.as_table_mut()) {
            let keys: Vec<String> = groups_t.iter().map(|(k, _)| k.to_string()).collect();
            for k in keys {
                groups_t.remove(&k);
            }
        }
        // Write groups
        if !payload.groups.is_empty() {
            if perms_root.get("groups").is_none() {
                perms_root["groups"] = Item::Table(toml_edit::Table::new());
            }
            if let Some(groups_t) = perms_root.get_mut("groups").and_then(|s| s.as_table_mut()) {
                for g in &payload.groups {
                    let id = g.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = g.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let perms = g.get("permissions").and_then(|v| v.as_array());
                    if id.is_empty() || name.is_empty() {
                        continue;
                    }
                    let mut it = toml_edit::InlineTable::new();
                    it.insert("name", toml_edit::Value::from(name));
                    if let Some(arr) = perms {
                        let t_arr = toml_edit::Array::from_iter(
                            arr.iter()
                                .filter_map(|v| v.as_str().map(toml_edit::Value::from)),
                        );
                        it.insert("permissions", toml_edit::Value::Array(t_arr));
                    } else {
                        it.insert(
                            "permissions",
                            toml_edit::Value::Array(toml_edit::Array::new()),
                        );
                    }
                    groups_t[id] = Item::Value(toml_edit::Value::InlineTable(it));
                }
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
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE)
        .await?;

    let raw = match read_config_raw() {
        Ok(s) => s,
        Err(e) => return Ok(Json(serde_json::json!({"success": false, "message": e}))),
    };

    let config = match parse_config(&raw) {
        Ok(c) => c,
        Err(e) => return Ok(Json(serde_json::json!({"success": false, "message": e}))),
    };

    Ok(Json(serde_json::json!({"success": true, "config": config})))
}

/// PUT /api/admin/config
/// manage_site permission required — updates config.toml
pub async fn update_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdateConfigPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE)
        .await?;

    // Validate
    if payload.server.site_url.trim().is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "站点URL不能为空"}),
        ));
    }

    // Validate admin password is not empty
    if payload.admin.password.trim().is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "管理员密码不能为空"}),
        ));
    }

    // Validate admin display_name uniqueness: no user's display_name or username matches
    {
        let admin_dn = payload.admin.display_name.trim();
        if !admin_dn.is_empty() {
            let users = state.users.lock().await;
            let is_taken = users
                .values()
                .any(|u| u.id != "admin" && (u.display_name == admin_dn || u.username == admin_dn));
            if is_taken {
                return Ok(Json(serde_json::json!({
                    "success": false,
                    "message": "管理员显示名称已被其他人使用"
                })));
            }
        }
    }

    let raw = match read_config_raw() {
        Ok(s) => s,
        Err(e) => return Ok(Json(serde_json::json!({"success": false, "message": e}))),
    };

    let updated = match apply_config(&raw, &payload) {
        Ok(s) => s,
        Err(e) => return Ok(Json(serde_json::json!({"success": false, "message": e}))),
    };

    match write_config_raw(&updated) {
        Ok(_) => {
            // Also update in-memory difficulty so it applies immediately
            let mut levels = std::collections::HashMap::new();
            for (name, fields) in &payload.difficulty {
                levels.insert(
                    name.clone(),
                    DifficultyLevel {
                        label: fields.get("label").cloned().unwrap_or_else(|| name.clone()),
                        color: fields
                            .get("color")
                            .cloned()
                            .unwrap_or_else(|| "#888888".to_string()),
                    },
                );
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
                    tags.insert(
                        name.clone(),
                        crate::types::DiscussionTag {
                            id: name.clone(),
                            name: name.clone(),
                            color: fields
                                .get("color")
                                .cloned()
                                .unwrap_or_else(|| "#888888".to_string()),
                            description: fields.get("description").cloned().unwrap_or_default(),
                            admin_only: fields
                                .get("admin_only")
                                .and_then(|v| v.parse::<bool>().ok())
                                .unwrap_or(false),
                        },
                    );
                }
            }
            {
                let mut emojis = state.discussion_emojis.write().await;
                emojis.clear();
                for (name, fields) in &payload.discussion_emojis {
                    if let Some(ch) = fields.get("char") {
                        emojis.insert(
                            name.clone(),
                            crate::types::DiscussionEmoji {
                                id: name.clone(),
                                name: name.clone(),
                                char: ch.clone(),
                            },
                        );
                    }
                }
            }
            // Reload role→permissions in-memory
            if !payload.permissions.is_empty() {
                *state.role_permissions.write().await = payload.permissions.clone();
            }
            // Sync member_groups in-memory from payload.groups
            if !payload.groups.is_empty() {
                let mut mg = state.member_groups.write().await;
                mg.clear();
                for g in &payload.groups {
                    let id = g
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = g
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let perms: Vec<String> = g
                        .get("permissions")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if !id.is_empty() && !name.is_empty() {
                        mg.insert(
                            id.clone(),
                            crate::types::MemberGroup {
                                id,
                                name,
                                permissions: perms,
                            },
                        );
                    }
                }
            }
            // Update in-memory backup_directory immediately if changed
            *state.backup_directory.write().await = payload.backup.backup_directory.clone();
            Ok(Json(
                serde_json::json!({"success": true, "message": "配置已保存，立即生效"}),
            ))
        }
        Err(e) => Ok(Json(serde_json::json!({"success": false, "message": e}))),
    }
}

// ============== Admin Initialization ==============

#[derive(serde::Deserialize)]
pub struct InitAdminPayload {
    pub display_name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub password: String,
}

/// GET /api/admin/init-status
/// No auth required — returns whether admin initialization is needed.
pub async fn init_admin_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // Check if config admin.password is set and non-empty
    let raw = read_config_raw().unwrap_or_default();
    let config_pw_set = {
        let doc = raw.parse::<toml_edit::DocumentMut>().ok();
        doc.and_then(|d| {
            d.get("admin")
                .and_then(|s| s.get("password"))
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
        })
        .unwrap_or(false)
    };

    // Check if admin user has password_hash set
    let user_has_hash = {
        let users = state.users.lock().await;
        users
            .get(crate::state::ADMIN_USER_ID)
            .and_then(|u| u.password_hash.as_ref())
            .map(|h| !h.is_empty())
            .unwrap_or(false)
    };

    let initialized = config_pw_set || user_has_hash;

    Json(serde_json::json!({"initialized": initialized}))
}

/// POST /api/admin/init
/// No auth required, only works when NOT initialized.
/// Sets up the admin user with a password and display name.
pub async fn init_admin(
    State(state): State<AppState>,
    Json(payload): Json<InitAdminPayload>,
) -> Json<serde_json::Value> {
    // Check not already initialized
    let raw = read_config_raw().unwrap_or_default();
    let already_initialized = {
        let doc = raw.parse::<toml_edit::DocumentMut>().ok();
        let config_pw = doc
            .and_then(|d| {
                d.get("admin")
                    .and_then(|s| s.get("password"))
                    .and_then(|v| v.as_str())
                    .map(|s| !s.trim().is_empty())
            })
            .unwrap_or(false);
        if config_pw {
            true
        } else {
            let users = state.users.lock().await;
            users
                .get(crate::state::ADMIN_USER_ID)
                .and_then(|u| u.password_hash.as_ref())
                .map(|h| !h.is_empty())
                .unwrap_or(false)
        }
    };

    if already_initialized {
        return Json(serde_json::json!({
            "success": false,
            "message": "管理员已初始化，无法重复初始化"
        }));
    }

    // Validate password and display_name
    if payload.password.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "message": "密码不能为空"
        }));
    }
    if payload.display_name.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "message": "显示名称不能为空"
        }));
    }

    // Hash password
    let password_hash = match bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "message": format!("密码加密失败: {}", e)
            }));
        }
    };

    // Read config, update admin.password + admin.display_name, write config
    let updated_config = {
        let mut doc = match raw.parse::<toml_edit::DocumentMut>() {
            Ok(d) => d,
            Err(e) => {
                return Json(serde_json::json!({
                    "success": false,
                    "message": format!("配置文件格式错误: {}", e)
                }));
            }
        };
        if let Some(t) = doc.get_mut("admin").and_then(|s| s.as_table_mut()) {
            t["password"] = toml_edit::Item::Value(toml_edit::Value::from(&payload.password));
            t["display_name"] =
                toml_edit::Item::Value(toml_edit::Value::from(payload.display_name.trim()));
        }
        doc.to_string()
    };

    if let Err(e) = write_config_raw(&updated_config) {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("写入配置文件失败: {}", e)
        }));
    }

    // Update admin user in memory and SQLite
    {
        let mut users = state.users.lock().await;
        if let Some(admin_user) = users.get_mut(crate::state::ADMIN_USER_ID) {
            admin_user.display_name = payload.display_name.trim().to_string();
            if let Some(url) = &payload.avatar_url {
                if !url.trim().is_empty() {
                    admin_user.avatar_url = Some(url.trim().to_string());
                }
            }
            admin_user.password_hash = Some(password_hash.clone());
            // Clone for SQLite update
            let updated_user = admin_user.clone();
            // Release lock before SQLite call
            drop(users);

            // Write to SQLite using upsert_user
            state.upsert_user(&updated_user).await;
        } else {
            drop(users);
            return Json(serde_json::json!({
                "success": false,
                "message": "系统错误：找不到管理员用户"
            }));
        }
    }

    // Update in-memory state.admin_password for immediate effect
    // Arc<RwLock> allows cross-clone sharing
    *state.admin_password.write().await = payload.password.clone();

    // Ensure admin is a team member
    {
        let members = state.team_members.read().await;
        if !members.values().any(|m| m.user_id == crate::state::ADMIN_USER_ID) {
            drop(members);
            state.insert_team_member(&crate::types::TeamMember {
                id: crate::state::ADMIN_USER_ID.to_string(),
                user_id: crate::state::ADMIN_USER_ID.to_string(),
                joined_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            }).await;
        }
    }
    
    Json(serde_json::json!({
        "success": true,
        "message": "管理员已初始化"
    }))
}

/// POST /api/admin/restart
/// manage_site permission required — restarts the mcguffin service
pub async fn restart_service(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE)
        .await?;

    // Spawn restart in background — this will restart the service after we respond
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = std::process::Command::new("systemctl")
            .arg("restart")
            .arg("mcguffin")
            .status();
    });

    Ok(Json(
        serde_json::json!({"success": true, "message": "服务正在重启..."}),
    ))
}
