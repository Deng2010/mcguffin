use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::Value;

use crate::plugin::host::{
    allocate_in_memory, read_null_terminated_string, register_host_functions,
    write_null_terminated_string,
};
use crate::plugin::trait_::PluginContext;
use crate::state::AppState;
use crate::utils::AuthUser;

/// POST /plugins/{plugin_id}/call
/// Authenticated entry point that invokes a WASM plugin's `_plugin_handle_request`.
pub async fn call_plugin(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (wasm_path, manifest) = state.plugins.get_wasm_plugin(&plugin_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": "Plugin not found",
            })),
        )
    })?;

    for perm in &manifest.permissions_needed {
        auth.require_perm(&state, perm).await?;
    }

    let context = PluginContext {
        base_url: state.site_url.clone(),
        user_id: auth.user_id.clone(),
        user_role: auth.user.effective_role.clone(),
        request_method: "POST".to_string(),
        request_path: format!("/plugins/{}/call", plugin_id),
        request_body: body,
    };
    let context_json = serde_json::to_string(&context).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": format!("failed to serialize context: {}", e),
            })),
        )
    })?;

    let wasm_bytes = tokio::fs::read(&wasm_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": format!("failed to read WASM: {}", e),
            })),
        )
    })?;

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_bytes).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": format!("invalid WASM module: {}", e),
            })),
        )
    })?;

    let alloc = Arc::new(Mutex::new(0usize));
    let mut linker = wasmtime::Linker::new(&engine);
    register_host_functions(
        &mut linker,
        state.clone(),
        plugin_id.clone(),
        auth.user_id.clone(),
        alloc.clone(),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": format!("failed to register host functions: {}", e),
            })),
        )
    })?;

    let mut store = wasmtime::Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": format!("failed to instantiate WASM module: {}", e),
            })),
        )
    })?;

    let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": "Plugin does not export memory",
            })),
        )
    })?;

    let context_ptr = allocate_in_memory(&alloc, &memory, &mut store, context_json.len() + 1)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("failed to allocate context memory: {}", e),
                })),
            )
        })?;

    let data = memory.data_mut(&mut store);
    write_null_terminated_string(data, context_ptr, &context_json).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "message": "context string does not fit in WASM memory",
            })),
        )
    })?;

    let func = instance.get_func(&mut store, "_plugin_handle_request").ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "message": "Plugin does not export _plugin_handle_request",
            })),
        )
    })?;

    let mut results = vec![wasmtime::Val::I32(0)];
    func.call(&mut store, &[wasmtime::Val::I32(context_ptr as i32)], &mut results)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("WASM execution failed: {}", e),
                })),
            )
        })?;

    let ret_ptr = match results[0] {
        wasmtime::Val::I32(ptr) => ptr as usize,
        _ => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "message": "Invalid return value from plugin",
                })),
            ));
        }
    };

    let response_str = {
        let data = memory.data(&store);
        read_null_terminated_string(data, ret_ptr)
    };
    let response: Value = serde_json::from_str(&response_str).unwrap_or_else(|_| {
        serde_json::json!({
            "success": false,
            "message": "Invalid plugin response JSON",
        })
    });

    Ok(Json(response))
}
