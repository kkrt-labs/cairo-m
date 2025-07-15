//! Cairo-M compiler library
#![allow(clippy::option_if_let_else)]

pub mod db;
pub mod project_discovery;
use std::collections::HashMap;
use std::sync::Arc;

use cairo_m_common::Program;
use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticSeverity, build_diagnostic_message};
use cairo_m_compiler_parser::{SourceFile, parse_file};
use cairo_m_compiler_semantic::Crate as SemanticCrate;
use cairo_m_compiler_semantic::db::{crate_from_project, project_validate_semantics};
use db::CompilerDatabase;
use thiserror::Error;

/// Result type for compilation operations
pub type Result<T> = std::result::Result<T, CompilerError>;

/// Errors that can occur during compilation
#[derive(Debug, Clone, Error)]
pub enum CompilerError {
    /// Parse errors occurred
    #[error("Parse errors: {count} errors found", count = .0.len())]
    ParseErrors(Vec<Diagnostic>),
    /// Semantic validation errors occurred
    #[error("Semantic errors: {count} errors found", count = .0.len())]
    SemanticErrors(Vec<Diagnostic>),
    /// MIR generation failed
    #[error("Failed to generate MIR")]
    MirGenerationFailed,
    /// Code generation failed
    #[error("Code generation failed: {0}")]
    CodeGenerationFailed(String),
}

/// Options for compilation
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Enable verbose output
    pub verbose: bool,
}

/// Compilation output including the compiled program and any diagnostics
#[derive(Debug)]
pub struct CompilerOutput {
    /// The compiled program
    pub program: Arc<Program>,
    /// Any non-error diagnostics generated during compilation
    pub diagnostics: Vec<Diagnostic>,
}

/// Compiles a Cairo-M source file from a string
///
/// # Arguments
/// * `source_text` - The source code to compile
/// * `source_name` - Name of the source file (for error reporting)
/// * `options` - Compilation options
///
/// # Returns
/// * `Ok(CompilerOutput)` - Successfully compiled program with any warnings
/// * `Err(CompilerError)` - Compilation failed with errors
pub fn compile_cairo(
    source_text: String,
    source_name: String,
    options: CompilerOptions,
) -> Result<CompilerOutput> {
    let db = CompilerDatabase::new();
    let source = SourceFile::new(&db, source_text, source_name);

    compile_from_file(&db, source, options)
}

/// Compiles a Cairo-M program from a SourceFile
///
/// This is a lower-level API that allows reusing a database instance
/// for incremental compilation scenarios.
pub fn compile_from_file(
    db: &CompilerDatabase,
    source: SourceFile,
    _options: CompilerOptions,
) -> Result<CompilerOutput> {
    // Parse the program
    let parsed_program = parse_file(db, source);

    if !parsed_program.diagnostics.is_empty() {
        return Err(CompilerError::ParseErrors(parsed_program.diagnostics));
    }

    // Create a single-file crate for semantic validation
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), source);
    let crate_id = SemanticCrate::new(
        db,
        modules,
        "main".to_string(),
        std::path::PathBuf::from("."),
        "single_file".to_string(),
    );

    // Validate semantics using crate-based API
    let semantic_diagnostics = project_validate_semantics(db, crate_id);

    let (semantic_errors, diagnostics): (Vec<_>, Vec<_>) = semantic_diagnostics
        .into_iter()
        .partition(|d| d.severity == DiagnosticSeverity::Error);

    if !semantic_errors.is_empty() {
        return Err(CompilerError::SemanticErrors(semantic_errors));
    }

    let program = cairo_m_compiler_codegen::db::compile_project(db, crate_id)
        .map_err(|e| CompilerError::CodeGenerationFailed(e.to_string()))?;

    Ok(CompilerOutput {
        program,
        diagnostics,
    })
}

/// Compiles a Cairo-M project
///
/// This compiles all files in the project and handles multi-file dependencies.
pub fn compile_project(
    db: &CompilerDatabase,
    project: cairo_m_project::Project,
    _options: CompilerOptions,
) -> Result<CompilerOutput> {
    // Create a semantic crate from the project
    let crate_id = match crate_from_project(db, project) {
        Ok(crate_id) => crate_id,
        Err(diagnostics) => {
            let errors = diagnostics.errors().into_iter().cloned().collect();
            return Err(CompilerError::ParseErrors(errors));
        }
    };

    // Validate semantics using crate-based API
    let semantic_diagnostics = project_validate_semantics(db, crate_id);

    let (semantic_errors, diagnostics): (Vec<_>, Vec<_>) = semantic_diagnostics
        .into_iter()
        .partition(|d| d.severity == DiagnosticSeverity::Error);

    if !semantic_errors.is_empty() {
        return Err(CompilerError::SemanticErrors(semantic_errors));
    }

    let program = cairo_m_compiler_codegen::db::compile_project(db, crate_id)
        .map_err(|e| CompilerError::CodeGenerationFailed(e.to_string()))?;

    Ok(CompilerOutput {
        program,
        diagnostics,
    })
}

/// Formats diagnostics for display (single file)
///
/// # Arguments
/// * `source_text` - The source code text
/// * `diagnostics` - The diagnostics to format
/// * `use_color` - Whether to use color in the output
///
/// # Returns
/// A formatted string containing all diagnostics
pub fn format_diagnostics(
    source_text: &str,
    diagnostics: &[Diagnostic],
    use_color: bool,
) -> String {
    diagnostics
        .iter()
        .map(|d| build_diagnostic_message(source_text, d, use_color))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Formats diagnostics for display (multi-file)
///
/// # Arguments
/// * `source_map` - Map from file path to source code text
/// * `diagnostics` - The diagnostics to format
/// * `use_color` - Whether to use color in the output
///
/// # Returns
/// A formatted string containing all diagnostics
pub fn format_diagnostics_multi_file(
    source_map: &HashMap<String, String>,
    diagnostics: &[Diagnostic],
    use_color: bool,
) -> String {
    diagnostics
        .iter()
        .map(|d| {
            // Get the source text for this diagnostic's file
            let source_text = source_map
                .get(&d.file_path)
                .map(|s| s.as_str())
                .unwrap_or(""); // Use empty string if file not found
            build_diagnostic_message(source_text, d, use_color)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Creates a new compiler database
///
/// This can be used for advanced scenarios where you want to
/// manage the database lifecycle yourself.
pub fn create_compiler_database() -> CompilerDatabase {
    CompilerDatabase::new()
}
