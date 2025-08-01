use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::ProjectManifest;

/// Unique identifier for a crate within a workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectId(pub usize);

/// Represents a Cairo-M project structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Project configuration
    pub config: ProjectManifest,
    /// Root file of the project (main.cm or lib.cm)
    pub root_directory: PathBuf,
    /// Name of the project
    pub name: String,
    /// Source layout configuration
    pub source_layout: SourceLayout,
}

impl Project {
    /// Get the absolute path to the source directory. The `root_directory` is the file path to the `.cm` file that is the project root.
    pub fn source_directory(&self) -> PathBuf {
        self.root_directory.parent().unwrap().to_owned()
    }

    /// Check if a path belongs to this project
    pub fn contains_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root_directory)
    }

    /// Get all source files in the project
    pub fn source_files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let src_dir = self.root_directory.parent().unwrap();
        let src_dir_name = src_dir.file_name().unwrap().to_str().unwrap();

        // TODO: refactor with a stronger mechanism regarding standalone file detection
        //1. If it's not an `src` dir we're in the standalone file case and there's only one file, the crate root.
        if src_dir_name != "src" {
            tracing::info!(
                "Compiling standalone file: {}",
                self.root_directory.display()
            );
            return Ok(vec![self.root_directory.clone()]);
        }
        tracing::info!("Compiling project: {}", self.root_directory.display());

        //2. if it's a `src` dir, walk the files
        let mut files = Vec::new();

        use ignore::WalkBuilder;
        let walker = WalkBuilder::new(src_dir).follow_links(false).build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("cm") {
                files.push(path.to_owned());
            }
        }

        // If no files found in directory walk, it might be a standalone file
        if files.is_empty() {
            return Ok(vec![self.root_directory.clone()]);
        }

        Ok(files)
    }

    /// Convert a source file path to its module name representation.
    ///
    /// This is the canonical implementation for converting file paths to Cairo-M module names.
    /// It follows the convention where the file structure under `src/` directly maps to the
    /// module hierarchy, with directory separators replaced by `::`.
    ///
    /// For example:
    /// - `/project/src/main.cm` -> `main`
    /// - `/project/src/x/y.cm` -> `x::y`
    /// - `/project/src/a/b/c.cm` -> `a::b::c`
    ///
    /// ## Arguments
    /// * `path` - The absolute path to the source file
    ///
    /// ## Returns
    /// The module name representation if the path is within the project's source directory,
    /// or an error if the path is outside the project.
    pub fn module_name_from_path(&self, path: &Path) -> Result<String, String> {
        let src_dir = self.source_directory();

        // Strip the source directory prefix
        let relative_path = path
            .strip_prefix(&src_dir)
            .map_err(|e| format!("Path is not within project source directory: {}", e))?;

        // Get the file stem (filename without extension)
        let file_stem = relative_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid file name".to_string())?;

        // Build module path from parent directories
        if let Some(parent) = relative_path.parent() {
            if parent.as_os_str().is_empty() {
                // File is directly in src/
                Ok(file_stem.to_string())
            } else {
                // Convert path components to module path
                let parent_modules = parent
                    .components()
                    .filter_map(|c| match c {
                        std::path::Component::Normal(name) => {
                            Some(name.to_string_lossy().to_string())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("::");
                Ok(format!("{}::{}", parent_modules, file_stem))
            }
        } else {
            Ok(file_stem.to_string())
        }
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
    /// Projects in the workspace, indexed by project ID
    pub projects: HashMap<ProjectId, Project>,
    /// Mapping from project name to project ID
    pub name_to_id: HashMap<String, ProjectId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_name_from_path() {
        let project = Project {
            config: Default::default(),
            root_directory: PathBuf::from("/test/src/main.cm"),
            name: "test".to_string(),
            source_layout: SourceLayout::default(),
        };

        // Test file directly in src/
        assert_eq!(
            project
                .module_name_from_path(&PathBuf::from("/test/src/main.cm"))
                .unwrap(),
            "main"
        );

        // Test file in subdirectory
        assert_eq!(
            project
                .module_name_from_path(&PathBuf::from("/test/src/x/y.cm"))
                .unwrap(),
            "x::y"
        );

        // Test deeply nested file
        assert_eq!(
            project
                .module_name_from_path(&PathBuf::from("/test/src/a/b/c.cm"))
                .unwrap(),
            "a::b::c"
        );

        // Test file outside source directory
        assert!(project
            .module_name_from_path(&PathBuf::from("/other/file.cm"))
            .is_err());
    }
}
