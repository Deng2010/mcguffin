use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Local;
use std::path::PathBuf;
use std::str::FromStr;
use toml_edit::{DocumentMut, Item, Value as TomlValue};

use crate::state::resolve_config_path;
use crate::state::AppState;
use crate::types::{
    ChangeRolePayload, CreateGroupPayload, DifficultyLevel, SetAclPayload, SetProblemAclPayload,
    SetUserGroupsPayload, SetUserPermissionsPayload, ShowcaseConfigPayload, UpdateGroupPayload,
    PERM_WILDCARD,
};
use crate::utils::AuthUser;

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
data_file = "mcguffin_data.json"

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
async fn sync_groups_to_config(state: &AppState) {
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
                data_file: String::new(),
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

    // Validate admin display_name uniqueness: no user's display_name or username matches
    {
        let admin_dn = payload.admin.display_name.trim();
        if !admin_dn.is_empty() {
            let users = state.users.read().await;
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
            Ok(Json(
                serde_json::json!({"success": true, "message": "配置已保存，立即生效"}),
            ))
        }
        Err(e) => Ok(Json(serde_json::json!({"success": false, "message": e}))),
    }
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

// ============== Data Backup / Restore ==============

/// 获取备份目录（与 data_file 同目录下的 backups/）
fn backup_dir(state: &AppState) -> PathBuf {
    // Use db_path for backup dir derivation
    let db_path = std::path::Path::new(&state.data_file).with_extension("db");
    let parent = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    parent.join("backups")
}

/// 获取 SQLite 数据库文件路径（由 data_file 路径推导）
fn db_path_from_state(state: &AppState) -> PathBuf {
    std::path::Path::new(&state.data_file).with_extension("db")
}

/// 获取备份文件列表，按名称降序（最新的在前）
/// 同时支持 .db（SQLite 完整备份）和 .json（JSON 导出）
fn list_backup_files(dir: &PathBuf) -> Vec<serde_json::Value> {
    let dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut entries: Vec<_> = dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json" || ext == "db")
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            let ext = e.path().extension()?.to_str()?.to_string();
            Some(serde_json::json!({
                "name": name,
                "size": meta.len(),
                "type": if ext == "db" { "sqlite" } else { "json" },
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
/// manage_backups permission required — creates timestamped backups (JSON + SQLite)
pub async fn create_backup(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    let dir = backup_dir(&state);
    if std::fs::create_dir_all(&dir).is_err() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无法创建备份目录"}),
        ));
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");

    // 1. 创建 SQLite 在线备份（一致性快照）
    let db_path = db_path_from_state(&state);
    let db_backup_name = format!("mcguffin_data_{}.db", timestamp);
    let db_backup_path = dir.join(&db_backup_name);

    let db_result = if db_path.exists() {
        match crate::db::create_consistent_backup(
            &db_path.to_string_lossy(),
            &db_backup_path.to_string_lossy(),
        ) {
            Ok(()) => {
                tracing::info!("SQLite backup created: {:?}", db_backup_path);
                Some(db_backup_name)
            }
            Err(e) => {
                tracing::warn!("SQLite backup failed (可忽略，JSON 备份仍可用): {}", e);
                None
            }
        }
    } else {
        None
    };

    // 2. 创建 JSON 备份（兼容旧格式，若文件存在）
    let json_data_path = PathBuf::from(&state.data_file);
    let json_backup_name = format!("mcguffin_data_{}.json", timestamp);
    let json_backup_path = dir.join(&json_backup_name);

    let json_result = if json_data_path.exists() {
        match std::fs::copy(&json_data_path, &json_backup_path) {
            Ok(_) => {
                tracing::info!("JSON backup created: {:?}", json_backup_path);
                Some(json_backup_name)
            }
            Err(e) => {
                tracing::warn!("JSON backup failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 3. 返回结果
    let results: Vec<serde_json::Value> = [db_result, json_result]
        .into_iter()
        .flatten()
        .map(|name| {
            serde_json::json!({
                "name": name,
                "type": if name.ends_with(".db") { "sqlite" } else { "json" }
            })
        })
        .collect();

    if results.is_empty() {
        Ok(Json(
            serde_json::json!({"success": false, "message": "备份失败：数据文件和数据库均不可用"}),
        ))
    } else {
        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("已创建 {} 个备份", results.len()),
            "backups": results,
        })))
    }
}

/// GET /api/admin/backups
/// manage_backups permission required — lists all available backups with size and date
pub async fn list_backups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    let dir = backup_dir(&state);
    let backups = list_backup_files(&dir);

    Ok(Json(
        serde_json::json!({"success": true, "backups": backups}),
    ))
}

/// POST /api/admin/backup/restore/:name
/// manage_backups permission required — restores from a named backup (supports .db and .json)
pub async fn restore_backup(
    State(mut state): State<AppState>,
    auth: AuthUser,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    // Validate filename — prevent path traversal
    let name_clean = name.trim();
    if name_clean.is_empty()
        || name_clean.contains('/')
        || name_clean.contains('\\')
        || name_clean.contains("..")
    {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效的备份文件名"}),
        ));
    }
    if !name_clean.ends_with(".json") && !name_clean.ends_with(".db") {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效的备份文件格式，仅支持 .db 或 .json"}),
        ));
    }

    let dir = backup_dir(&state);
    let backup_path = dir.join(name_clean);
    if !backup_path.exists() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "备份文件不存在"}),
        ));
    }

    let is_db_backup = name_clean.ends_with(".db");
    let safety_timestamp = Local::now().format("%Y%m%d_%H%M%S");

    if is_db_backup {
        // ── 从 SQLite .db 文件恢复 ──
        let db_path = db_path_from_state(&state);

        // 1. 创建 SQLite 安全备份
        let safety_name = format!("pre_restore_{}.db", safety_timestamp);
        let safety_path = dir.join(&safety_name);
        if let Err(e) = crate::db::create_consistent_backup(
            &db_path.to_string_lossy(),
            &safety_path.to_string_lossy(),
        ) {
            tracing::warn!("创建安全备份失败: {}", e);
        }

        // 2. 关闭连接池
        state.db.close().await;

        // 3. 用备份文件覆盖主数据库
        if let Err(e) = std::fs::copy(&backup_path, &db_path) {
            match crate::db::init_db(&db_path.to_string_lossy()).await {
                Ok(pool) => state.db = pool,
                Err(_) => {
                    // 文件失败且无法重建连接，只能 panic
                    panic!("恢复失败且无法重建连接");
                }
            }
            return Ok(Json(
                serde_json::json!({"success": false, "message": format!("文件复制失败: {}", e)}),
            ));
        }

        // 清理残留 WAL/SHM 文件
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));

        // 4. 重建连接池
        match crate::db::init_db(&db_path.to_string_lossy()).await {
            Ok(pool) => state.db = pool,
            Err(e) => {
                tracing::error!("重建数据库连接失败: {}", e);
                // 回退到 :memory:
                state.db = crate::db::init_db(":memory:")
                    .await
                    .expect("内存数据库也无法创建");
            }
        }

        // 5. 验证完整性
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&state.db)
            .await
            .unwrap_or_else(|_| "error".to_string());

        if integrity != "ok" {
            tracing::error!("恢复后完整性检查失败: {}", integrity);
            return Ok(Json(
                serde_json::json!({"success": false, "message": format!("数据完整性检查失败: {}，建议手动恢复安全备份 {}", integrity, safety_name)}),
            ));
        }

        // 6. 从 SQLite 重新加载数据到内存
        state.reload().await;

        tracing::info!("Restored from SQLite backup: {:?}", backup_path);
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "SQLite 备份已恢复，安全备份已创建",
            "safety_backup": safety_name,
        })))
    } else {
        // ── 从 JSON .json 文件恢复（旧格式兼容） ──
        let data_path = PathBuf::from(&state.data_file);

        // 创建 JSON 安全备份
        let safety_name = format!("pre_restore_{}.json", safety_timestamp);
        let safety_path = dir.join(&safety_name);
        let _ = std::fs::copy(&data_path, &safety_path);

        match std::fs::copy(&backup_path, &data_path) {
            Ok(_) => {
                tracing::info!("Restored from JSON backup: {:?}", backup_path);
                // 重新导入 SQLite（同步到数据库）
                if let Ok(json) = std::fs::read_to_string(data_path) {
                    if let Ok(saved) = serde_json::from_str::<crate::state::SavedData>(&json) {
                        // 清空 SQLite 并重新导入
                        if let Err(e) = crate::db::reimport_all_data(&state.db, &saved).await {
                            tracing::warn!("JSON 恢复到 SQLite 失败: {}", e);
                        }
                    }
                }
                state.reload().await;
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": "JSON 备份已恢复，安全备份已创建",
                    "safety_backup": safety_name,
                })))
            }
            Err(e) => Ok(Json(
                serde_json::json!({"success": false, "message": format!("恢复失败: {}", e)}),
            )),
        }
    }
}

/// GET /api/admin/backup/download/{name}
/// 下载备份文件。.json 直接返回文本，.db 返回 base64 编码。
pub async fn download_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    let name_clean = name.trim();
    if name_clean.is_empty()
        || name_clean.contains('/')
        || name_clean.contains('\\')
        || name_clean.contains("..")
    {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效的备份文件名"}),
        ));
    }

    let backup_path = backup_dir(&state).join(name_clean);
    if !backup_path.exists() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "备份文件不存在"}),
        ));
    }

    let is_json = name_clean.ends_with(".json");

    if is_json {
        // JSON 文件直接返回文本
        match std::fs::read_to_string(&backup_path) {
            Ok(content) => Ok(Json(serde_json::json!({
                "success": true,
                "content": content,
                "filename": name_clean,
                "mime": "application/json",
            }))),
            Err(e) => Ok(Json(
                serde_json::json!({"success": false, "message": format!("读取备份失败: {}", e)}),
            )),
        }
    } else {
        // .db 文件是二进制，返回 base64 编码
        match std::fs::read(&backup_path) {
            Ok(bytes) => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Ok(Json(serde_json::json!({
                    "success": true,
                    "content": encoded,
                    "filename": name_clean,
                    "mime": "application/octet-stream",
                    "encoding": "base64",
                })))
            }
            Err(e) => Ok(Json(
                serde_json::json!({"success": false, "message": format!("读取备份失败: {}", e)}),
            )),
        }
    }
}

/// DELETE /api/admin/backup/:name
pub async fn delete_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    let name_clean = name.trim();
    if name_clean.is_empty()
        || name_clean.contains('/')
        || name_clean.contains('\\')
        || name_clean.contains("..")
    {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效的备份文件名"}),
        ));
    }

    let dir = backup_dir(&state);
    let backup_path = dir.join(name_clean);
    if !backup_path.exists() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "备份文件不存在"}),
        ));
    }

    match std::fs::remove_file(&backup_path) {
        Ok(_) => {
            tracing::info!("Backup deleted: {:?}", backup_path);
            Ok(Json(
                serde_json::json!({"success": true, "message": "备份已删除"}),
            ))
        }
        Err(e) => Ok(Json(
            serde_json::json!({"success": false, "message": format!("删除失败: {}", e)}),
        )),
    }
}

// ============== Data / Config Export ==============

/// GET /api/admin/export/data
/// manage_site permission required — exports all data as JSON download.
/// 直接从 SQLite 读取并序列化为 JSON，不依赖本地文件。
pub async fn export_data(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE)
        .await?;

    // 直接从 SQLite 导出，不依赖本地 JSON 文件
    let content = crate::db::export_db_to_json_string(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("导出数据失败: {}", e),
                })),
            )
        })?;

    let filename = format!(
        "mcguffin_data_{}.json",
        Local::now().format("%Y%m%d_%H%M%S")
    );
    Ok(Json(serde_json::json!({
        "success": true,
        "content": content,
        "filename": filename,
        "mime": "application/json",
    })))
}

/// GET /api/admin/export/config
/// manage_site permission required — exports the config file (TOML) as download
pub async fn export_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE)
        .await?;

    match std::fs::read_to_string(resolve_config_path()) {
        Ok(content) => {
            let filename = format!("config_{}.toml", Local::now().format("%Y%m%d_%H%M%S"));
            Ok(Json(serde_json::json!({
                "success": true,
                "content": content,
                "filename": filename,
                "mime": "text/plain",
            })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({"success": false, "message": format!("读取配置文件失败: {}", e)}),
        )),
    }
}

// ============== Data / Config Import ==============

/// POST /api/admin/import/data
/// manage_backups permission required — imports data from a JSON string (replaces all data)
pub async fn import_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false, "message": "缺少 content 字段"
                })),
            )
        })?;

    // 尝试两种格式：新格式（数组，由 export 产生）和旧格式（HashMap，由 SavedData 序列化）
    let saved = parse_import_data(content).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false, "message": format!("JSON 解析失败: {}", e)
            })),
        )
    })?;

    // 先创建安全备份
    let safety_filename = format!(
        "pre_import_{}.json",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    if let Ok(json) = std::fs::read_to_string(&state.data_file) {
        let backup_dir = std::path::Path::new(&state.data_file)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let _ = std::fs::write(backup_dir.join(&safety_filename), &json);
    }

    // 清空 SQLite 并重新导入
    crate::db::reimport_all_data(&state.db, &saved)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false, "message": format!("数据导入失败: {}", e)
                })),
            )
        })?;

    // 从 SQLite 重新加载数据到内存
    state.reload().await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "数据已导入，当前数据的备份已保存到 backups/ 目录",
        "safety_backup": safety_filename,
    })))
}

/// POST /api/admin/import/config
/// manage_site permission required — imports config from a TOML string
pub async fn import_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE)
        .await?;

    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false, "message": "缺少 content 字段"
                })),
            )
        })?;

    // 验证 TOML 格式
    let _: toml::Value = toml::from_str(content).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false, "message": format!("TOML 解析失败: {}", e)
            })),
        )
    })?;

    // 创建配置文件的备份
    let config_path = crate::state::resolve_config_path();
    if config_path.exists() {
        let backup_path = config_path.with_extension("toml.bak");
        let _ = std::fs::copy(&config_path, &backup_path);
    }

    // 写入新配置
    std::fs::write(&config_path, content).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false, "message": format!("写入配置文件失败: {}", e)
            })),
        )
    })?;

    tracing::info!("配置文件已更新: {:?}", config_path);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "配置文件已更新，将在服务重启后生效",
        "config_file": config_path.to_string_lossy().to_string(),
    })))
}

// ============== Showcase Configuration ==============

/// GET /api/admin/showcase
/// edit_showcase permission required — returns current showcase selections
pub async fn get_showcase_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::EDIT_SHOWCASE)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "problem_ids": state.showcase_problem_ids.read().await.clone(),
        "contest_ids": state.showcase_contest_ids.read().await.clone(),
    })))
}

/// PUT /api/admin/showcase
/// edit_showcase permission required — updates which problems/contests appear on the showcase
pub async fn update_showcase_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<ShowcaseConfigPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::EDIT_SHOWCASE)
        .await?;

    let problem_ids_json = serde_json::to_string(&payload.problem_ids).unwrap_or_default();
    let contest_ids_json = serde_json::to_string(&payload.contest_ids).unwrap_or_default();
    *state.showcase_problem_ids.write().await = payload.problem_ids;
    *state.showcase_contest_ids.write().await = payload.contest_ids;

    // 同步写入 SQLite meta 表
    let _ =
        sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_problem_ids', ?)")
            .bind(&problem_ids_json)
            .execute(&state.db)
            .await;
    let _ =
        sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_contest_ids', ?)")
            .bind(&contest_ids_json)
            .execute(&state.db)
            .await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "展板配置已保存"}),
    ))
}

// ============== Audit Log ==============

/// GET /api/admin/audit-log
/// view_stats permission required — returns recent permission audit entries
/// Row type for sqlx query_as when reading from audit_log table
#[derive(sqlx::FromRow)]
struct AuditLogRow {
    timestamp: String,
    user_id: String,
    user_name: String,
    action: String,
    resource: String,
    result: String,
    reason: Option<String>,
}

pub async fn get_audit_log(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::VIEW_STATS)
        .await?;

    let rows = sqlx::query_as::<_, AuditLogRow>(
        "SELECT timestamp, user_id, user_name, action, resource, result, reason \
         FROM audit_log ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "数据库查询失败"})),
        )
    })?;

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "timestamp": r.timestamp,
                "user_id": r.user_id,
                "user_name": r.user_name,
                "action": r.action,
                "resource": r.resource,
                "result": r.result,
                "reason": r.reason,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(entries)))
}

// ============== User Management ==============

use crate::state::ADMIN_USER_ID;
/// GET /api/admin/users
/// List all users (superadmin only)
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct AdminUserRow {
    id: String,
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    email: Option<String>,
    role: String,
    team_status: String,
    created_at: String,
    bio: String,
    group_ids: String,
    user_permissions: String,
    is_team_member: bool,
}

pub async fn admin_list_users(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    // Try SQLite first, then fallback to HashMap
    let sql_result = sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.username, u.display_name, u.avatar_url, u.email, u.role, \
         u.team_status, u.created_at, u.bio, u.group_ids, u.user_permissions, \
         CASE WHEN tm.user_id IS NOT NULL THEN 1 ELSE 0 END as is_team_member \
         FROM users u \
         LEFT JOIN team_members tm ON u.id = tm.user_id \
         ORDER BY u.created_at DESC",
    )
    .fetch_all(&state.db)
    .await;

    match sql_result {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "username": r.username,
                        "display_name": r.display_name,
                        "email": r.email,
                        "role": r.role,
                        "team_status": r.team_status,
                        "is_team_member": r.is_team_member,
                        "group_ids": serde_json::from_str::<Vec<String>>(&r.group_ids).unwrap_or_default(),
                        "user_permissions": serde_json::from_str::<Vec<String>>(&r.user_permissions).unwrap_or_default(),
                        "created_at": r.created_at,
                    })
                })
                .collect();
            Ok(Json(serde_json::json!(result)))
        }
        Err(_) => {
            // Fallback to HashMap
            let users = state.users.read().await;
            let members = state.team_members.read().await;
            let result: Vec<serde_json::Value> = users
                .values()
                .map(|u| {
                    let is_team_member = members.values().any(|m| m.user_id == u.id);
                    serde_json::json!({
                        "id": u.id,
                        "username": u.username,
                        "display_name": u.display_name,
                        "email": u.email,
                        "role": u.role,
                        "team_status": u.team_status,
                        "is_team_member": is_team_member,
                        "group_ids": u.group_ids,
                        "user_permissions": u.user_permissions,
                        "created_at": u.created_at,
                    })
                })
                .collect();
            Ok(Json(serde_json::json!(result)))
        }
    }
}

/// POST /api/admin/users/{user_id}/role
/// Change a user's role (superadmin only)
pub async fn admin_change_user_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<ChangeRolePayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    if user_id == ADMIN_USER_ID {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "不能修改系统管理员的角色"}),
        ));
    }
    if payload.role != "admin" && payload.role != "member" && payload.role != "guest" {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效角色"}),
        ));
    }
    if state.users.read().await.contains_key(&user_id) {
        state
            .update_user_field(&user_id, "role", payload.role.clone())
            .await;
        Ok(Json(
            serde_json::json!({"success": true, "message": "角色已更新"}),
        ))
    } else {
        Ok(Json(
            serde_json::json!({"success": false, "message": "用户不存在"}),
        ))
    }
}

/// POST /api/admin/users/{user_id}/remove
/// Remove (delete) a user (superadmin only)
pub async fn admin_remove_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    if user_id == ADMIN_USER_ID {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "不能删除系统管理员"}),
        ));
    }
    if user_id == auth.user_id {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "不能删除自己"}),
        ));
    }
    state.delete_user(&user_id).await;
    state.remove_team_member_by_user(&user_id).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "用户已删除"}),
    ))
}

// ============== Member Groups CRUD ==============

/// GET /api/admin/groups — list all member groups
pub async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let groups = state.member_groups.read().await;
    let result: Vec<serde_json::Value> = groups
        .values()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "name": g.name,
                "permissions": g.permissions,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(result)))
}

/// POST /api/admin/groups — create a member group
pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateGroupPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "组名不能为空"}),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let group = crate::types::MemberGroup {
        id: id.clone(),
        name,
        permissions: payload.permissions,
    };
    state.member_groups.write().await.insert(id.clone(), group);
    sync_groups_to_config(&state).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "成员组已创建", "id": id}),
    ))
}

/// PUT /api/admin/groups/{group_id} — update a member group
pub async fn update_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<String>,
    Json(payload): Json<UpdateGroupPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let mut groups = state.member_groups.write().await;
    let group = match groups.get_mut(&group_id) {
        Some(g) => g,
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "成员组不存在"}),
            ))
        }
    };
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "组名不能为空"}),
        ));
    }
    group.name = name;
    group.permissions = payload.permissions;
    drop(groups);
    sync_groups_to_config(&state).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "成员组已更新"}),
    ))
}

/// DELETE /api/admin/groups/{group_id} — delete a member group
pub async fn delete_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let mut groups = state.member_groups.write().await;
    if !groups.contains_key(&group_id) {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "成员组不存在"}),
        ));
    }
    groups.remove(&group_id);
    drop(groups);

    state.remove_group_from_all_users(&group_id).await;
    sync_groups_to_config(&state).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "成员组已删除"}),
    ))
}

// ============== User-Group Membership ==============

/// PUT /api/admin/users/{user_id}/groups — set user's group membership
pub async fn set_user_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<SetUserGroupsPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let user = match state.users.read().await.get(&user_id) {
        Some(u) => u.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "用户不存在"}),
            ))
        }
    };
    let mut user = user;
    user.group_ids = payload.group_ids;
    state.upsert_user(&user).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "用户组已更新"}),
    ))
}

// ============== User Individual Permissions ==============

/// PUT /api/admin/users/{user_id}/permissions — set user's individual permissions
pub async fn set_user_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(payload): Json<SetUserPermissionsPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let user = match state.users.read().await.get(&user_id) {
        Some(u) => u.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "用户不存在"}),
            ))
        }
    };
    let mut user = user;
    user.user_permissions = payload.permissions;
    state.upsert_user(&user).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "用户权限已更新"}),
    ))
}

// ============== Problem Resource ACL ==============

/// PUT /api/admin/problems/{problem_id}/acl — set who can edit a problem
pub async fn set_problem_acl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(problem_id): Path<String>,
    Json(payload): Json<SetProblemAclPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;
    let mut problem = match state.problems.read().await.get(&problem_id) {
        Some(p) => p.clone(),
        None => {
            return Ok(Json(
                serde_json::json!({"success": false, "message": "题目不存在"}),
            ))
        }
    };
    problem.editable_by = payload.editable_by;
    state.insert_problem(&problem).await;
    Ok(Json(
        serde_json::json!({"success": true, "message": "题目访问控制已更新"}),
    ))
}

// ============== Unified Resource ACL ==============

/// PUT /api/admin/acl/{resource_type}/{resource_id} — set ACL for any resource
/// resource_type: "problem" | "contest" | "post"
pub async fn set_resource_acl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Json(payload): Json<SetAclPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, PERM_WILDCARD).await?;

    {
        let mut found = false;
        match resource_type.as_str() {
            "problem" => {
                if let Some(mut p) = state.problems.read().await.get(&resource_id).cloned() {
                    p.visible_to = payload.visible_to.clone();
                    p.editable_by = payload.editable_by.clone();
                    state.insert_problem(&p).await;
                    found = true;
                }
            }
            "contest" => {
                if let Some(mut c) = state.contests.read().await.get(&resource_id).cloned() {
                    c.visible_to = payload.visible_to.clone();
                    c.editable_by = payload.editable_by.clone();
                    state.insert_contest(&c).await;
                    found = true;
                }
            }
            "post" | "discussion" => {
                if let Some(mut p) = state.posts.read().await.get(&resource_id).cloned() {
                    p.visible_to = payload.visible_to.clone();
                    p.editable_by = payload.editable_by.clone();
                    state.upsert_post(&p).await;
                    found = true;
                }
            }
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"success": false, "message": "无效的资源类型"})),
                ))
            }
        }
        if !found {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "message": "资源不存在"})),
            ));
        }
    } // write locks dropped here

    Ok(Json(
        serde_json::json!({"success": true, "message": "访问控制已更新"}),
    ))
}

/// 解析导入数据，兼容两种格式：
/// - 新格式（数组）：由 export 产生，`users` 等字段是 JSON 数组
/// - 旧格式（HashMap）：`users` 等字段是 `{"id": {...}}`
/// - 旧版本导出的部分字段数据：自动补全缺失字段
fn parse_import_data(content: &str) -> Result<crate::state::SavedData, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("无效的 JSON: {}", e))?;

    // 自动检测并转换数组格式 → HashMap 格式
    let convert = |arr: &mut serde_json::Value, key_field: &str| {
        if let Some(items) = arr.as_array_mut() {
            if items.is_empty() {
                // 空数组 → 空 Map（HashMap 需要 object 而非 array）
                *arr = serde_json::Value::Object(serde_json::Map::new());
            } else if items.iter().any(|v| v.is_object() && v.get(key_field).and_then(|i| i.as_str()).is_some())
            {
                let mut map = serde_json::Map::new();
                for item in items.drain(..) {
                    if let Some(id) = item.get(key_field).and_then(|i| i.as_str()) {
                        map.insert(id.to_string(), item);
                    }
                }
                *arr = serde_json::Value::Object(map);
            }
        }
    };

    if let Some(obj) = value.as_object_mut() {
        // 补全缺失的顶层字段（没有 #[serde(default)] 的 SavedData 字段）
        let required_fields: Vec<(&str, serde_json::Value)> = vec![
            ("users", serde_json::Value::Object(serde_json::Map::new())),
            ("sessions", serde_json::Value::Object(serde_json::Map::new())),
            ("refresh_tokens", serde_json::Value::Object(serde_json::Map::new())),
            ("team_members", serde_json::Value::Object(serde_json::Map::new())),
            ("problems", serde_json::Value::Object(serde_json::Map::new())),
            ("join_requests", serde_json::Value::Object(serde_json::Map::new())),
        ];
        for (field, default_val) in &required_fields {
            if !obj.contains_key(*field) {
                obj.insert(field.to_string(), default_val.clone());
            }
        }

        // sessions 和 refresh_tokens 用 token 作 key
        let array_fields: Vec<(&str, &str)> = vec![
            ("users", "id"),
            ("sessions", "token"),
            ("refresh_tokens", "token"),
            ("team_members", "id"),
            ("join_requests", "id"),
            ("contests", "id"),
            ("problems", "id"),
            ("notifications", "id"),
            ("posts", "id"),
        ];
        for (field, key) in &array_fields {
            if let Some(arr) = obj.get_mut(*field) {
                convert(arr, key);
            }
        }

        // 补全旧版本导出数据缺失的字段（Contest 必须有 start_time/end_time/description/created_by）
        if let Some(contests) = obj.get_mut("contests").and_then(|c| c.as_object_mut()) {
            for contest in contests.values_mut() {
                if let Some(c) = contest.as_object_mut() {
                    if !c.contains_key("start_time") { c.insert("start_time".into(), serde_json::Value::String(String::new())); }
                    if !c.contains_key("end_time") { c.insert("end_time".into(), serde_json::Value::String(String::new())); }
                    if !c.contains_key("description") { c.insert("description".into(), serde_json::Value::String(String::new())); }
                    if !c.contains_key("created_by") { c.insert("created_by".into(), serde_json::Value::String(String::new())); }
                    if !c.contains_key("problem_order") { c.insert("problem_order".into(), serde_json::Value::Array(Vec::new())); }
                    if !c.contains_key("visible_to") { c.insert("visible_to".into(), serde_json::Value::Array(Vec::new())); }
                    if !c.contains_key("editable_by") { c.insert("editable_by".into(), serde_json::Value::Array(Vec::new())); }
                    if !c.contains_key("link") { c.insert("link".into(), serde_json::Value::Null); }
                }
            }
        }

        // 补全旧版本 Problem 缺失字段
        if let Some(problems) = obj.get_mut("problems").and_then(|p| p.as_object_mut()) {
            for problem in problems.values_mut() {
                if let Some(p) = problem.as_object_mut() {
                    if !p.contains_key("contest") { p.insert("contest".into(), serde_json::Value::String(String::new())); }
                    if !p.contains_key("contest_id") { p.insert("contest_id".into(), serde_json::Value::Null); }
                    if !p.contains_key("content") { p.insert("content".into(), serde_json::Value::String(String::new())); }
                    if !p.contains_key("solution") { p.insert("solution".into(), serde_json::Value::Null); }
                    if !p.contains_key("public_at") { p.insert("public_at".into(), serde_json::Value::Null); }
                    if !p.contains_key("claimed_by") { p.insert("claimed_by".into(), serde_json::Value::Null); }
                    if !p.contains_key("verifier_solution") { p.insert("verifier_solution".into(), serde_json::Value::Null); }
                    if !p.contains_key("visible_to") { p.insert("visible_to".into(), serde_json::Value::Array(Vec::new())); }
                    if !p.contains_key("link") { p.insert("link".into(), serde_json::Value::Null); }
                    if !p.contains_key("remark") { p.insert("remark".into(), serde_json::Value::Null); }
                    if !p.contains_key("editable_by") { p.insert("editable_by".into(), serde_json::Value::Array(Vec::new())); }
                }
            }
        }

        // 补全旧版本 Post 缺失字段
        if let Some(posts) = obj.get_mut("posts").and_then(|p| p.as_object_mut()) {
            for post in posts.values_mut() {
                if let Some(p) = post.as_object_mut() {
                    if !p.contains_key("content") { p.insert("content".into(), serde_json::Value::String(String::new())); }
                    if !p.contains_key("updated_at") { p.insert("updated_at".into(), p.get("created_at").cloned().unwrap_or(serde_json::Value::String(String::new()))); }
                    if !p.contains_key("tags") { p.insert("tags".into(), serde_json::Value::Array(Vec::new())); }
                    if !p.contains_key("pinned") { p.insert("pinned".into(), serde_json::Value::Bool(false)); }
                    if !p.contains_key("team_only") { p.insert("team_only".into(), serde_json::Value::Bool(false)); }
                    if !p.contains_key("emoji") { p.insert("emoji".into(), serde_json::Value::Null); }
                    if !p.contains_key("reactions") { p.insert("reactions".into(), serde_json::Value::Array(Vec::new())); }
                    if !p.contains_key("status") { p.insert("status".into(), serde_json::Value::String("normal".into())); }
                    if !p.contains_key("visible_to") { p.insert("visible_to".into(), serde_json::Value::Array(Vec::new())); }
                    if !p.contains_key("editable_by") { p.insert("editable_by".into(), serde_json::Value::Array(Vec::new())); }
                    if !p.contains_key("reply_count") { p.insert("reply_count".into(), serde_json::Value::Number(serde_json::Number::from(0))); }
                    if !p.contains_key("solution") { p.insert("solution".into(), serde_json::Value::String(String::new())); }
                }
            }
        }

        // 补全旧版本 User 缺失字段
        if let Some(users) = obj.get_mut("users").and_then(|u| u.as_object_mut()) {
            for user in users.values_mut() {
                if let Some(u) = user.as_object_mut() {
                    if !u.contains_key("bio") { u.insert("bio".into(), serde_json::Value::String(String::new())); }
                    if !u.contains_key("password_hash") { u.insert("password_hash".into(), serde_json::Value::Null); }
                    if !u.contains_key("effective_role") { u.insert("effective_role".into(), serde_json::Value::String(String::new())); }
                    if !u.contains_key("group_ids") { u.insert("group_ids".into(), serde_json::Value::Array(Vec::new())); }
                    if !u.contains_key("user_permissions") { u.insert("user_permissions".into(), serde_json::Value::Array(Vec::new())); }
                }
            }
        }

        // 补全旧版本 join_requests 缺失字段
        if let Some(requests) = obj.get_mut("join_requests").and_then(|r| r.as_object_mut()) {
            for request in requests.values_mut() {
                if let Some(r) = request.as_object_mut() {
                    if !r.contains_key("user_email") { r.insert("user_email".into(), serde_json::Value::Null); }
                }
            }
        }
    }

    serde_json::from_value(value)
        .map_err(|e| format!("数据格式转换失败: {}", e))
}
