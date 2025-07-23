use std::path::Path;

use serde::{Deserialize, Serialize};

/// Crate-specific configuration in the manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    /// Name of the crate
    pub name: String,
    /// Version of the crate
    #[serde(default = "default_version")]
    pub version: String,
    /// Entry point file (relative to src/)
    pub entry_point: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

#[cfg(test)]
impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            entry_point: "main.cm".to_string(),
        }
    }
}

impl ProjectManifest {
    /// Load manifest from a file path
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_file_content(&content)
    }

    pub fn from_file_content(content: &str) -> anyhow::Result<Self> {
        let manifest: Self = toml::from_str(content).map_err(|e| {
            tracing::error!("Failed to parse TOML: {}", e);
            e
        })?;
        Ok(manifest)
    }
}
