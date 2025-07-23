use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::debug;

use crate::{Project, ProjectId, ProjectManifest, SourceLayout, Workspace};

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
            debug!(
                "No project manifest found starting from: {}, treating as standalone file",
                start_path.display()
            );
            let project = setup_as_standalone_file(start_path)?;
            Ok(Some(project))
        }
    }
}

/// Quick workaround to setup a standalone file as a project
///
/// This function will:
/// 1. Create a temporary directory
/// 2. Create a src directory
/// 3. Copy the file to the src directory
/// 4. Create a manifest file in the root directory
fn setup_as_standalone_file(file_path: &Path) -> Result<Project> {
    let canonical_file_path = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());
    let file_stem = canonical_file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    // For standalone files, the entry point is just the file name
    let manifest_content = format!(
        r#"name = "{file_stem}"
version = "0.1.0"
entry_point = "{file_stem}.cm"
"#
    );
    let project_manifest =
        ProjectManifest::from_file_content(&manifest_content).expect("Failed to parse manifest");

    let project = Project {
        config: project_manifest,
        root_directory: canonical_file_path.clone(),
        name: file_stem.to_string(),
        source_layout: SourceLayout::default(),
    };

    Ok(project)
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

        if path.file_name() == Some(std::ffi::OsStr::new(crate::MANIFEST_FILE_NAME)) {
            match load_project_from_manifest(path) {
                Ok(project) => {
                    let crate_id = ProjectId(next_id);
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
pub fn find_project_manifest(start_path: &Path) -> Result<Option<PathBuf>> {
    let start_dir = if start_path.is_file() {
        start_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path: no parent directory"))?
    } else {
        start_path
    };

    let mut current = start_dir;

    loop {
        let manifest_path = current.join(crate::MANIFEST_FILE_NAME);
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
    let manifest = ProjectManifest::from_path(manifest_path)
        .with_context(|| format!("Failed to parse manifest at {}", manifest_path.display()))?;

    let project_folder = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Manifest has no parent directory"))?
        .to_owned();

    let source_layout = SourceLayout::default();

    // The entry_point in the manifest is relative to the project root
    // It might include "src/" prefix or not, so we need to handle both cases
    let entry_point_path = Path::new(&manifest.entry_point);
    let project_root_file = if entry_point_path.is_absolute() {
        entry_point_path.to_path_buf()
    } else if entry_point_path.starts_with(&source_layout.src_dir) {
        // Entry point already includes src/ prefix
        project_folder.join(&manifest.entry_point)
    } else {
        // Entry point doesn't include src/ prefix, add it
        project_folder
            .join(&source_layout.src_dir)
            .join(&manifest.entry_point)
    };

    let manifest_name = manifest.name.clone();

    Ok(Project {
        config: manifest,
        root_directory: project_root_file,
        name: manifest_name,
        source_layout,
    })
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
            entry_point = "main.cm"
        "#;
        fs::write(
            project_dir.join(crate::MANIFEST_FILE_NAME),
            manifest_content,
        )
        .unwrap();

        // Create src directory
        fs::create_dir(project_dir.join("src")).unwrap();

        // Test discovery from project root
        let project = discover_project(&project_dir).unwrap().unwrap();
        assert_eq!(project.name, "test_project");
        assert_eq!(project.root_directory, project_dir.join("src/main.cm"));

        // Test discovery from subdirectory
        let sub_dir = project_dir.join("src");
        let project2 = discover_project(&sub_dir).unwrap().unwrap();
        assert_eq!(project2.name, "test_project");
    }
}
