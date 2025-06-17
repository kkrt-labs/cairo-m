//! Cairo-M compiler library

pub mod db;
use std::sync::Arc;

pub use cairo_m_compiler_codegen::compiled_program::{
    CompiledInstruction, CompiledProgram, ProgramMetadata,
};
use cairo_m_compiler_diagnostics::{build_diagnostic_message, Diagnostic, DiagnosticSeverity};
use cairo_m_compiler_parser::{parse_program, SourceProgram};
use db::CompilerDatabase;

/// Result type for compilation operations
pub type Result<T> = std::result::Result<T, CompilerError>;

/// Errors that can occur during compilation
#[derive(Debug, Clone)]
pub enum CompilerError {
    /// Parse errors occurred
    ParseErrors(Vec<Diagnostic>),
    /// Semantic validation errors occurred
    SemanticErrors(Vec<Diagnostic>),
    /// MIR generation failed
    MirGenerationFailed,
    /// Code generation failed
    CodeGenerationFailed(String),
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseErrors(diagnostics) => {
                write!(f, "Parse errors: {} errors found", diagnostics.len())
            }
            Self::SemanticErrors(diagnostics) => {
                write!(f, "Semantic errors: {} errors found", diagnostics.len())
            }
            Self::MirGenerationFailed => write!(f, "Failed to generate MIR"),
            Self::CodeGenerationFailed(e) => write!(f, "Code generation failed: {}", e),
        }
    }
}

impl std::error::Error for CompilerError {}

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
    pub program: Arc<CompiledProgram>,
    /// Any warnings generated during compilation
    pub warnings: Vec<Diagnostic>,
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
    let source = SourceProgram::new(&db, source_text, source_name);

    compile_from_source(&db, source, options)
}

/// Compiles a Cairo-M program from a SourceProgram
///
/// This is a lower-level API that allows reusing a database instance
/// for incremental compilation scenarios.
pub fn compile_from_source(
    db: &CompilerDatabase,
    source: SourceProgram,
    _options: CompilerOptions,
) -> Result<CompilerOutput> {
    // Parse the program
    let parsed_program = parse_program(db, source);

    if !parsed_program.diagnostics.is_empty() {
        return Err(CompilerError::ParseErrors(parsed_program.diagnostics));
    }

    // Validate semantics
    let semantic_diagnostics = cairo_m_compiler_semantic::db::validate_semantics(db, source);

    let semantic_errors: Vec<_> = semantic_diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .cloned()
        .collect();

    if !semantic_errors.is_empty() {
        return Err(CompilerError::SemanticErrors(semantic_errors));
    }

    // Collect warnings
    let warnings: Vec<_> = semantic_diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Warning)
        .cloned()
        .collect();

    // Generate MIR
    let _mir_module = cairo_m_compiler_mir::db::generate_mir(db, source)
        .ok_or(CompilerError::MirGenerationFailed)?;

    // Generate code
    let program = cairo_m_compiler_codegen::db::compile_module(db, source)
        .map_err(|e| CompilerError::CodeGenerationFailed(e.to_string()))?;

    Ok(CompilerOutput { program, warnings })
}

/// Formats diagnostics for display
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

/// Creates a new compiler database
///
/// This can be used for advanced scenarios where you want to
/// manage the database lifecycle yourself.
pub fn create_compiler_database() -> CompilerDatabase {
    CompilerDatabase::new()
}
