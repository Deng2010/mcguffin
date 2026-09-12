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
// 注意：domain 的 re-export 由 `pub use types::*` 提供（types.rs = `pub use crate::domain::*`），
// 此处不要再写 `pub use domain::*` —— rustc 1.98 起会判定该 glob 冗余并报 unused_imports
// （在 `-D warnings` 下直接失败）。需要根路径类型时用 `mcguffin_server_lib::domain::X`。
pub use routes::build_router;
pub use state::{resolve_config_path, AppState};
pub use types::*;

/// 读取配置中的服务端口（供服务入口使用）。
pub fn configured_port() -> u16 {
    infra::config::load_config().server.port
}
