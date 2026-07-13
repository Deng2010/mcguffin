use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Local;
use std::path::PathBuf;

use crate::db::{create_consistent_backup, restore_from_backup};
use crate::state::AppState;
use crate::utils::AuthUser;

// ============== Data Backup / Restore ==============

/// 获取备份目录
/// 优先使用自定义备份目录，否则使用数据库文件同级目录下的 backups/
async fn backup_dir(state: &AppState) -> PathBuf {
    // 如果设置了自定义备份目录，优先使用
    if let Some(dir) = state.backup_directory.read().await.as_ref() {
        if !dir.is_empty() {
            return std::path::PathBuf::from(dir);
        }
    }
    // 默认：数据库文件同级目录下的 backups/
    let db_path = std::path::Path::new(&state.db_path);
    db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("backups")
}

/// 获取 SQLite 数据库文件路径
fn db_path_from_state(state: &AppState) -> PathBuf {
    std::path::PathBuf::from(&state.db_path)
}

/// 获取备份文件列表，按名称降序（最新的在前）
/// 只列出 .db（SQLite）备份文件
fn list_backup_files(dir: &PathBuf) -> Vec<serde_json::Value> {
    let dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut entries: Vec<_> = dir
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "db").unwrap_or(false))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            Some(serde_json::json!({
                "name": name,
                "size": meta.len(),
                "type": "sqlite",
                "modified": chrono::DateTime::<chrono::Utc>::from(meta.modified().ok()?)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            }))
        })
        .collect();
    entries.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        bn.cmp(an)
    });
    entries
}

/// POST /api/admin/backup
/// manage_backups permission required — creates a SQLite backup (.db)
pub async fn create_backup(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    auth.require_perm(&state, crate::types::perms::MANAGE_BACKUPS)
        .await?;

    let dir = backup_dir(&state).await;
    if std::fs::create_dir_all(&dir).is_err() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无法创建备份目录"}),
        ));
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let db_path = db_path_from_state(&state);
    let db_backup_name = format!("mcguffin_data_{}.db", timestamp);
    let db_backup_path = dir.join(&db_backup_name);

    if !db_path.exists() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "数据库文件不存在，无法创建备份"}),
        ));
    }

    // 备份前同步 HashMap 数据到 SQLite 并刷新 WAL
    state.sync_to_db().await;

    match create_consistent_backup(
        &db_path.to_string_lossy(),
        &db_backup_path.to_string_lossy(),
    ) {
        Ok(()) => {
            tracing::info!("备份已创建: {:?}", db_backup_path);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "备份已创建",
                "backup": db_backup_name,
            })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({"success": false, "message": format!("备份失败: {}", e)}),
        )),
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

    let dir = backup_dir(&state).await;
    let backups = list_backup_files(&dir);

    Ok(Json(
        serde_json::json!({"success": true, "backups": backups}),
    ))
}

/// POST /api/admin/backup/restore/:name
/// manage_backups permission required — restores from a named backup (supports .db and .json)
pub async fn restore_backup(
    State(state): State<AppState>,
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
    if !name_clean.ends_with(".db") {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "仅支持 .db 备份文件恢复"}),
        ));
    }

    let dir = backup_dir(&state).await;
    let backup_path = dir.join(name_clean);
    if !backup_path.exists() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "备份文件不存在"}),
        ));
    }

    let safety_timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let db_path = db_path_from_state(&state);

    // 1. 创建安全备份
    let safety_name = format!("pre_restore_{}.db", safety_timestamp);
    let safety_path = dir.join(&safety_name);
    if let Err(e) = create_consistent_backup(
        &db_path.to_string_lossy(),
        &safety_path.to_string_lossy(),
    ) {
        tracing::warn!("创建安全备份失败: {}", e);
    }

    // 2. 使用 SQLite 在线备份 API 将备份恢复到主数据库（无需关闭连接池）
    if let Err(e) =
        restore_from_backup(&backup_path.to_string_lossy(), &db_path.to_string_lossy())
    {
        return Ok(Json(
            serde_json::json!({"success": false, "message": format!("恢复失败: {}", e)}),
        ));
    }

    // 3. 验证完整性
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

    // 4. 从 SQLite 重新加载数据到内存
    state.reload().await;

    tracing::info!("从备份恢复成功: {:?}", backup_path);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "备份已恢复，安全备份已创建",
        "safety_backup": safety_name,
    })))
}

/// POST /api/admin/backup/restore-upload
/// 接收前端上传的 .db 文件（base64 编码），保存到备份目录后执行恢复
pub async fn restore_upload_backup(
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

    let filename = payload
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("uploaded_backup.db");

    // 确保文件名安全
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无效的文件名"}),
        ));
    }

    // 解码 base64
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false, "message": "base64 解码失败"
                })),
            )
        })?;

    let dir = backup_dir(&state).await;
    if std::fs::create_dir_all(&dir).is_err() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "无法创建备份目录"}),
        ));
    }

    // 写入备份目录
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let upload_name = format!("uploaded_{}_{}", timestamp, filename);
    let upload_path = dir.join(&upload_name);
    if let Err(e) = std::fs::write(&upload_path, &bytes) {
        return Ok(Json(
            serde_json::json!({"success": false, "message": format!("文件写入失败: {}", e)}),
        ));
    }

    tracing::info!("收到上传的 .db 备份文件: {:?}", upload_path);

    // 创建安全备份
    let safety_timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let safety_name = format!("pre_restore_{}.db", safety_timestamp);
    let safety_path = dir.join(&safety_name);
    if let Err(e) =
        create_consistent_backup(&state.db_path, &safety_path.to_string_lossy())
    {
        tracing::warn!("创建安全备份失败: {}", e);
    }

    let db_path = db_path_from_state(&state);

    // 使用 SQLite 在线备份 API 将上传的备份恢复到主数据库（无需关闭连接池）
    if let Err(e) =
        restore_from_backup(&upload_path.to_string_lossy(), &db_path.to_string_lossy())
    {
        return Ok(Json(
            serde_json::json!({"success": false, "message": format!("恢复失败: {}", e)}),
        ));
    }

    // 验证完整性
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&state.db)
        .await
        .unwrap_or_else(|_| "error".to_string());

    if integrity != "ok" {
        tracing::error!("上传恢复后完整性检查失败: {}", integrity);
        return Ok(Json(
            serde_json::json!({"success": false, "message": format!("数据完整性检查失败: {}，建议手动恢复安全备份 {}", integrity, safety_name)}),
        ));
    }

    // 从 SQLite 重新加载数据到内存
    state.reload().await;

    tracing::info!("从上传的 .db 文件恢复成功: {:?}", upload_path);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "从 .db 文件恢复成功，安全备份已创建",
        "safety_backup": safety_name,
    })))
}

/// GET /api/admin/backup/download/{name}
/// 下载备份文件。从 .db 备份生成完整 JSON 内容返回。
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

    let backup_path = backup_dir(&state).await.join(name_clean);
    if !backup_path.exists() {
        return Ok(Json(
            serde_json::json!({"success": false, "message": "备份文件不存在"}),
        ));
    }

    let is_json = name_clean.ends_with(".json");

    if is_json {
        // 旧格式 .json 备份文件，直接读取
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
        // .db 备份 — 返回 base64 编码的二进制文件
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

    let dir = backup_dir(&state).await;
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
