//! # Semantic Analysis Database
//!
//! This module defines the database traits and implementations for semantic analysis
//! using the Salsa incremental computation framework. It extends the parser database
//! to provide semantic-specific functionality.
//!
//! The database system enables incremental recompilation by caching query results
//! and invalidating them only when their dependencies change.

use cairo_m_compiler_diagnostics::DiagnosticCollection;
use cairo_m_compiler_parser as parser;
// We may need the parser implementation later for cross-database operations
#[allow(unused_imports)]
use parser::ParserDatabaseImpl;
use parser::{Db as ParserDb, Upcast};

/// Database trait for semantic analysis, extending the parser database
///
/// This trait defines the interface for semantic-specific database operations.
/// Type resolution and inference queries are defined as standalone tracked functions.
#[salsa::db]
pub trait SemanticDb: ParserDb + Upcast<dyn ParserDb> {
    // Type queries are defined as standalone tracked functions in type_resolution.rs
    // This trait includes any database-specific configuration or settings
}

/// Concrete database implementation for semantic analysis
///
/// This provides the actual storage and implementation for all database queries.
/// It combines both parser and semantic analysis capabilities in a single database.
///
/// # Thread Safety
///
/// This implementation is `Clone` and can be safely shared between threads.
/// Salsa handles the synchronization internally.
#[salsa::db]
#[derive(Clone, Default)]
pub struct SemanticDatabaseImpl {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for SemanticDatabaseImpl {}
#[salsa::db]
impl ParserDb for SemanticDatabaseImpl {}
#[salsa::db]
impl SemanticDb for SemanticDatabaseImpl {}

impl Upcast<dyn ParserDb> for SemanticDatabaseImpl {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

/// Validate semantics of a source file and return diagnostics.
///
/// This is a tracked query that performs comprehensive semantic validation
/// including scope checking, type checking, and control flow analysis.
#[salsa::tracked]
pub fn validate_semantics(
    db: &dyn SemanticDb,
    file: parser::SourceProgram,
) -> DiagnosticCollection {
    // Parse the program first
    let parsed = parser::parse_program(db.upcast(), file);

    // Get the semantic index (this is already a tracked query)
    let index = crate::semantic_index::semantic_index_from_module(db, &parsed.module, file);

    // Create validator registry with all available validators
    let registry = crate::validation::validator::create_default_registry();
    registry.validate_all(db, file, &index)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[salsa::db]
    #[derive(Clone)]
    pub struct TestDb {
        storage: salsa::Storage<Self>,
    }

    impl salsa::Database for TestDb {}
    #[salsa::db]
    impl ParserDb for TestDb {}
    #[salsa::db]
    impl SemanticDb for TestDb {}

    impl Upcast<dyn ParserDb> for TestDb {
        fn upcast(&self) -> &(dyn ParserDb + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
            self
        }
    }

    pub fn test_db() -> TestDb {
        TestDb {
            storage: salsa::Storage::default(),
        }
    }
}
