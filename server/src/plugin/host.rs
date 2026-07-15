use std::sync::{Arc, Mutex as StdMutex};

/// Register McGuffin host functions on a wasmtime linker.
///
/// Exposed to the WASM module under the `mcguffin` module:
/// - `get_data(namespace_ptr, key_ptr) -> value_ptr`
/// - `set_data(namespace_ptr, key_ptr, value_ptr)`
/// - `get_team_members() -> json_ptr`
/// - `get_current_user() -> json_ptr`
pub fn register_host_functions(
    linker: &mut wasmtime::Linker<()>,
    app_state: crate::state::AppState,
    plugin_id: String,
    user_id: String,
    alloc: Arc<StdMutex<usize>>,
) -> Result<(), String> {
    let app_state_get = app_state.clone();
    let plugin_id_get = plugin_id.clone();
    let alloc_get = alloc.clone();
    linker
        .func_wrap(
            "mcguffin",
            "get_data",
            move |mut caller: wasmtime::Caller<'_, ()>, ns_ptr: i32, key_ptr: i32| -> i32 {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => {
                        tracing::warn!("mcguffin_get_data: no memory export");
                        return 0;
                    }
                };
                let (ns, key) = {
                    let data = memory.data(&caller);
                    (
                        read_null_terminated_string(data, ns_ptr as usize),
                        read_null_terminated_string(data, key_ptr as usize),
                    )
                };
                let value = match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(app_state_get.get_plugin_data(&plugin_id_get, &ns, &key))
                }) {
                    v if v.is_empty() => return 0,
                    v => v,
                };
                let offset = match allocate_in_memory(&alloc_get, &memory, &mut caller, value.len() + 1) {
                    Ok(o) => o,
                    Err(e) => {
                        tracing::warn!("mcguffin_get_data: allocation failed: {}", e);
                        return 0;
                    }
                };
                let data = memory.data_mut(&mut caller);
                if write_null_terminated_string(data, offset, &value).is_none() {
                    tracing::warn!("mcguffin_get_data: write out of bounds");
                    return 0;
                }
                offset as i32
            },
        )
        .map_err(|e| format!("failed to register mcguffin_get_data: {}", e))?;

    let app_state_set = app_state.clone();
    let plugin_id_set = plugin_id.clone();
    linker
        .func_wrap(
            "mcguffin",
            "set_data",
            move |mut caller: wasmtime::Caller<'_, ()>,
                  ns_ptr: i32,
                  key_ptr: i32,
                  value_ptr: i32| {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => {
                        tracing::warn!("mcguffin_set_data: no memory export");
                        return;
                    }
                };
                let (ns, key, value) = {
                    let data = memory.data(&caller);
                    (
                        read_null_terminated_string(data, ns_ptr as usize),
                        read_null_terminated_string(data, key_ptr as usize),
                        read_null_terminated_string(data, value_ptr as usize),
                    )
                };
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(app_state_set.set_plugin_data(&plugin_id_set, &ns, &key, value))
                });
            },
        )
        .map_err(|e| format!("failed to register mcguffin_set_data: {}", e))?;

    let state_members = app_state.clone();
    let alloc_members = alloc.clone();
    linker
        .func_wrap(
            "mcguffin",
            "get_team_members",
            move |mut caller: wasmtime::Caller<'_, ()>| -> i32 {
                let json = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let members = state_members.team_members.read().await;
                        let users = state_members.users.lock().await;
                        let list: Vec<serde_json::Value> = members
                            .values()
                            .map(|m| {
                                let user = users.get(&m.user_id);
                                serde_json::json!({
                                    "user_id": m.user_id,
                                    "joined_at": m.joined_at,
                                    "display_name": user.map(|u| u.display_name.clone()),
                                    "role": user.map(|u| u.effective_role.clone()),
                                })
                            })
                            .collect();
                        serde_json::to_string(&list).unwrap_or_default()
                    })
                });
                write_json_return(&mut caller, &alloc_members, &json)
            },
        )
        .map_err(|e| format!("failed to register mcguffin_get_team_members: {}", e))?;

    let state_user = app_state.clone();
    let alloc_user = alloc;
    linker
        .func_wrap(
            "mcguffin",
            "get_current_user",
            move |mut caller: wasmtime::Caller<'_, ()>| -> i32 {
                let json = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let users = state_user.users.lock().await;
                        users
                            .get(&user_id)
                            .map(|u| {
                                serde_json::json!({
                                    "id": u.id,
                                    "username": u.username,
                                    "display_name": u.display_name,
                                    "role": u.role,
                                    "effective_role": u.effective_role,
                                    "team_status": u.team_status,
                                })
                            })
                            .and_then(|v| serde_json::to_string(&v).ok())
                            .unwrap_or_default()
                    })
                });
                write_json_return(&mut caller, &alloc_user, &json)
            },
        )
        .map_err(|e| format!("failed to register mcguffin_get_current_user: {}", e))?;

    Ok(())
}

pub(crate) fn allocate_in_memory(
    alloc: &StdMutex<usize>,
    memory: &wasmtime::Memory,
    ctx: &mut impl wasmtime::AsContextMut<Data = ()>,
    size: usize,
) -> Result<usize, String> {
    const PAGE_SIZE: usize = 64 * 1024;
    let mut offset = alloc.lock().map_err(|e| format!("alloc lock poisoned: {}", e))?;
    let needed = offset.checked_add(size).ok_or("allocation overflow")?;
    let current_bytes = memory.size(&*ctx) as usize * PAGE_SIZE;
    if current_bytes < needed {
        let delta = (needed - current_bytes).div_ceil(PAGE_SIZE) as u64;
        memory
            .grow(ctx, delta)
            .map_err(|e| format!("failed to grow WASM memory: {}", e))?;
    }
    let result = *offset;
    *offset = needed;
    Ok(result)
}

pub(crate) fn write_null_terminated_string(
    memory: &mut [u8],
    offset: usize,
    value: &str,
) -> Option<usize> {
    let bytes = value.as_bytes();
    let end = offset.checked_add(bytes.len())?.checked_add(1)?;
    if end > memory.len() {
        return None;
    }
    memory[offset..offset + bytes.len()].copy_from_slice(bytes);
    memory[offset + bytes.len()] = 0;
    Some(bytes.len() + 1)
}

fn write_json_return(
    caller: &mut wasmtime::Caller<'_, ()>,
    alloc: &StdMutex<usize>,
    json: &str,
) -> i32 {
    let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(m) => m,
        None => {
            tracing::warn!("mcguffin json return: no memory export");
            return 0;
        }
    };
    let offset = match allocate_in_memory(alloc, &memory, caller, json.len() + 1) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("mcguffin json return: allocation failed: {}", e);
            return 0;
        }
    };
    let data = memory.data_mut(caller);
    if write_null_terminated_string(data, offset, json).is_none() {
        tracing::warn!("mcguffin json return: write out of bounds");
        return 0;
    }
    offset as i32
}

pub(crate) fn read_null_terminated_string(memory: &[u8], offset: usize) -> String {
    if offset >= memory.len() {
        return String::new();
    }
    let mut end = offset;
    while end < memory.len() && memory[end] != 0 {
        end += 1;
    }
    std::str::from_utf8(&memory[offset..end])
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read_null_terminated_string() {
        let mut buffer = vec![0u8; 64];
        let value = "hello wasm";
        let written = write_null_terminated_string(&mut buffer, 4, value).unwrap();
        assert_eq!(written, value.len() + 1);
        let read = read_null_terminated_string(&buffer, 4);
        assert_eq!(read, value);
    }

    #[test]
    fn test_write_null_terminated_string_too_large() {
        let mut buffer = vec![0u8; 4];
        assert!(write_null_terminated_string(&mut buffer, 0, "hello").is_none());
    }

    #[test]
    fn test_read_null_terminated_string_out_of_bounds() {
        let buffer = vec![0u8; 4];
        assert_eq!(read_null_terminated_string(&buffer, 10), "");
    }
}
