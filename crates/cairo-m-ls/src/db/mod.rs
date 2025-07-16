//! Language Server database definitions and implementations.
//!
//! This module defines the concrete database used by the language server,
//! implementing all necessary traits from the compiler phases.

mod swapper;

use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_parser::{Db as ParserDb, SourceFile, Upcast};
use cairo_m_compiler_semantic::SemanticDb;
pub use swapper::AnalysisDatabaseSwapper;

/// The unified project crate representation used by the language server.
///
/// This serves as the single source of truth for project structure,
/// replacing the various ad-hoc crate creations throughout the codebase.
#[salsa::input(debug)]
pub struct ProjectCrate {
    /// The root directory of the project
    #[return_ref]
    pub root_dir: PathBuf,

    /// The main module name (e.g., "main" for main.cm)
    #[return_ref]
    pub main_module: String,

    /// All source files in the project, keyed by their absolute path
    #[return_ref]
    pub files: HashMap<PathBuf, SourceFile>,
}

/// The language server's analysis database.
///
/// This database extends the compiler database with additional
/// functionality needed for language server operations.
#[salsa::db]
#[derive(Clone, Default)]
pub struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

impl AnalysisDatabase {
    /// Create a new analysis database.
    pub fn new() -> Self {
        Self::default()
    }
}

// Implement all required database traits
impl salsa::Database for AnalysisDatabase {}

#[salsa::db]
impl ParserDb for AnalysisDatabase {}

#[salsa::db]
impl SemanticDb for AnalysisDatabase {}

// Implement upcast traits for each database level
impl Upcast<dyn ParserDb> for AnalysisDatabase {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

impl Upcast<dyn SemanticDb> for AnalysisDatabase {
    fn upcast(&self) -> &(dyn SemanticDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn SemanticDb + 'static) {
        self
    }
}

/// Trait for converting ProjectCrate to the various crate representations
/// used by different compiler phases.
pub trait ProjectCrateExt {
    /// Convert to a semantic::Crate for semantic analysis.
    fn to_semantic_crate(&self, db: &dyn SemanticDb) -> cairo_m_compiler_semantic::Crate;
}

// Module path resolution is now centralized in cairo-m-project crate.
// This implementation delegates to the compiler's canonical module naming logic.
impl ProjectCrateExt for ProjectCrate {
    fn to_semantic_crate(&self, db: &dyn SemanticDb) -> cairo_m_compiler_semantic::Crate {
        let files = self.files(db);
        let main_module = self.main_module(db);
        let root_dir = self.root_dir(db);

        let mut modules = HashMap::new();

        // Create a temporary Project instance to use its module_name_from_path method
        let project = cairo_m_project::Project {
            manifest_path: root_dir.join("cairom.toml"),
            root_directory: root_dir.clone(),
            name: root_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            source_layout: cairo_m_project::SourceLayout::default(),
            entry_point: None,
        };

        // Convert PathBuf keys to module names with proper nesting
        for (path, source_file) in files {
            // Use the Project's module_name_from_path method
            match project.module_name_from_path(&path) {
                Ok(module_name) => {
                    modules.insert(module_name, source_file);
                }
                Err(_) => {
                    // Fallback for files outside project root (shouldn't happen)
                    if let Some(module_name) = path.file_stem().and_then(|s| s.to_str()) {
                        modules.insert(module_name.to_string(), source_file);
                    }
                }
            }
        }

        cairo_m_compiler_semantic::Crate::new(
            db,
            modules,
            main_module,
            root_dir.clone(),
            root_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
        )
    }
}
