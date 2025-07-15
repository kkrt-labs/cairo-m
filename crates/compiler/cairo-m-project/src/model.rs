use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Unique identifier for a crate within a workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrateId(pub usize);

/// Represents a Cairo-M project structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Path to the project manifest (cairom.toml)
    pub manifest_path: PathBuf,
    /// Root directory of the project
    pub root_directory: PathBuf,
    /// Name of the project
    pub name: String,
    /// Source layout configuration
    pub source_layout: SourceLayout,
    /// Entry point file (if specified)
    pub entry_point: Option<PathBuf>,
}

impl Project {
    /// Get the absolute path to the source directory
    pub fn source_directory(&self) -> PathBuf {
        self.root_directory.join(&self.source_layout.src_dir)
    }

    /// Check if a path belongs to this project
    pub fn contains_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root_directory)
    }

    /// Get all source files in the project
    pub fn source_files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let src_dir = self.source_directory();
        let mut files = Vec::new();

        use ignore::WalkBuilder;
        let walker = WalkBuilder::new(&src_dir).follow_links(false).build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("cm") {
                files.push(path.to_owned());
            }
        }

        Ok(files)
    }
}

/// Source layout configuration for a project
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLayout {
    /// Source directory (default: "src")
    pub src_dir: PathBuf,
    /// Test directory (default: "tests")
    pub test_dir: PathBuf,
}

impl Default for SourceLayout {
    fn default() -> Self {
        Self {
            src_dir: PathBuf::from("src"),
            test_dir: PathBuf::from("tests"),
        }
    }
}

/// Represents a workspace containing multiple projects
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Root directory of the workspace
    pub root_directory: PathBuf,
    /// Projects in the workspace, indexed by crate ID
    pub projects: HashMap<CrateId, Project>,
    /// Mapping from project name to crate ID
    pub name_to_id: HashMap<String, CrateId>,
}
