//! mcguffin-server 服务入口
//!
//! 初始化 tracing 日志、加载 AppState、构建路由并启动 HTTP 服务。
//! 默认监听 :3000（端口由 config.toml 的 [server].port 控制）。

use std::net::SocketAddr;

use mcguffin_server_lib::{build_router, configured_port, error, AppState};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let port = configured_port();
    let state = AppState::new().await;

    // 层序（从外到内）：CORS → Trace → RequestId → 错误中间件 → CatchPanic → 路由。
    // CatchPanic 必须在最内层，这样 panic 兜底产生的 500 也会经过错误中间件补 request_id 并记录。
    let app = build_router(state)
        .layer(CatchPanicLayer::custom(error::panic_to_response))
        .layer(axum::middleware::from_fn(
            error::log_errors_and_inject_request_id,
        ))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("无法监听 {}: {}", addr, e));
    tracing::info!("McGuffin server listening on http://{}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("服务器运行失败");
}
