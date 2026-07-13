use axum::{
    http::header,
    routing::get,
};
use mcguffin_server_lib::{build_router, AppState, resolve_config_path};
use std::net::SocketAddr;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::new().await;

    // Start auto-backup — read interval and retention from config.toml
    let state_for_backup = std::sync::Arc::new(state.clone());
    let backup_config = {
        let path = resolve_config_path();
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let doc = raw.parse::<toml_edit::DocumentMut>().unwrap_or_default();
        let interval_secs = doc
            .get("backup")
            .and_then(|s| s.get("interval_minutes"))
            .and_then(|v| v.as_integer())
            .map(|n| n as u64 * 60)
            .unwrap_or(3600);
        let retention = doc
            .get("backup")
            .and_then(|s| s.get("retention_count"))
            .and_then(|v| v.as_integer())
            .map(|n| n as usize)
            .unwrap_or(48);
        (interval_secs, retention)
    };
    state_for_backup.start_auto_backup(backup_config.0, backup_config.1);

    mcguffin_server_lib::handlers::post::truncate_existing_posts(&state).await;
    mcguffin_server_lib::handlers::post::cleanup_orphan_reactions(&state).await;

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

    // Frontend dist path — 优先 MCGUFFIN_WEB_DIST 环境变量，其次相对路径探测
    let dist_path = std::env::var("MCGUFFIN_WEB_DIST")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            ["../web/dist", "web/dist", "../dist"]
                .iter()
                .map(std::path::PathBuf::from)
                .find(|p| p.join("index.html").exists())
        })
        .unwrap_or_else(|| {
            eprintln!("错误: 找不到前端构建产物 dist/index.html");
            eprintln!("  请设置 MCGUFFIN_WEB_DIST 环境变量或先运行 'bun run build'");
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

    let app = build_router(state.clone())
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
        .nest_service("/assets", ServeDir::new(assets_path))
        // SPA fallback: 所有非 API 路径返回 index.html（前端路由）
        .fallback(move || {
            let html = spa_index_fallback.clone();
            async move {
                axum::http::Response::builder()
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
                    .header(header::PRAGMA, "no-cache")
                    .body(axum::body::Body::new(html))
                    .unwrap()
            }
        })
        .layer(cors)
        .layer(CompressionLayer::new());

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
    println!("  /api/v1/*      → Canonical API routes");
    println!("  /api/*         → Backward-compatible API routes");

    axum::serve(listener, app).await.unwrap();
}
