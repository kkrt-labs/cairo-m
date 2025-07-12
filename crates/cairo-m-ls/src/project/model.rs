use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use cairo_m_compiler::project_discovery::{ProjectDiscoveryConfig, discover_project_files};
use cairo_m_compiler_parser::SourceFile;
use tower_lsp::lsp_types::Url;
use tracing::{debug, info};

use crate::db::{AnalysisDatabase, ProjectCrate};

/// Information about a single crate in the project
#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
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
        ProjectModel {
            crates: Arc::new(RwLock::new(HashMap::new())),
            file_to_project: Arc::new(RwLock::new(HashMap::new())),
            project_crate_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load a new crate into the model
    pub fn load_crate(
        &self,
        crate_info: CrateInfo,
        db: &mut AnalysisDatabase,
    ) -> Result<(), String> {
        info!(
            "Loading crate: {} at {}",
            crate_info.name,
            crate_info.root.display()
        );

        // Discover project files
        let files = self.discover_crate_files(&crate_info)?;

        // Find main file
        let main_file = self.find_main_file(&crate_info, &files);

        let crate_obj = Crate {
            info: crate_info.clone(),
            main_file: main_file.clone(),
            files: files.clone(),
        };

        // Apply to database
        self.apply_crate_to_db(&crate_obj, db)?;

        // Update internal state
        {
            let mut crates = self.crates.write().unwrap();
            crates.insert(crate_info.root.clone(), crate_obj);
        }

        // Update file mappings
        {
            let mut file_to_project = self.file_to_project.write().unwrap();
            for file_path in files.keys() {
                if let Ok(url) = Url::from_file_path(file_path) {
                    file_to_project.insert(url, crate_info.root.clone());
                }
            }
        }

        Ok(())
    }

    /// Load a standalone file (no project)
    pub fn load_standalone(
        &self,
        file_path: PathBuf,
        db: &mut AnalysisDatabase,
    ) -> Result<(), String> {
        info!("Loading standalone file: {}", file_path.display());

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let source_file = SourceFile::new(db, file_path.to_string_lossy().to_string(), content);

        // For standalone files, create a minimal crate
        let crate_info = CrateInfo {
            name: file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("standalone")
                .to_string(),
            root: file_path.parent().unwrap_or(&file_path).to_path_buf(),
            manifest_path: file_path.clone(), // Use the file itself as "manifest"
        };

        let mut files = HashMap::new();
        files.insert(file_path.clone(), source_file);

        let crate_obj = Crate {
            info: crate_info.clone(),
            main_file: Some(file_path.clone()),
            files,
        };

        self.apply_crate_to_db(&crate_obj, db)?;

        // Update internal state
        {
            let mut crates = self.crates.write().unwrap();
            crates.insert(crate_info.root.clone(), crate_obj);
        }

        Ok(())
    }

    /// Get the crate for a given file URL
    pub fn get_crate_for_file(&self, file_url: &Url) -> Option<Crate> {
        let file_to_project = self.file_to_project.read().unwrap();
        let project_root = file_to_project.get(file_url)?;

        let crates = self.crates.read().unwrap();
        crates.get(project_root).cloned()
    }

    /// Get all loaded crates
    pub fn all_crates(&self) -> Vec<Crate> {
        let crates = self.crates.read().unwrap();
        crates.values().cloned().collect()
    }

    /// Get the ProjectCrate for a given file URL
    pub fn get_project_crate_for_file(&self, file_url: &Url) -> Option<ProjectCrate> {
        let file_to_project = self.file_to_project.read().unwrap();
        let project_root = file_to_project.get(file_url)?;

        let project_crate_ids = self.project_crate_ids.read().unwrap();
        project_crate_ids.get(project_root).cloned()
    }

    /// Get the ProjectCrate for a given project root
    pub fn get_project_crate_for_root(&self, root: &PathBuf) -> Option<ProjectCrate> {
        let project_crate_ids = self.project_crate_ids.read().unwrap();
        project_crate_ids.get(root).cloned()
    }

    /// Clear all loaded projects
    pub fn clear(&self) {
        let mut crates = self.crates.write().unwrap();
        let mut file_to_project = self.file_to_project.write().unwrap();
        let mut project_crate_ids = self.project_crate_ids.write().unwrap();

        crates.clear();
        file_to_project.clear();
        project_crate_ids.clear();
    }

    fn discover_crate_files(
        &self,
        crate_info: &CrateInfo,
    ) -> Result<HashMap<PathBuf, SourceFile>, String> {
        debug!("Discovering files for crate: {}", crate_info.name);

        // For now, use the existing discovery logic
        // This will be improved to use compiler-driven discovery
        let config = ProjectDiscoveryConfig::default();
        let discovered = discover_project_files(&crate_info.root, &config)
            .map_err(|e| format!("Failed to discover project files: {}", e))?;

        // Create a temporary database to create SourceFile instances
        let temp_db = AnalysisDatabase::new();

        let mut files = HashMap::new();
        for path in discovered.files {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let source_file =
                    SourceFile::new(&temp_db, path.to_string_lossy().to_string(), content);
                files.insert(path, source_file);
            }
        }

        Ok(files)
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

    fn apply_crate_to_db(
        &self,
        crate_obj: &Crate,
        db: &mut AnalysisDatabase,
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
            let mut project_crate_ids = self.project_crate_ids.write().unwrap();
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
