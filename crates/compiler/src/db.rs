//! Unified database implementation for the Cairo-M compiler.
//!
//! This module provides a single database that implements all compilation phases
//! (parsing, semantic analysis, MIR generation, and code generation) with full
//! Salsa incremental compilation support.

use cairo_m_compiler_codegen::CodegenDb;
use cairo_m_compiler_mir::MirDb;
use cairo_m_compiler_parser::{Db as ParserDb, Upcast};
use cairo_m_compiler_semantic::SemanticDb;

/// The main compiler database that supports all compilation phases.
///
/// This database implements all the trait requirements for:
/// - Parsing (ParserDb)
/// - Semantic analysis (SemanticDb)
/// - MIR generation (MirDb)
/// - Code generation (CodegenDb)
///
/// It provides a unified interface for the entire compilation pipeline
/// with automatic incremental compilation support through Salsa.
#[salsa::db]
#[derive(Clone, Default)]
pub struct CompilerDatabase {
    storage: salsa::Storage<Self>,
}

// Implement all required database traits
impl salsa::Database for CompilerDatabase {}

#[salsa::db]
impl ParserDb for CompilerDatabase {}

#[salsa::db]
impl SemanticDb for CompilerDatabase {}

#[salsa::db]
impl MirDb for CompilerDatabase {}

#[salsa::db]
impl CodegenDb for CompilerDatabase {}

// Implement upcast traits for each database level
impl Upcast<dyn ParserDb> for CompilerDatabase {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

impl Upcast<dyn SemanticDb> for CompilerDatabase {
    fn upcast(&self) -> &(dyn SemanticDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn SemanticDb + 'static) {
        self
    }
}

impl Upcast<dyn MirDb> for CompilerDatabase {
    fn upcast(&self) -> &(dyn MirDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn MirDb + 'static) {
        self
    }
}

impl Upcast<dyn CodegenDb> for CompilerDatabase {
    fn upcast(&self) -> &(dyn CodegenDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn CodegenDb + 'static) {
        self
    }
}

impl CompilerDatabase {
    /// Create a new compiler database.
    pub fn new() -> Self {
        Self::default()
    }
}
