use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

use crate::state::AppState;
use crate::types::perms;
use crate::utils::AuthUser;

/// Maximum file size for plugin files (50MB).
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;

// ── Helpers ────────────────────────────────────────────────

async fn require_plugin_access(
    state: &AppState,
    plugin_id: &str,
) -> Result<PathBuf, (StatusCode, Json<Value>)> {
    if !state.plugins.has_plugin(plugin_id).await {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": format!("Plugin '{}' is not installed", plugin_id)
            })),
        ));
    }
    // Base directory for this plugin's files
    Ok(state.plugins.plugins_dir.join(plugin_id).join("files"))
}

/// Validate file path to prevent directory traversal.
fn validate_path(file_path: &str) -> Result<String, (StatusCode, Json<Value>)> {
    let cleaned = file_path.trim_start_matches('/');
    if cleaned.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": "文件路径不能包含 '..'"
            })),
        ));
    }
    if cleaned.starts_with('/') || cleaned.starts_with('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": "文件路径不能是绝对路径"
            })),
        ));
    }
    Ok(cleaned.to_string())
}

/// Infer MIME type from file extension.
fn infer_mime(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png"  => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif"  => "image/gif",
        "svg"  => "image/svg+xml",
        "webp" => "image/webp",
        "json" => "application/json",
        "txt"  => "text/plain",
        "html" => "text/html",
        "css"  => "text/css",
        "js"   => "application/javascript",
        "wasm" => "application/wasm",
        "pdf"  => "application/pdf",
        _      => "application/octet-stream",
    }
}

// ── Request types ────────────────────────────────────────

#[derive(Deserialize)]
pub struct FileListQuery {
    #[serde(default)]
    pub prefix: Option<String>,
}

// ── Handlers ───────────────────────────────────────────────

/// POST /plugins/{plugin_id}/files/{*file_path}
/// Upload or overwrite a file.
pub async fn plugin_write_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((plugin_id, file_path)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::VIEW_SHOWCASE).await?;
    let base_dir = require_plugin_access(&state, &plugin_id).await?;
    let safe_path = validate_path(&file_path)?;

    if body.len() as u64 > MAX_FILE_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "success": false,
                "message": format!("文件大小超过限制 ({}MB)", MAX_FILE_SIZE / 1024 / 1024)
            })),
        ));
    }

    let full_path = base_dir.join(&safe_path);
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "message": format!("创建目录失败: {}", e) })),
        ))?;
    }

    tokio::fs::write(&full_path, &body).await.map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "success": false, "message": format!("写入文件失败: {}", e) })),
    ))?;

    Ok(Json(serde_json::json!({
        "success": true, "path": safe_path, "size": body.len(),
    })))
}

/// GET /plugins/{plugin_id}/files/{*file_path}
/// Read a file.
pub async fn plugin_read_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((plugin_id, file_path)): Path<(String, String)>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::VIEW_SHOWCASE).await?;
    let base_dir = require_plugin_access(&state, &plugin_id).await?;
    let safe_path = validate_path(&file_path)?;
    let full_path = base_dir.join(&safe_path);

    if !full_path.exists() {
        return Err((StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "message": "文件不存在" }))));
    }

    let content = tokio::fs::read(&full_path).await.map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "success": false, "message": format!("读取文件失败: {}", e) })),
    ))?;

    Ok(([(header::CONTENT_TYPE, infer_mime(&safe_path))], content).into_response())
}

/// DELETE /plugins/{plugin_id}/files/{*file_path}
/// Delete a file. Cross-plugin isolation is enforced by directory structure:
/// each plugin can only access its own `{plugins_dir}/{id}/files/` directory.
pub async fn plugin_delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((plugin_id, file_path)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::VIEW_SHOWCASE).await?;
    let base_dir = require_plugin_access(&state, &plugin_id).await?;
    let safe_path = validate_path(&file_path)?;
    let full_path = base_dir.join(&safe_path);

    if !full_path.exists() {
        return Err((StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "message": "文件不存在" }))));
    }

    tokio::fs::remove_file(&full_path).await.map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "success": false, "message": format!("删除文件失败: {}", e) })),
    ))?;

    // Clean up empty parent directories
    if let Some(parent) = full_path.parent() {
        if parent != base_dir {
            let _ = tokio::fs::remove_dir(parent).await;
        }
    }

    Ok(Json(serde_json::json!({ "success": true, "path": safe_path })))
}

/// GET /plugins/{plugin_id}/files/list?prefix=...
/// List files in the plugin's file directory.
pub async fn plugin_list_files(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(query): Query<FileListQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    auth.require_perm(&state, perms::VIEW_SHOWCASE).await?;
    let base_dir = require_plugin_access(&state, &plugin_id).await?;

    if !base_dir.exists() {
        return Ok(Json(serde_json::json!({ "files": [], "count": 0 })));
    }

    let prefix_path = match &query.prefix {
        Some(p) if !p.is_empty() => base_dir.join(p.trim_start_matches('/')),
        None | Some(_) => base_dir.clone(),
    };

    let mut files = Vec::new();
    if prefix_path.is_dir() {
        let mut walk = tokio::fs::read_dir(&prefix_path).await.map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "message": format!("读取目录失败: {}", e) })),
        ))?;

        while let Ok(Some(entry)) = walk.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Ok(rel) = path.strip_prefix(&base_dir) {
                    files.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }

    files.sort();
    Ok(Json(serde_json::json!({ "files": files, "count": files.len() })))
}
