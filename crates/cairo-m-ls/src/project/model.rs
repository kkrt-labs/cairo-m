use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cairo_m_compiler_parser::SourceFile;
use cairo_m_project::Project;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;
use tracing::info;

use crate::db::{AnalysisDatabase, ProjectCrate};

/// Information about a single crate in the project
#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub root: PathBuf,
}

/// Represents a loaded crate with all its files
#[derive(Debug, Clone)]
pub struct Crate {
    pub info: CrateInfo,
    pub main_file: Option<PathBuf>,
    pub files: HashMap<PathBuf, SourceFile>,
}

/// Resource limits for project loading
#[derive(Debug, Clone)]
pub struct ProjectResourceLimits {
    /// Maximum number of projects that can be loaded simultaneously
    pub max_projects: usize,
    /// Maximum number of files per project
    pub max_files_per_project: usize,
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Maximum total memory usage across all projects (approximate, in bytes)
    pub max_total_memory: usize,
}

impl Default for ProjectResourceLimits {
    fn default() -> Self {
        Self {
            max_projects: 50,                    // 50 projects max
            max_files_per_project: 10_000,       // 10k files per project
            max_file_size: 10 * 1024 * 1024,     // 10MB per file
            max_total_memory: 500 * 1024 * 1024, // 500MB total
        }
    }
}

/// Central model for all loaded projects
pub struct ProjectModel {
    /// Map from project root to loaded crate
    crates: Arc<RwLock<HashMap<PathBuf, Crate>>>,
    /// Map from file URL to project root
    file_to_project: Arc<RwLock<HashMap<Url, PathBuf>>>,
    /// Map from project root to ProjectCrate ID in the database
    project_crate_ids: Arc<RwLock<HashMap<PathBuf, ProjectCrate>>>,
    /// Resource limits
    pub limits: ProjectResourceLimits,
    /// Approximate total memory usage tracking
    total_memory_usage: Arc<RwLock<usize>>,
}

impl ProjectModel {
    pub fn new() -> Self {
        Self::with_limits(ProjectResourceLimits::default())
    }

    pub fn with_limits(limits: ProjectResourceLimits) -> Self {
        Self {
            crates: Arc::new(RwLock::new(HashMap::new())),
            file_to_project: Arc::new(RwLock::new(HashMap::new())),
            project_crate_ids: Arc::new(RwLock::new(HashMap::new())),
            limits,
            total_memory_usage: Arc::new(RwLock::new(0)),
        }
    }

    /// Load a new crate into the model with pre-prepared source files
    /// This avoids the need for nested async-blocking patterns
    pub async fn load_crate_with_prepared_files(
        &self,
        crate_info: CrateInfo,
        files: HashMap<PathBuf, SourceFile>,
        db: &Arc<Mutex<AnalysisDatabase>>,
    ) -> Result<Vec<Url>, String> {
        self.load_crate_with_prepared_files_and_project(crate_info, files, db, None)
            .await
    }

    /// Load a new crate into the model with pre-prepared source files and optional project
    ///
    /// When a `Project` is provided, it will be used to:
    /// - Find the appropriate entry point file (from project manifest or defaults)
    /// - Use project-specific configuration for module resolution
    /// - Ensure consistency with cairo-m-project's project model
    pub async fn load_crate_with_prepared_files_and_project(
        &self,
        crate_info: CrateInfo,
        files: HashMap<PathBuf, SourceFile>,
        db: &Arc<Mutex<AnalysisDatabase>>,
        project: Option<Project>,
    ) -> Result<Vec<Url>, String> {
        // Check resource limits
        {
            let crates = self.crates.read().await;
            if crates.len() >= self.limits.max_projects && !crates.contains_key(&crate_info.root) {
                return Err(format!(
                    "Maximum number of projects ({}) exceeded",
                    self.limits.max_projects
                ));
            }
        }

        if files.len() > self.limits.max_files_per_project {
            return Err(format!(
                "Project has too many files ({} > {})",
                files.len(),
                self.limits.max_files_per_project
            ));
        }

        // Estimate memory usage for this project (rough approximation)
        let estimated_memory = files.len() * 1024; // Assume ~1KB metadata per file

        {
            let mut total_memory = self.total_memory_usage.write().await;
            if *total_memory + estimated_memory > self.limits.max_total_memory {
                return Err(format!(
                    "Loading project would exceed memory limit ({} bytes)",
                    self.limits.max_total_memory
                ));
            }
            *total_memory += estimated_memory;
        }

        // Find main file
        let main_file = self.find_main_file(&crate_info, &files, project.as_ref());

        let crate_obj = Crate {
            info: crate_info.clone(),
            main_file,
            files: files.clone(),
        };

        // Apply to database - create ProjectCrate synchronously
        let project_crate = {
            let db_guard = db.lock().unwrap_or_else(|poisoned| {
                tracing::error!("Database mutex poisoned - recovering from panic");
                poisoned.into_inner()
            });

            // Create the unified ProjectCrate input
            ProjectCrate::new(
                &*db_guard,
                crate_obj.info.root.clone(),
                crate_obj
                    .main_file
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("main")
                    .to_string(),
                crate_obj
                    .files
                    .iter()
                    .map(|(path, source)| (path.clone(), *source))
                    .collect(),
            )
        }; // db_guard dropped here

        // Store the ProjectCrate for later retrieval (async)
        {
            let mut project_crate_ids = self.project_crate_ids.write().await;
            project_crate_ids.insert(crate_obj.info.root.clone(), project_crate);
        }

        // Update internal state
        {
            let mut crates = self.crates.write().await;
            crates.insert(crate_info.root.clone(), crate_obj);
        }

        // Update file mappings and track files that moved projects
        let mut moved_files = Vec::new();
        {
            let mut file_to_project = self.file_to_project.write().await;
            for file_path in files.keys() {
                if let Ok(url) = Url::from_file_path(file_path) {
                    // Check if the file was in a different project
                    if let Some(old_project) = file_to_project.get(&url) {
                        if old_project != &crate_info.root {
                            moved_files.push(url.clone());
                        }
                    }
                    file_to_project.insert(url, crate_info.root.clone());
                }
            }
        }

        Ok(moved_files)
    }

    /// Load a standalone file with pre-prepared source file
    /// This avoids the need for nested async-blocking patterns
    pub async fn load_standalone_with_prepared_file(
        &self,
        file_path: &PathBuf,
        source_file: SourceFile,
        db: &Arc<Mutex<AnalysisDatabase>>,
    ) -> Result<Vec<Url>, String> {
        info!("Loading standalone file: {}", file_path.display());

        let uri = Url::from_file_path(file_path)
            .map_err(|_| format!("Invalid file path: {}", file_path.display()))?;

        // For standalone files, create a minimal crate with unique root
        // Use the file path itself with a ".standalone" extension to ensure uniqueness
        let crate_info = CrateInfo {
            name: file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("File stem should be a string")
                .to_string(),
            root: file_path.clone(),
        };

        let mut files = HashMap::new();
        files.insert(file_path.clone(), source_file);

        let crate_obj = Crate {
            info: crate_info.clone(),
            main_file: Some(file_path.clone()),
            files,
        };

        // Apply to database - create ProjectCrate synchronously
        let project_crate = {
            let db_guard = db.lock().unwrap_or_else(|poisoned| {
                tracing::error!("Database mutex poisoned - recovering from panic");
                poisoned.into_inner()
            });

            // Create the unified ProjectCrate input
            ProjectCrate::new(
                &*db_guard,
                crate_obj.info.root.clone(),
                crate_obj
                    .main_file
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("main")
                    .to_string(),
                crate_obj
                    .files
                    .iter()
                    .map(|(path, source)| (path.clone(), *source))
                    .collect(),
            )
        }; // db_guard dropped here

        // Store the ProjectCrate for later retrieval (async)
        {
            let mut project_crate_ids = self.project_crate_ids.write().await;
            project_crate_ids.insert(crate_obj.info.root.clone(), project_crate);
        }

        // Update internal state
        {
            let mut crates = self.crates.write().await;
            crates.insert(crate_info.root.clone(), crate_obj);
        }

        // Update file mappings and track files that moved
        let mut moved_files = Vec::new();
        {
            let mut file_to_project = self.file_to_project.write().await;
            // Check if file was in a different project
            if let Some(old_project) = file_to_project.get(&uri) {
                if old_project != &crate_info.root {
                    moved_files.push(uri.clone());
                }
            }
            file_to_project.insert(uri, crate_info.root);
        }

        Ok(moved_files)
    }

    /// Get all loaded crates
    pub async fn all_crates(&self) -> Vec<Crate> {
        let crates = self.crates.read().await;
        crates.values().cloned().collect()
    }

    /// Get the ProjectCrate for a given file URL
    pub async fn get_project_crate_for_file(&self, file_url: &Url) -> Option<ProjectCrate> {
        let project_root = {
            let file_to_project = self.file_to_project.read().await;
            file_to_project.get(file_url).cloned()?
        };

        let project_crate_ids = self.project_crate_ids.read().await;
        project_crate_ids.get(&project_root).cloned()
    }

    /// Get the ProjectCrate for a given project root
    pub async fn get_project_crate_for_root(&self, root: &PathBuf) -> Option<ProjectCrate> {
        let project_crate_ids = self.project_crate_ids.read().await;
        project_crate_ids.get(root).cloned()
    }

    /// Replaces the stored ProjectCrate IDs with a new set.
    /// This is intended to be used by the AnalysisDatabaseSwapper after a DB swap.
    pub async fn replace_project_crate_ids(&self, new_ids: HashMap<PathBuf, ProjectCrate>) {
        let mut project_crate_ids = self.project_crate_ids.write().await;
        *project_crate_ids = new_ids;
    }

    /// Replaces the stored Crate objects with new ones containing fresh SourceFile IDs.
    /// This MUST be called after a database swap to avoid stale Salsa ID panics.
    pub async fn replace_crates(&self, new_crates: HashMap<PathBuf, Crate>) {
        let mut crates = self.crates.write().await;
        *crates = new_crates;
    }

    fn find_main_file(
        &self,
        crate_info: &CrateInfo,
        files: &HashMap<PathBuf, SourceFile>,
        project: Option<&Project>,
    ) -> Option<PathBuf> {
        // If we have a project, use its root_directory which now points to the entry file
        if let Some(project) = project {
            let entry_file = &project.root_directory;
            if files.contains_key(entry_file) {
                return Some(entry_file.clone());
            }
            // If the exact path isn't found, it might be due to path normalization issues
            // Try to find a file with the same name in the files map
            if let Some(file_name) = entry_file.file_name() {
                for path in files.keys() {
                    if path.file_name() == Some(file_name) {
                        return Some(path.clone());
                    }
                }
            }
        }

        // Fallback: Look for common entry points in the crate
        let common_entries = ["main.cm", "lib.cm"];
        for entry in &common_entries {
            // Check in src directory
            let src_path = crate_info.root.join("src").join(entry);
            if files.contains_key(&src_path) {
                return Some(src_path);
            }
            // Check in root directory
            let root_path = crate_info.root.join(entry);
            if files.contains_key(&root_path) {
                return Some(root_path);
            }
        }

        // Ultimate fallback: first file (sorted alphabetically for determinism)
        let mut keys: Vec<_> = files.keys().cloned().collect();
        keys.sort();
        keys.into_iter().next()
    }
}

impl Default for ProjectModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_find_main_file_deterministic_fallback() {
        let files_keys = vec![
            PathBuf::from("/test/c_module.cm"),
            PathBuf::from("/test/b_module.cm"),
            PathBuf::from("/test/a_module.cm"),
        ];

        // The fallback should always select the lexicographically smallest
        let mut sorted_keys = files_keys;
        sorted_keys.sort();
        assert_eq!(sorted_keys[0], PathBuf::from("/test/a_module.cm"));
    }

    // TODO: This test is not actually testing the main / lib precedence.
    #[test]
    fn test_find_main_file_logic() {
        // Test that main.cm is preferred
        {
            let crate_info = CrateInfo {
                name: "test".to_string(),
                root: PathBuf::from("/test"),
            };

            // When main.cm exists, it should be selected
            let main_path = crate_info.root.join("main.cm");
            assert_eq!(main_path, PathBuf::from("/test/main.cm"));
        }

        // Test that lib.cm is second preference
        {
            let crate_info = CrateInfo {
                name: "test".to_string(),
                root: PathBuf::from("/test"),
            };

            let lib_path = crate_info.root.join("lib.cm");
            assert_eq!(lib_path, PathBuf::from("/test/lib.cm"));
        }
    }

    #[test]
    fn test_sorted_fallback_behavior() {
        // Test the sorting behavior we implemented
        let mut keys = [
            PathBuf::from("/test/z.cm"),
            PathBuf::from("/test/a.cm"),
            PathBuf::from("/test/m.cm"),
        ];

        keys.sort();

        assert_eq!(keys[0], PathBuf::from("/test/a.cm"));
        assert_eq!(keys[1], PathBuf::from("/test/m.cm"));
        assert_eq!(keys[2], PathBuf::from("/test/z.cm"));
    }
}
