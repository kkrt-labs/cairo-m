use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, trace};

use crate::{CairoMToml, CrateId, Project, SourceLayout, Workspace};

/// Discovers a Cairo-M project from a given path
///
/// This function will:
/// 1. Search upward from the given path to find a cairom.toml file
/// 2. Parse the manifest to get project configuration
/// 3. Return a Project struct with all necessary information
///
/// ## Arguments
/// * `start_path` - Path to start searching from (can be a file or directory)
///
/// ## Returns
/// * `Ok(Some(Project))` if a project is found
/// * `Ok(None)` if no project is found
/// * `Err` if there's an error during discovery
pub fn discover_project(start_path: &Path) -> Result<Option<Project>> {
    let manifest_path = find_project_manifest(start_path)?;

    match manifest_path {
        Some(manifest_path) => {
            debug!("Found project manifest at: {}", manifest_path.display());
            let project = load_project_from_manifest(&manifest_path)?;
            Ok(Some(project))
        }
        None => {
            trace!(
                "No project manifest found starting from: {}",
                start_path.display()
            );
            Ok(None)
        }
    }
}

/// Discovers a workspace containing multiple projects
///
/// ## Arguments
/// * `workspace_root` - Root directory of the workspace
///
/// ## Returns
/// * `Workspace` containing all discovered projects
pub fn discover_workspace(workspace_root: &Path) -> Result<Workspace> {
    let mut projects = HashMap::new();
    let mut name_to_id = HashMap::new();
    let mut next_id = 0;

    // Walk the directory tree looking for cairom.toml files
    use ignore::WalkBuilder;
    let walker = WalkBuilder::new(workspace_root).follow_links(false).build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if path.file_name() == Some(std::ffi::OsStr::new("cairom.toml")) {
            match load_project_from_manifest(path) {
                Ok(project) => {
                    let crate_id = CrateId(next_id);
                    next_id += 1;

                    name_to_id.insert(project.name.clone(), crate_id);
                    projects.insert(crate_id, project);
                }
                Err(e) => {
                    // Log but don't fail on individual project errors
                    debug!("Failed to load project from {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(Workspace {
        root_directory: workspace_root.to_owned(),
        projects,
        name_to_id,
    })
}

/// Find the project manifest (cairom.toml) starting from a given path
fn find_project_manifest(start_path: &Path) -> Result<Option<PathBuf>> {
    let start_dir = if start_path.is_file() {
        start_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path: no parent directory"))?
    } else {
        start_path
    };

    let mut current = start_dir;

    loop {
        let manifest_path = current.join("cairom.toml");
        if manifest_path.exists() {
            return Ok(Some(manifest_path));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(None),
        }
    }
}

/// Load a project from its manifest file
fn load_project_from_manifest(manifest_path: &Path) -> Result<Project> {
    let manifest = CairoMToml::from_path(manifest_path)
        .with_context(|| format!("Failed to parse manifest at {}", manifest_path.display()))?;

    let root_directory = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Manifest has no parent directory"))?
        .to_owned();

    let source_layout = SourceLayout::default();

    let entry_point = manifest
        .crate_manifest
        .entry_point
        .map(|ep| source_layout.src_dir.join(ep));

    Ok(Project {
        manifest_path: manifest_path.to_owned(),
        root_directory,
        name: manifest.crate_manifest.name,
        source_layout,
        entry_point,
    })
}

/// Find the entry point file for a project
///
/// Looks for main.cm or lib.cm in the source directory
pub fn find_entry_point(project: &Project) -> Option<PathBuf> {
    if let Some(ref entry_point) = project.entry_point {
        let full_path = project.root_directory.join(entry_point);
        if full_path.exists() {
            return Some(full_path);
        }
    }

    // Check for default entry points
    let src_dir = project.source_directory();
    for entry_name in ["main.cm", "lib.cm"] {
        let entry_path = src_dir.join(entry_name);
        if entry_path.exists() {
            return Some(entry_path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_discover_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("my_project");
        fs::create_dir(&project_dir).unwrap();

        // Create a simple manifest
        let manifest_content = r#"
            name = "test_project"
            version = "0.1.0"
        "#;
        fs::write(project_dir.join("cairom.toml"), manifest_content).unwrap();

        // Create src directory
        fs::create_dir(project_dir.join("src")).unwrap();

        // Test discovery from project root
        let project = discover_project(&project_dir).unwrap().unwrap();
        assert_eq!(project.name, "test_project");
        assert_eq!(project.root_directory, project_dir);

        // Test discovery from subdirectory
        let sub_dir = project_dir.join("src");
        let project2 = discover_project(&sub_dir).unwrap().unwrap();
        assert_eq!(project2.name, "test_project");
    }
}
