//! Language Server database definitions and implementations.
//!
//! This module defines the concrete database used by the language server,
//! implementing all necessary traits from the compiler phases.

mod swapper;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

// TODO this is a complete mess. Project discovery should be properly handled by the compiler. It should
// absolutely not be done here.
//
// This must critically be addressed before merging this PR.
impl ProjectCrateExt for ProjectCrate {
    fn to_semantic_crate(&self, db: &dyn SemanticDb) -> cairo_m_compiler_semantic::Crate {
        let files = self.files(db);
        let main_module = self.main_module(db);
        let root_dir = self.root_dir(db);

        let mut modules = HashMap::new();

        // Convert PathBuf keys to module names with proper nesting
        for (path, source_file) in files {
            // Calculate relative path from project source directory
            if let Ok(module_name) = get_module_path_repr_from_source_file_path(&path, &root_dir) {
                // Convert path to module name (e.g., "x/y/z.cm" -> "x::y::z")
                modules.insert(module_name, source_file);
            } else {
                // Fallback for files outside project root (shouldn't happen)
                if let Some(module_name) = path.file_stem().and_then(|s| s.to_str()) {
                    modules.insert(module_name.to_string(), source_file);
                }
            }
        }

        tracing::info!("Creating semantic crate with modules: {:?}", modules);
        cairo_m_compiler_semantic::Crate::new(db, modules, main_module)
    }
}

// Gets the module path representation relative to the root/src directory.
fn get_module_path_repr_from_source_file_path(
    path: &Path,
    root_dir: &Path,
) -> Result<String, String> {
    let src_path = root_dir.join("src");
    let relative_path = path.strip_prefix(&src_path);

    if let Err(e) = relative_path {
        return Err(e.to_string());
    }

    let relative_path = relative_path.unwrap();
    let module_name = if let Some(stem) = relative_path.file_stem() {
        let stem_str = stem.to_string_lossy();

        // Get parent directories as module path
        if let Some(parent) = relative_path.parent() {
            if parent.as_os_str().is_empty() {
                // File is in root directory
                stem_str.to_string()
            } else {
                // Convert path separators to :: and append file stem
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
                format!("{}::{}", parent_modules, stem_str)
            }
        } else {
            stem_str.to_string()
        }
    } else {
        // Fallback to simple file stem if something goes wrong
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    };

    Ok(module_name)
}
