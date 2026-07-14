use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::handlers::admin::{
    admin_change_user_role, admin_list_users, admin_remove_user, create_backup, create_group,
    delete_backup, delete_group, download_backup, export_config, export_data, export_db,
    get_audit_log, get_config, get_showcase_config, import_config, import_data, init_admin,
    init_admin_status, list_backups, list_groups, restore_backup, restore_upload_backup,
    restart_service, set_problem_acl, set_resource_acl, set_user_groups, set_user_permissions,
    update_config, update_group, update_showcase_config,
};
use crate::handlers::auth::{
    get_permissions, login, oauth_authorize, oauth_callback, refresh_token,
};
use crate::handlers::contest::{
    create_contest, delete_contest, get_contest_problems, get_contests, set_contest_status,
    set_problem_order, update_contest,
};
use crate::handlers::info::{get_difficulties, get_site_info, update_site_description};
use crate::handlers::notification::{
    get_notifications, mark_all_notifications_read, mark_notification_read,
};
use crate::handlers::pages::{login_page, portfolio_page};
use crate::handlers::plugin::{
    get_plugin_routes, get_plugins, install_plugin_from_url, install_plugin_upload,
    reload_plugins, uninstall_plugin,
};
use crate::handlers::post::{
    create_announcement, create_post, create_suggestion, delete_announcement, delete_post,
    delete_post_reply, delete_suggestion, delete_suggestion_reply, get_announcement_detail,
    get_announcements, get_community_posts, get_discussion_emojis, get_discussion_tags,
    get_post_detail, get_posts, get_suggestion_detail, get_suggestions, react_to_post,
    react_to_reply, reply_to_post, reply_to_suggestion, update_announcement, update_post,
    update_suggestion,
};
use crate::handlers::problem::{
    claim_problem, delete_problem, get_pending_problems_admin, get_problem_detail, get_problems,
    get_team_members_for_visibility, review_problem, set_problem_contest, set_problem_visibility,
    submit_problem, submit_verifier_solution, unclaim_problem, update_problem,
};
use crate::handlers::team::{
    apply_to_join, change_member_role, get_pending_requests, get_team_members, remove_member,
    review_application,
};
use crate::handlers::user::{
    check_name_available, get_current_user, get_public_profile, logout, update_profile,
    verify_token,
};
use crate::state::AppState;

async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": state.site_version,
    }))
}

/// Builds the API + SSR pages router.
///
/// API routes are registered under `/api/v1/` (canonical) and `/api/`
/// (backward compatibility). Legacy discussion/suggestion/announcement paths
/// are kept under `/api/` so existing frontend clients keep working.
pub fn build_router(state: AppState) -> Router {
    let api_router = Router::new()
        // Health
        .route("/health", get(health_check))
        // OAuth / auth
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/callback", get(oauth_callback))
        .route("/oauth/token", post(refresh_token))
        .route("/auth/login", post(login))
        .route("/auth/permissions", get(get_permissions))
        // User
        .route("/user/me", get(get_current_user))
        .route("/user/profile/{username}", get(get_public_profile))
        .route("/user/profile", put(update_profile))
        .route("/user/check-name", get(check_name_available))
        .route("/user/verify", get(verify_token))
        .route("/logout", post(logout))
        // Team
        .route("/team/members", get(get_team_members))
        .route("/team/requests", get(get_pending_requests))
        .route("/team/apply", post(apply_to_join))
        .route("/team/review/{request_id}/{action}", post(review_application))
        .route("/team/members/role/{user_id}", post(change_member_role))
        .route("/team/members/remove/{user_id}", post(remove_member))
        // Problems
        .route("/problems", get(get_problems).post(submit_problem))
        .route("/problems/detail/{problem_id}", get(get_problem_detail))
        .route("/problems/claim/{problem_id}", post(claim_problem))
        .route("/problems/unclaim/{problem_id}", post(unclaim_problem))
        .route(
            "/problems/verifier-solution/{problem_id}",
            post(submit_verifier_solution),
        )
        .route(
            "/problems/visibility/{problem_id}",
            post(set_problem_visibility),
        )
        .route("/problems/review/{problem_id}/{action}", post(review_problem))
        .route("/problems/admin/pending", get(get_pending_problems_admin))
        .route(
            "/problems/admin/members",
            get(get_team_members_for_visibility),
        )
        .route(
            "/problems/{problem_id}",
            put(update_problem).delete(delete_problem),
        )
        // Contests
        .route("/contests", get(get_contests).post(create_contest))
        .route("/contests/{contest_id}", delete(delete_contest))
        .route("/contests/{contest_id}", put(update_contest))
        .route("/contests/{contest_id}/status", post(set_contest_status))
        .route("/contests/{contest_id}/problems", get(get_contest_problems))
        .route(
            "/contests/{contest_id}/problem-order",
            post(set_problem_order),
        )
        .route("/problems/contest/{problem_id}", post(set_problem_contest))
        // Site info
        .route("/site/info", get(get_site_info))
        .route("/site/description", put(update_site_description))
        .route("/site/difficulties", get(get_difficulties))
        // Unified posts
        .route("/posts", get(get_posts).post(create_post))
        .route(
            "/posts/{id}",
            get(get_post_detail).put(update_post).delete(delete_post),
        )
        .route("/posts/{id}/reply", post(reply_to_post))
        .route("/posts/{id}/reply/{reply_id}", delete(delete_post_reply))
        .route("/posts/{id}/react", post(react_to_post))
        .route(
            "/posts/{id}/reply/{reply_id}/react",
            post(react_to_reply),
        )
        // Community feed
        .route("/community/posts", get(get_community_posts))
        // Tags & emojis
        .route("/posts/tags", get(get_discussion_tags))
        .route("/posts/emojis", get(get_discussion_emojis))
        // Notifications
        .route("/notifications", get(get_notifications))
        .route("/notifications/read/{id}", post(mark_notification_read))
        .route("/notifications/read-all", post(mark_all_notifications_read))
        // Admin config
        .route("/admin/config", get(get_config).put(update_config))
        .route("/admin/init-status", get(init_admin_status))
        .route("/admin/init", post(init_admin))
        .route("/admin/restart", post(restart_service))
        // Admin backup
        .route("/admin/backup", post(create_backup))
        .route("/admin/backups", get(list_backups))
        .route("/admin/backup/restore/{name}", post(restore_backup))
        .route(
            "/admin/backup/restore-upload",
            post(restore_upload_backup),
        )
        .route("/admin/backup/download/{name}", get(download_backup))
        .route("/admin/backup/{name}", delete(delete_backup))
        // Admin showcase
        .route(
            "/admin/showcase",
            get(get_showcase_config).put(update_showcase_config),
        )
        // Admin export
        .route("/admin/export/data", get(export_data))
        .route("/admin/export/db", get(export_db))
        .route("/admin/export/config", get(export_config))
        // Admin import
        .route("/admin/import/data", post(import_data))
        .route("/admin/import/config", post(import_config))
        // Admin audit log
        .route("/admin/audit-log", get(get_audit_log))
        // Admin user management
        .route("/admin/users", get(admin_list_users))
        .route("/admin/users/{user_id}/role", post(admin_change_user_role))
        .route("/admin/users/{user_id}/remove", post(admin_remove_user))
        .route("/admin/users/{user_id}/groups", put(set_user_groups))
        .route(
            "/admin/users/{user_id}/permissions",
            put(set_user_permissions),
        )
        // Admin member groups
        .route("/admin/groups", get(list_groups).post(create_group))
        .route(
            "/admin/groups/{group_id}",
            put(update_group).delete(delete_group),
        )
        // Admin problem ACL
        .route("/admin/problems/{problem_id}/acl", put(set_problem_acl))
        // Admin unified resource ACL
        .route(
            "/admin/acl/{resource_type}/{resource_id}",
            put(set_resource_acl),
        )
        .route("/plugins", get(get_plugins))
        .route("/plugins/routes", get(get_plugin_routes))
        .route("/plugins/reload", post(reload_plugins))
        // Plugin install/uninstall (admin only)
        .route("/admin/plugins/install-url", post(install_plugin_from_url))
        .route("/admin/plugins/install", post(install_plugin_upload))
        .route("/admin/plugins/{id}", delete(uninstall_plugin));

    let plugin_router = state.plugins.build_plugin_router(state.clone());
    let api_router = api_router.merge(plugin_router);
    let api_router = api_router.with_state(state.clone());

    let legacy_router = Router::new()
        // Legacy discussion routes -> unified posts
        .route("/discussions", get(get_posts).post(create_post))
        .route(
            "/discussions/{id}",
            get(get_post_detail).put(update_post).delete(delete_post),
        )
        .route("/discussions/{id}/reply", post(reply_to_post))
        .route(
            "/discussions/{id}/reply/{reply_id}",
            delete(delete_post_reply),
        )
        .route("/discussions/{id}/react", post(react_to_post))
        .route(
            "/discussions/{id}/reply/{reply_id}/react",
            post(react_to_reply),
        )
        .route("/discussions/tags", get(get_discussion_tags))
        .route("/discussions/emojis", get(get_discussion_emojis))
        // Legacy suggestion routes
        .route("/suggestions", get(get_suggestions).post(create_suggestion))
        .route(
            "/suggestions/{id}",
            get(get_suggestion_detail)
                .put(update_suggestion)
                .delete(delete_suggestion),
        )
        .route("/suggestions/{id}/reply", post(reply_to_suggestion))
        .route(
            "/suggestions/{id}/reply/{reply_id}",
            delete(delete_suggestion_reply),
        )
        // Legacy announcement routes
        .route(
            "/announcements",
            get(get_announcements).post(create_announcement),
        )
        .route(
            "/announcements/{id}",
            get(get_announcement_detail)
                .put(update_announcement)
                .delete(delete_announcement),
        )
        .with_state(state.clone());

    let pages_router = Router::new()
        .route("/login", get(login_page))
        .route("/portfolio", get(portfolio_page))
        .with_state(state.clone());

    let api_with_legacy = Router::new()
        .merge(api_router.clone())
        .merge(legacy_router);

    Router::new()
        .nest("/api/v1", api_router)
        .nest("/api", api_with_legacy)
        .merge(pages_router)
}
