use axum::{
    response::Html,
    routing::{get, post, delete, put},
    Router,
};
use mcguffin_server_lib::*;
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::new();

    let frontend_origin: axum::http::HeaderValue = state.site_url
        .parse()
        .expect("SITE_URL must be a valid origin");

    let local_vite_origin: axum::http::HeaderValue =
        "http://localhost:5173".parse().expect("valid origin");

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            frontend_origin,
            local_vite_origin,
        ]))
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

    // Frontend dist path (relative to the server working directory)
    let dist_path = std::path::PathBuf::from("../mcguffin-web/dist");
    let spa_index = std::fs::read_to_string(dist_path.join("index.html"))
        .expect("Failed to read dist/index.html — run 'bun run build' in mcguffin-web first");

    println!("Serving frontend SPA from: {:?}", dist_path.canonicalize().unwrap_or(dist_path.clone()));
    println!("McGuffin Server running on http://0.0.0.0:3000");
    println!("Public URL: {}", state.site_url);

    let app = Router::new()
        // SPA entry point — serve the built frontend at root
        .route("/", get(move || {
            let html = spa_index.clone();
            async move { Html(html) }
        }))
        // Static assets from the production build
        .nest_service("/assets", ServeDir::new("../mcguffin-web/dist/assets"))
        // Server-rendered pages (backward compatible)
        .route("/login", get(login_page))
        .route("/portfolio", get(portfolio_page))
        // OAuth routes
        .route("/api/oauth/authorize", get(oauth_authorize))
        .route("/api/oauth/callback", get(oauth_callback))
        .route("/api/oauth/token", post(refresh_token))
        .route("/api/auth/admin-login", post(admin_login))
        // User routes
        .route("/api/user/me", get(get_current_user))
        .route("/api/user/profile", put(update_profile))
        .route("/api/user/verify", get(verify_token))
        .route("/api/logout", post(logout))
        // Team routes
        .route("/api/team/members", get(get_team_members))
        .route("/api/team/requests", get(get_pending_requests))
        .route("/api/team/apply", post(apply_to_join))
        .route("/api/team/review/:request_id/:action", post(review_application))
        .route("/api/team/members/role/:user_id", post(change_member_role))
        .route("/api/team/members/remove/:user_id", post(remove_member))
        // Problem routes
        .route("/api/problems", get(get_problems))
        .route("/api/problems", post(submit_problem))
        .route("/api/problems/detail/:problem_id", get(get_problem_detail))
        .route("/api/problems/claim/:problem_id", post(claim_problem))
        .route("/api/problems/unclaim/:problem_id", post(unclaim_problem))
        .route("/api/problems/verifier-solution/:problem_id", post(submit_verifier_solution))
        .route("/api/problems/visibility/:problem_id", post(set_problem_visibility))
        .route("/api/problems/review/:problem_id/:action", post(review_problem))
        .route("/api/problems/admin/pending", get(get_pending_problems_admin))
        .route("/api/problems/admin/members", get(get_team_members_for_visibility))
        .route("/api/problems/:problem_id", put(update_problem).delete(delete_problem))
        // Contest routes
        .route("/api/contests", get(get_contests))
        .route("/api/contests", post(create_contest))
        .route("/api/contests/:contest_id", delete(delete_contest))
        .route("/api/contests/:contest_id", put(update_contest))
        .route("/api/contests/:contest_id/status", post(set_contest_status))
        .route("/api/contests/:contest_id/problems", get(get_contest_problems))
        .route("/api/contests/:contest_id/problem-order", post(set_problem_order))
        // Problem contest assignment
        .route("/api/problems/contest/:problem_id", post(set_problem_contest))
        // Site info
        .route("/api/site/info", get(get_site_info))
        .route("/api/site/description", put(update_site_description))
        .route("/api/site/difficulties", get(get_difficulties))
        // Admin config (superadmin only)
        .route("/api/admin/config", get(get_config).put(update_config))
        .route("/api/admin/restart", post(restart_service))
        // Admin backup (superadmin only)
        .route("/api/admin/backup", post(create_backup))
        .route("/api/admin/backups", get(list_backups))
        .route("/api/admin/backup/restore/:name", post(restore_backup))
        .route("/api/admin/backup/:name", delete(delete_backup))
        // Admin export (superadmin only)
        .route("/api/admin/export/data", get(export_data))
        .route("/api/admin/export/config", get(export_config))
        .layer(cors)
        .layer(CompressionLayer::new())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("Open {} in your browser", std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()));
    println!("Available routes:");
    println!("  /              → SPA frontend (React app)");
    println!("  /login         → Server-rendered login page (legacy)");
    println!("  /portfolio     → Server-rendered portfolio page (legacy)");
    println!("  /api/*         → API endpoints");

    axum::serve(listener, app).await.unwrap();
}
