use std::path::Path;

use serde::{Deserialize, Serialize};

/// Cairo-M project manifest (cairom.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CairoMToml {
    #[serde(flatten)]
    pub crate_manifest: CrateManifest,
}

/// Crate-specific configuration in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateManifest {
    /// Name of the crate
    pub name: String,
    /// Version of the crate
    #[serde(default = "default_version")]
    pub version: String,
    /// Entry point file (relative to src/)
    pub entry_point: Option<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

impl CairoMToml {
    /// Load manifest from a file path
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = toml::from_str(&content).map_err(|e| {
            tracing::error!("Failed to parse TOML: {}", e);
            e
        })?;
        Ok(manifest)
    }
}
