use std::path::{Path, PathBuf};

use tracing::debug;

/// Represents different types of project manifests
#[derive(Debug, Clone)]
pub enum ProjectManifestPath {
    /// A cairom.toml file was found
    CairoM(PathBuf),
}

impl ProjectManifestPath {
    /// Discover project manifest starting from a given file path
    pub fn discover(file_path: &Path) -> Option<Self> {
        debug!("Discovering project manifest for: {}", file_path.display());

        // Start from the file's directory
        let start_dir = if file_path.is_file() {
            file_path.parent()?
        } else {
            file_path
        };

        // Look for cairom.toml
        if let Some(cairom_path) = Self::find_manifest_file(start_dir, "cairom.toml") {
            return Some(Self::CairoM(cairom_path));
        }

        None
    }

    /// Find a manifest file by walking up the directory tree
    fn find_manifest_file(start_dir: &Path, filename: &str) -> Option<PathBuf> {
        let mut current = start_dir;

        loop {
            let manifest_path = current.join(filename);
            if manifest_path.exists() && manifest_path.is_file() {
                debug!("Found manifest: {}", manifest_path.display());
                return Some(manifest_path);
            }

            // Move up to parent directory
            current = current.parent()?;
        }
    }

    /// Get the project root directory
    pub fn project_root(&self) -> Option<&Path> {
        match self {
            Self::CairoM(path) => path.parent(),
        }
    }

    /// Get the manifest file path
    pub fn path(&self) -> &Path {
        match self {
            Self::CairoM(path) => path,
        }
    }
}
