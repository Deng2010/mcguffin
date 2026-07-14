pub mod manager;
pub mod trait_;

pub use manager::PluginManager;
pub use trait_::{
    NavPlacement, PermissionDef, PluginContext, PluginManifest, PluginResponse, PluginRouteDef,
    PluginSource,
};
