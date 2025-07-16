use std::path::{Path, PathBuf};

use cairo_m_project::discover_project;
use tracing::debug;

/// Represents different types of project manifests
#[derive(Debug, Clone)]
pub enum ProjectManifestPath {
    /// A cairom.toml file was found
    CairoM(PathBuf),
}

impl ProjectManifestPath {
    /// Discover project manifest starting from a given file path
    ///
    /// This method uses cairo-m-project's discovery mechanism to find and parse
    /// cairom.toml files, ensuring consistency with the compiler's project model.
    pub fn discover(file_path: &Path) -> Option<Self> {
        debug!("Discovering project manifest for: {}", file_path.display());

        // Use cairo-m-project's discovery mechanism
        match discover_project(file_path) {
            Ok(Some(project)) => Some(Self::CairoM(project.manifest_path)),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Project discovery error details: {:?}", e);
                None
            }
        }
    }

    /// Get the manifest file path
    pub fn path(&self) -> &Path {
        match self {
            Self::CairoM(path) => path,
        }
    }
}
