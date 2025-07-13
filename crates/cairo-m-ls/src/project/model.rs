use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cairo_m_compiler_parser::SourceFile;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;
use tracing::{debug, info};

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

/// Central model for all loaded projects
pub struct ProjectModel {
    /// Map from project root to loaded crate
    crates: Arc<RwLock<HashMap<PathBuf, Crate>>>,
    /// Map from file URL to project root
    file_to_project: Arc<RwLock<HashMap<Url, PathBuf>>>,
    /// Map from project root to ProjectCrate ID in the database
    project_crate_ids: Arc<RwLock<HashMap<PathBuf, ProjectCrate>>>,
}

impl ProjectModel {
    pub fn new() -> Self {
        Self {
            crates: Arc::new(RwLock::new(HashMap::new())),
            file_to_project: Arc::new(RwLock::new(HashMap::new())),
            project_crate_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load a new crate into the model
    /// Returns URLs of files that moved from other projects (for diagnostics clearing)
    #[allow(clippy::future_not_send)]
    pub async fn load_crate(
        &self,
        crate_info: CrateInfo,
        file_paths: Vec<PathBuf>,
        db: &mut AnalysisDatabase,
        get_source_file: impl Fn(&mut AnalysisDatabase, &Url) -> Option<SourceFile>,
    ) -> Result<Vec<Url>, String> {
        info!(
            "Loading crate: {} with {} files",
            crate_info.name,
            file_paths.len()
        );

        // Use the provided closure to get or create SourceFile entities
        let mut files = HashMap::new();
        for path in file_paths {
            if let Ok(uri) = Url::from_file_path(&path) {
                if let Some(source_file) = get_source_file(db, &uri) {
                    files.insert(path, source_file);
                }
            }
        }

        // Find main file
        let main_file = self.find_main_file(&crate_info, &files);

        let crate_obj = Crate {
            info: crate_info.clone(),
            main_file,
            files: files.clone(),
        };

        // Apply to database
        self.apply_crate_to_db(&crate_obj, db).await?;

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
                    // Check if file was in a different project
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

    /// Load a standalone file (no project)
    /// Returns URLs of files that moved from other projects (for diagnostics clearing)
    #[allow(clippy::future_not_send)]
    pub async fn load_standalone(
        &self,
        file_path: PathBuf,
        db: &mut AnalysisDatabase,
        get_source_file: impl Fn(&mut AnalysisDatabase, &Url) -> Option<SourceFile>,
    ) -> Result<Vec<Url>, String> {
        info!("Loading standalone file: {}", file_path.display());

        let uri = Url::from_file_path(&file_path)
            .map_err(|_| format!("Invalid file path: {}", file_path.display()))?;

        let source_file = get_source_file(db, &uri)
            .ok_or_else(|| format!("Failed to get source file for: {}", file_path.display()))?;

        // For standalone files, create a minimal crate with unique root
        // Use the file path itself with a ".standalone" extension to ensure uniqueness
        let unique_root = file_path.with_extension("standalone");
        let crate_info = CrateInfo {
            name: file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("standalone")
                .to_string(),
            root: unique_root,
        };

        let mut files = HashMap::new();
        files.insert(file_path.clone(), source_file);

        let crate_obj = Crate {
            info: crate_info.clone(),
            main_file: Some(file_path),
            files,
        };

        self.apply_crate_to_db(&crate_obj, db).await?;

        // Update internal state
        {
            let mut crates = self.crates.write().await;
            crates.insert(crate_info.root.clone(), crate_obj);
        }

        // Check if file moved from another project
        let mut moved_files = Vec::new();
        {
            let mut file_to_project = self.file_to_project.write().await;
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
    ) -> Option<PathBuf> {
        // Look for main.cm or lib.cm
        let main_path = crate_info.root.join("main.cm");
        if files.contains_key(&main_path) {
            return Some(main_path);
        }

        let lib_path = crate_info.root.join("lib.cm");
        if files.contains_key(&lib_path) {
            return Some(lib_path);
        }

        // Fallback to first file
        files.keys().next().cloned()
    }

    #[allow(clippy::future_not_send)]
    async fn apply_crate_to_db(
        &self,
        crate_obj: &Crate,
        db: &AnalysisDatabase,
    ) -> Result<(), String> {
        debug!("Applying crate {} to database", crate_obj.info.name);

        // Create the unified ProjectCrate input
        let project_crate = ProjectCrate::new(
            db,
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
        );

        // Store the ProjectCrate for later retrieval
        {
            let mut project_crate_ids = self.project_crate_ids.write().await;
            project_crate_ids.insert(crate_obj.info.root.clone(), project_crate);
        }

        Ok(())
    }
}

impl Default for ProjectModel {
    fn default() -> Self {
        Self::new()
    }
}
