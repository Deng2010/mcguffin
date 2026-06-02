use axum::http::header;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use mcguffin_server_lib::*;
use std::net::SocketAddr;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::new().await;

    // Start auto-backup (hourly, keep 48 backups)
    let state_for_backup = std::sync::Arc::new(state.clone());
    state_for_backup.start_auto_backup(3600, 48);

    crate::discussions::truncate_existing_posts(&state).await;
    crate::discussions::cleanup_orphan_reactions(&state).await;

    let frontend_origin: axum::http::HeaderValue = state
        .site_url
        .parse()
        .expect("SITE_URL must be a valid origin");

    let local_vite_origin: axum::http::HeaderValue =
        "http://localhost:5173".parse().expect("valid origin");

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([frontend_origin, local_vite_origin]))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    // Frontend dist path — try multiple locations relative to CWD
    let dist_path = ["../web/dist", "web/dist", "../dist"]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.join("index.html").exists())
        .unwrap_or_else(|| {
            eprintln!("错误: 找不到前端构建产物 dist/index.html");
            eprintln!("  请先运行 'bun run build'（web 目录下）或 'just build-frontend'");
            std::process::exit(1);
        })
        .canonicalize()
        .unwrap_or_else(|e| {
            eprintln!("错误: 无法解析前端路径: {}", e);
            std::process::exit(1);
        });
    let assets_path = dist_path.join("assets");
    if !assets_path.exists() {
        eprintln!("错误: 前端资产目录不存在: {:?}", assets_path);
        std::process::exit(1);
    }
    let spa_index = std::fs::read_to_string(dist_path.join("index.html"))
        .expect("Failed to read dist/index.html");
    let spa_index_fallback = spa_index.clone();

    println!("Serving frontend SPA from: {:?}", dist_path);
    println!("McGuffin Server running on http://0.0.0.0:3000");
    println!("Public URL: {}", state.site_url);

    let app = Router::new()
        // SPA entry point
        .route(
            "/",
            get(move || {
                let html = spa_index.clone();
                async move {
                    axum::http::Response::builder()
                        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
                        .header(header::PRAGMA, "no-cache")
                        .body(axum::body::Body::new(html))
                        .unwrap()
                }
            }),
        )
        // Static assets
        .nest_service("/assets", ServeDir::new(dist_path.join("assets")))
        // Server-rendered pages (backward compatible)
        .route("/login", get(login_page))
        .route("/portfolio", get(portfolio_page))
        // OAuth routes
        .route("/api/oauth/authorize", get(oauth_authorize))
        .route("/api/oauth/callback", get(oauth_callback))
        .route("/api/oauth/token", post(refresh_token))
        .route("/api/auth/login", post(login))
        .route("/api/auth/permissions", get(get_permissions))
        // User routes
        .route("/api/user/me", get(get_current_user))
        .route("/api/user/profile/{username}", get(get_public_profile))
        .route("/api/user/profile", put(update_profile))
        .route("/api/user/check-name", get(check_name_available))
        .route("/api/user/verify", get(verify_token))
        .route("/api/logout", post(logout))
        // Team routes
        .route("/api/team/members", get(get_team_members))
        .route("/api/team/requests", get(get_pending_requests))
        .route("/api/team/apply", post(apply_to_join))
        .route(
            "/api/team/review/{request_id}/{action}",
            post(review_application),
        )
        .route("/api/team/members/role/{user_id}", post(change_member_role))
        .route("/api/team/members/remove/{user_id}", post(remove_member))
        // Problem routes
        .route("/api/problems", get(get_problems))
        .route("/api/problems", post(submit_problem))
        .route("/api/problems/detail/{problem_id}", get(get_problem_detail))
        .route("/api/problems/claim/{problem_id}", post(claim_problem))
        .route("/api/problems/unclaim/{problem_id}", post(unclaim_problem))
        .route(
            "/api/problems/verifier-solution/{problem_id}",
            post(submit_verifier_solution),
        )
        .route(
            "/api/problems/visibility/{problem_id}",
            post(set_problem_visibility),
        )
        .route(
            "/api/problems/review/{problem_id}/{action}",
            post(review_problem),
        )
        .route(
            "/api/problems/admin/pending",
            get(get_pending_problems_admin),
        )
        .route(
            "/api/problems/admin/members",
            get(get_team_members_for_visibility),
        )
        .route(
            "/api/problems/{problem_id}",
            put(update_problem).delete(delete_problem),
        )
        // Contest routes
        .route("/api/contests", get(get_contests))
        .route("/api/contests", post(create_contest))
        .route("/api/contests/{contest_id}", delete(delete_contest))
        .route("/api/contests/{contest_id}", put(update_contest))
        .route(
            "/api/contests/{contest_id}/status",
            post(set_contest_status),
        )
        .route(
            "/api/contests/{contest_id}/problems",
            get(get_contest_problems),
        )
        .route(
            "/api/contests/{contest_id}/problem-order",
            post(set_problem_order),
        )
        .route(
            "/api/problems/contest/{problem_id}",
            post(set_problem_contest),
        )
        // Site info
        .route("/api/site/info", get(get_site_info))
        .route("/api/site/description", put(update_site_description))
        .route("/api/site/difficulties", get(get_difficulties))
        // Unified Post routes (primary)
        .route("/api/posts", get(get_posts).post(create_post))
        .route(
            "/api/posts/{id}",
            get(get_post_detail).put(update_post).delete(delete_post),
        )
        .route("/api/posts/{id}/reply", post(reply_to_post))
        .route(
            "/api/posts/{id}/reply/{reply_id}",
            delete(delete_post_reply),
        )
        .route("/api/posts/{id}/react", post(react_to_post))
        .route(
            "/api/posts/{id}/reply/{reply_id}/react",
            post(react_to_reply),
        )
        // Community (unified) routes
        .route("/api/community/posts", get(get_community_posts))
        // Tags & emojis
        .route("/api/posts/tags", get(get_discussion_tags))
        .route("/api/posts/emojis", get(get_discussion_emojis))
        // Legacy compat routes
        .route("/api/discussions", get(get_posts).post(create_post))
        .route(
            "/api/discussions/{id}",
            get(get_post_detail).put(update_post).delete(delete_post),
        )
        .route("/api/discussions/{id}/reply", post(reply_to_post))
        .route(
            "/api/discussions/{id}/reply/{reply_id}",
            delete(delete_post_reply),
        )
        .route("/api/discussions/{id}/react", post(react_to_post))
        .route(
            "/api/discussions/{id}/reply/{reply_id}/react",
            post(react_to_reply),
        )
        .route("/api/discussions/tags", get(get_discussion_tags))
        .route("/api/discussions/emojis", get(get_discussion_emojis))
        // Legacy suggestion routes
        .route(
            "/api/suggestions",
            get(get_suggestions).post(create_suggestion),
        )
        .route(
            "/api/suggestions/{id}",
            get(get_suggestion_detail)
                .put(update_suggestion)
                .delete(delete_suggestion),
        )
        .route("/api/suggestions/{id}/reply", post(reply_to_suggestion))
        .route(
            "/api/suggestions/{id}/reply/{reply_id}",
            delete(delete_suggestion_reply),
        )
        // Legacy announcement routes
        .route(
            "/api/announcements",
            get(get_announcements).post(create_announcement),
        )
        .route(
            "/api/announcements/{id}",
            get(get_announcement_detail)
                .put(update_announcement)
                .delete(delete_announcement),
        )
        // Notification routes
        .route("/api/notifications", get(get_notifications))
        .route("/api/notifications/read/{id}", post(mark_notification_read))
        .route(
            "/api/notifications/read-all",
            post(mark_all_notifications_read),
        )
        // Admin config
        .route("/api/admin/config", get(get_config).put(update_config))
        .route("/api/admin/restart", post(restart_service))
        // Admin backup
        .route("/api/admin/backup", post(create_backup))
        .route("/api/admin/backups", get(list_backups))
        .route("/api/admin/backup/restore/{name}", post(restore_backup))
        .route("/api/admin/backup/download/{name}", get(download_backup))
        .route("/api/admin/backup/{name}", delete(delete_backup))
        // Admin showcase
        .route(
            "/api/admin/showcase",
            get(get_showcase_config).put(update_showcase_config),
        )
        // Admin export
        .route("/api/admin/export/data", get(export_data))
        .route("/api/admin/export/config", get(export_config))
        // Admin import
        .route("/api/admin/import/data", post(import_data))
        .route("/api/admin/import/config", post(import_config))
        // Admin audit log
        .route("/api/admin/audit-log", get(get_audit_log))
        // Admin user management
        .route("/api/admin/users", get(admin_list_users))
        .route(
            "/api/admin/users/{user_id}/role",
            post(admin_change_user_role),
        )
        .route("/api/admin/users/{user_id}/remove", post(admin_remove_user))
        .route("/api/admin/users/{user_id}/groups", put(set_user_groups))
        .route(
            "/api/admin/users/{user_id}/permissions",
            put(set_user_permissions),
        )
        // Admin member groups
        .route("/api/admin/groups", get(list_groups).post(create_group))
        .route(
            "/api/admin/groups/{group_id}",
            put(update_group).delete(delete_group),
        )
        // Admin problem ACL
        .route("/api/admin/problems/{problem_id}/acl", put(set_problem_acl))
        // Admin unified resource ACL
        .route(
            "/api/admin/acl/{resource_type}/{resource_id}",
            put(set_resource_acl),
        )
        .layer(cors)
        .layer(CompressionLayer::new())
        .with_state(state);

    // SPA fallback: 所有非 API 路径返回 index.html（前端路由）
    let app = app.fallback(move || {
        let html = spa_index_fallback.clone();
        async move {
            axum::http::Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
                .header(header::PRAGMA, "no-cache")
                .body(axum::body::Body::new(html))
                .unwrap()
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!(
        "Open {} in your browser",
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
    );
    println!("Available routes:");
    println!("  /              → SPA frontend (React app)");
    println!("  /login         → Server-rendered login page (legacy)");
    println!("  /portfolio     → Server-rendered portfolio page (legacy)");
    println!("  /api/posts     → Unified post CRUD");
    println!("  /api/community/posts → Unified community feed");
    println!("  /api/discussions/*   → Legacy compat routes");
    println!("  /api/suggestions/*   → Legacy compat routes");
    println!("  /api/announcements/* → Legacy compat routes");

    axum::serve(listener, app).await.unwrap();
}
