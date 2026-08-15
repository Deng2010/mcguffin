pub mod db;
pub mod domain;
pub mod error;
pub mod handlers;
pub mod infra;
pub mod routes;
pub mod state;
pub mod types;
pub mod utils;

pub use db::*;
pub use domain::*;
pub use routes::build_router;
pub use state::{resolve_config_path, AppState};
pub use types::*;

/// 读取配置中的服务端口（供服务入口使用）。
pub fn configured_port() -> u16 {
    infra::config::load_config().server.port
}
