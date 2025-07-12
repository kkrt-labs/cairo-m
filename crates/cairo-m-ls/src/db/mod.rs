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

// We'll add these back when we integrate with MIR and Codegen
// For now, let's focus on Parser and Semantic phases

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
#[derive(Clone)]
pub struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

impl AnalysisDatabase {
    /// Create a new analysis database.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for AnalysisDatabase {
    fn default() -> Self {
        Self {
            storage: salsa::Storage::default(),
        }
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
    /// Convert to a parser::Crate for the parsing phase.
    fn to_parser_crate(&self, db: &dyn ParserDb) -> cairo_m_compiler_parser::Crate;

    /// Convert to a semantic::Crate for semantic analysis.
    fn to_semantic_crate(&self, db: &dyn SemanticDb) -> cairo_m_compiler_semantic::Crate;
}

impl ProjectCrateExt for ProjectCrate {
    fn to_parser_crate(&self, db: &dyn ParserDb) -> cairo_m_compiler_parser::Crate {
        // Access fields via getter methods
        let files = self.files(db);
        let root_dir = self.root_dir(db);
        let main_module = self.main_module(db);

        // Find the entry file path
        let entry_file = files
            .keys()
            .find(|path| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s == main_module)
                    .unwrap_or(false)
            })
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        cairo_m_compiler_parser::Crate::new(
            db,
            root_dir.to_string_lossy().to_string(),
            entry_file,
            files.values().cloned().collect(),
        )
    }

    fn to_semantic_crate(&self, db: &dyn SemanticDb) -> cairo_m_compiler_semantic::Crate {
        let files = self.files(db);
        let main_module = self.main_module(db);

        let mut modules = HashMap::new();

        // Convert PathBuf keys to module names
        for (path, source_file) in files {
            if let Some(module_name) = path.file_stem().and_then(|s| s.to_str()) {
                modules.insert(module_name.to_string(), source_file);
            }
        }

        cairo_m_compiler_semantic::Crate::new(db, modules, main_module.clone())
    }
}
