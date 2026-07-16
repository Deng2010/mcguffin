use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::state::AppState;
use crate::types::perms;
use crate::utils::AuthUser;

// ── Request types ────────────────────────────────────────

#[derive(Deserialize)]
pub struct DataQuery {
    pub namespace: String,
    pub key: String,
}

#[derive(Deserialize)]
pub struct SetDataBody {
    pub namespace: String,
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct AddBody {
    pub namespace: String,
    pub key: String,
    pub delta: i64,
}

#[derive(Deserialize)]
pub struct SetMemberBody {
    pub namespace: String,
    pub key: String,
    pub member: String,
}

#[derive(Deserialize)]
pub struct KeysQuery {
    pub namespace: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Deserialize)]
pub struct IsMemberQuery {
    pub namespace: String,
    pub key: String,
    pub member: String,
}

#[derive(Deserialize)]
pub struct NotifyBody {
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
}

// ── Helpers ────────────────────────────────────────────────

/// Verify auth + plugin exists. Returns an error response if either fails.
async fn require_plugin_access(
    state: &AppState,
    auth: &AuthUser,
    plugin_id: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    if !state.plugins.has_plugin(plugin_id).await {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": format!("Plugin '{}' is not installed", plugin_id)
            })),
        ));
    }
    auth.require_perm(state, perms::VIEW_SHOWCASE).await?;
    Ok(())
}

// ── Handlers ───────────────────────────────────────────────

/// GET /plugins/{plugin_id}/data?namespace=...&key=...
/// Read a plugin data value.
pub async fn plugin_get_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(query): Query<DataQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let value = state.get_plugin_data(&plugin_id, &query.namespace, &query.key).await;
    Ok(Json(serde_json::json!({ "value": value })))
}

/// POST /plugins/{plugin_id}/data
/// Write a plugin data value.
pub async fn plugin_set_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(body): Json<SetDataBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    state.set_plugin_data(&plugin_id, &body.namespace, &body.key, body.value).await;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /plugins/{plugin_id}/data/add
/// Atomically add a delta to a counter and return the new value.
pub async fn plugin_add(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(body): Json<AddBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let value = state.plugin_add(&plugin_id, &body.namespace, &body.key, body.delta).await;
    Ok(Json(serde_json::json!({ "value": value })))
}

/// POST /plugins/{plugin_id}/data/set-add
/// Add a member to a set. Returns `added: true` if new, `added: false` if already present.
pub async fn plugin_set_add(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(body): Json<SetMemberBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let added = state.plugin_set_add(&plugin_id, &body.namespace, &body.key, &body.member).await;
    Ok(Json(serde_json::json!({ "added": added })))
}

/// POST /plugins/{plugin_id}/data/set-remove
/// Remove a member from a set.
pub async fn plugin_set_remove(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(body): Json<SetMemberBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let removed = state.plugin_set_remove(&plugin_id, &body.namespace, &body.key, &body.member).await;
    Ok(Json(serde_json::json!({ "removed": removed })))
}

/// GET /plugins/{plugin_id}/data/set-members?namespace=...&key=...
/// Get all members of a set.
pub async fn plugin_set_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(query): Query<DataQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let members = state.plugin_set_members(&plugin_id, &query.namespace, &query.key).await;
    let count = members.len();
    Ok(Json(serde_json::json!({ "members": members, "count": count })))
}

/// GET /plugins/{plugin_id}/data/set-is-member?namespace=...&key=...&member=...
/// Check if a member exists in a set.
pub async fn plugin_set_is_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(query): Query<IsMemberQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let is_member = state.plugin_set_is_member(&plugin_id, &query.namespace, &query.key, &query.member).await;
    Ok(Json(serde_json::json!({ "is_member": is_member })))
}

/// GET /plugins/{plugin_id}/data/keys?namespace=...&prefix=...
/// List all keys in a namespace, optionally filtered by prefix.
pub async fn plugin_keys(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Query(query): Query<KeysQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    let keys = state.plugin_keys(&plugin_id, &query.namespace, query.prefix.as_deref()).await;
    Ok(Json(serde_json::json!({ "keys": keys })))
}

/// POST /plugins/{plugin_id}/notify
/// Create a notification on behalf of a plugin.
pub async fn plugin_create_notification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
    Json(body): Json<NotifyBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;
    crate::handlers::notification::create_notification(
        &state,
        &body.user_id,
        &body.title,
        &body.body,
        body.link.as_deref(),
    )
    .await;
    Ok(Json(serde_json::json!({ "success": true })))
}

// ── User info ───────────────────────────────────────────────

/// GET /plugins/{plugin_id}/users/me
/// Get the current authenticated user's profile.
pub async fn plugin_user_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;

    Ok(Json(serde_json::json!({
        "id": auth.user.id,
        "username": auth.user.username,
        "display_name": auth.user.display_name,
        "avatar_url": auth.user.avatar_url,
        "role": auth.user.role,
        "effective_role": auth.user.effective_role,
        "team_status": auth.user.team_status,
        "bio": auth.user.bio,
        "created_at": auth.user.created_at,
    })))
}

/// GET /plugins/{plugin_id}/users/{target_user_id}
/// Get a specific user's public profile.
pub async fn plugin_user_get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((plugin_id, target_user_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;

    let users = state.users.lock().await;
    let user = users.get(&target_user_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": "用户不存在"
            })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "avatar_url": user.avatar_url,
        "role": user.role,
        "effective_role": user.effective_role,
        "team_status": user.team_status,
        "bio": user.bio,
        "created_at": user.created_at,
    })))
}

/// GET /plugins/{plugin_id}/users
/// List all team members visible to plugins.
pub async fn plugin_user_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(plugin_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    require_plugin_access(&state, &auth, &plugin_id).await?;

    let members = state.team_members.read().await;
    let users = state.users.lock().await;
    let list: Vec<Value> = members
        .values()
        .filter_map(|m| {
            let user = users.get(&m.user_id)?;
            Some(serde_json::json!({
                "user_id": m.user_id,
                "username": user.username,
                "display_name": user.display_name,
                "avatar_url": user.avatar_url,
                "role": user.role,
                "effective_role": user.effective_role,
                "team_status": user.team_status,
                "joined_at": m.joined_at,
            }))
        })
        .collect();

    Ok(Json(serde_json::json!({
        "members": list,
        "count": list.len(),
    })))
}
