use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Local;

use crate::db::{create_consistent_backup, export_db_to_json_string, reimport_all_data};
use crate::infra::persistence::SavedData;
use crate::state::resolve_config_path;
use crate::state::AppState;
use crate::utils::AuthUser;

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
    let content = export_db_to_json_string(&state.db)
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

/// GET /api/admin/export/db
/// manage_site permission required — exports the SQLite database as .db download.
pub async fn export_db(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_SITE).await?;

    // Sync HashMap → SQLite first so the export is up-to-date
    state.sync_to_db().await;

    match std::fs::read(&state.db_path) {
        Ok(bytes) => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let filename = format!(
                "mcguffin_data_{}.db",
                Local::now().format("%Y%m%d_%H%M%S")
            );
            Ok(Json(serde_json::json!({
                "success": true,
                "content": encoded,
                "filename": filename,
                "mime": "application/octet-stream",
                "encoding": "base64",
            })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({"success": false, "message": format!("读取数据库文件失败: {}", e)}),
        )),
    }
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
        "pre_import_{}.db",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let backup_dir = std::path::Path::new(&state.db_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("backups");
    let _ = std::fs::create_dir_all(&backup_dir);
    // 创建 SQLite 安全备份
    let _ = create_consistent_backup(
        &state.db_path,
        &backup_dir.join(&safety_filename).to_string_lossy(),
    );

    // 清空 SQLite 并重新导入
    reimport_all_data(&state.db, &saved)
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
    let config_path = resolve_config_path();
    if config_path.exists() {
        let backup_path = config_path.with_extension("toml.bak");
        let _ = std::fs::copy(&config_path, &backup_path);
    }

    // 确保配置文件的父目录存在
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("无法创建配置文件目录: {}", e),
                })),
            )
        })?;
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

/// 解析导入数据，兼容两种格式：
/// - 新格式（数组）：由 export 产生，`users` 等字段是 JSON 数组
/// - 旧格式（HashMap）：`users` 等字段是 `{"id": {...}}`
/// - 旧版本导出的部分字段数据：自动补全缺失字段
fn parse_import_data(content: &str) -> Result<SavedData, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("无效的 JSON: {}", e))?;

    // 自动检测并转换数组格式 → HashMap 格式
    let convert = |arr: &mut serde_json::Value, key_field: &str| {
        if let Some(items) = arr.as_array_mut() {
            if items.is_empty() {
                // 空数组 → 空 Map（HashMap 需要 object 而非 array）
                *arr = serde_json::Value::Object(serde_json::Map::new());
            } else if items
                .iter()
                .any(|v| v.is_object() && v.get(key_field).and_then(|i| i.as_str()).is_some())
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
            (
                "sessions",
                serde_json::Value::Object(serde_json::Map::new()),
            ),
            (
                "refresh_tokens",
                serde_json::Value::Object(serde_json::Map::new()),
            ),
            (
                "team_members",
                serde_json::Value::Object(serde_json::Map::new()),
            ),
            (
                "problems",
                serde_json::Value::Object(serde_json::Map::new()),
            ),
            (
                "join_requests",
                serde_json::Value::Object(serde_json::Map::new()),
            ),
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
                    if !c.contains_key("start_time") {
                        c.insert(
                            "start_time".into(),
                            serde_json::Value::String(String::new()),
                        );
                    }
                    if !c.contains_key("end_time") {
                        c.insert("end_time".into(), serde_json::Value::String(String::new()));
                    }
                    if !c.contains_key("description") {
                        c.insert(
                            "description".into(),
                            serde_json::Value::String(String::new()),
                        );
                    }
                    if !c.contains_key("created_by") {
                        c.insert(
                            "created_by".into(),
                            serde_json::Value::String(String::new()),
                        );
                    }
                    if !c.contains_key("problem_order") {
                        c.insert("problem_order".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !c.contains_key("visible_to") {
                        c.insert("visible_to".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !c.contains_key("editable_by") {
                        c.insert("editable_by".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !c.contains_key("link") {
                        c.insert("link".into(), serde_json::Value::Null);
                    }
                }
            }
        }

        // 补全旧版本 Problem 缺失字段
        if let Some(problems) = obj.get_mut("problems").and_then(|p| p.as_object_mut()) {
            for problem in problems.values_mut() {
                if let Some(p) = problem.as_object_mut() {
                    if !p.contains_key("contest") {
                        p.insert("contest".into(), serde_json::Value::String(String::new()));
                    }
                    if !p.contains_key("contest_id") {
                        p.insert("contest_id".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("content") {
                        p.insert("content".into(), serde_json::Value::String(String::new()));
                    }
                    if !p.contains_key("solution") {
                        p.insert("solution".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("public_at") {
                        p.insert("public_at".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("claimed_by") {
                        p.insert("claimed_by".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("verifier_solution") {
                        p.insert("verifier_solution".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("visible_to") {
                        p.insert("visible_to".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !p.contains_key("link") {
                        p.insert("link".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("remark") {
                        p.insert("remark".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("editable_by") {
                        p.insert("editable_by".into(), serde_json::Value::Array(Vec::new()));
                    }
                }
            }
        }

        // 补全旧版本 Post 缺失字段
        if let Some(posts) = obj.get_mut("posts").and_then(|p| p.as_object_mut()) {
            for post in posts.values_mut() {
                if let Some(p) = post.as_object_mut() {
                    if !p.contains_key("content") {
                        p.insert("content".into(), serde_json::Value::String(String::new()));
                    }
                    if !p.contains_key("updated_at") {
                        p.insert(
                            "updated_at".into(),
                            p.get("created_at")
                                .cloned()
                                .unwrap_or(serde_json::Value::String(String::new())),
                        );
                    }
                    if !p.contains_key("tags") {
                        p.insert("tags".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !p.contains_key("pinned") {
                        p.insert("pinned".into(), serde_json::Value::Bool(false));
                    }
                    if !p.contains_key("team_only") {
                        p.insert("team_only".into(), serde_json::Value::Bool(false));
                    }
                    if !p.contains_key("emoji") {
                        p.insert("emoji".into(), serde_json::Value::Null);
                    }
                    if !p.contains_key("reactions") {
                        p.insert("reactions".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !p.contains_key("status") {
                        p.insert("status".into(), serde_json::Value::String("normal".into()));
                    }
                    if !p.contains_key("visible_to") {
                        p.insert("visible_to".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !p.contains_key("editable_by") {
                        p.insert("editable_by".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !p.contains_key("reply_count") {
                        p.insert(
                            "reply_count".into(),
                            serde_json::Value::Number(serde_json::Number::from(0)),
                        );
                    }
                    if !p.contains_key("solution") {
                        p.insert("solution".into(), serde_json::Value::String(String::new()));
                    }
                }
            }
        }

        // 补全旧版本 User 缺失字段
        if let Some(users) = obj.get_mut("users").and_then(|u| u.as_object_mut()) {
            for user in users.values_mut() {
                if let Some(u) = user.as_object_mut() {
                    if !u.contains_key("bio") {
                        u.insert("bio".into(), serde_json::Value::String(String::new()));
                    }
                    if !u.contains_key("password_hash") {
                        u.insert("password_hash".into(), serde_json::Value::Null);
                    }
                    if !u.contains_key("effective_role") {
                        u.insert(
                            "effective_role".into(),
                            serde_json::Value::String(String::new()),
                        );
                    }
                    if !u.contains_key("group_ids") {
                        u.insert("group_ids".into(), serde_json::Value::Array(Vec::new()));
                    }
                    if !u.contains_key("user_permissions") {
                        u.insert(
                            "user_permissions".into(),
                            serde_json::Value::Array(Vec::new()),
                        );
                    }
                }
            }
        }

        // 补全旧版本 join_requests 缺失字段
        if let Some(requests) = obj.get_mut("join_requests").and_then(|r| r.as_object_mut()) {
            for request in requests.values_mut() {
                if let Some(r) = request.as_object_mut() {
                    if !r.contains_key("user_email") {
                        r.insert("user_email".into(), serde_json::Value::Null);
                    }
                }
            }
        }
    }

    serde_json::from_value(value).map_err(|e| format!("数据格式转换失败: {}", e))
}
