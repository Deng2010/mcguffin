use serde::{Deserialize, Serialize};

/// Metadata describing a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub permissions_needed: Vec<String>,
}

/// A permission declared by a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionDef {
    pub key: String,
    pub label: String,
    pub description: String,
}

/// Where a plugin frontend route should be surfaced.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavPlacement {
    Main,
    Admin,
    Hidden,
}

/// A frontend route contributed by a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginRouteDef {
    pub path: String,
    pub label: String,
    pub icon: Option<String>,
    pub required_permission: Option<String>,
    pub nav_placement: NavPlacement,
}
