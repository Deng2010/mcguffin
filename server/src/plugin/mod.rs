pub mod builtins;
pub mod manager;
pub mod trait_;

pub use manager::PluginManager;
pub use trait_::{
    NavPlacement, PermissionDef, Plugin, PluginContext, PluginManifest, PluginResponse,
    PluginRouteDef, PluginSource,
};
