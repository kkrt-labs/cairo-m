//! # Semantic Analysis Database
//!
//! This module defines the database traits and implementations for semantic analysis
//! using the Salsa incremental computation framework. It extends the parser database
//! to provide semantic-specific functionality.
//!
//! The database system enables incremental recompilation by caching query results
//! and invalidating them only when their dependencies change.

use cairo_m_compiler_parser as parser;
use parser::{Db as ParserDb, Upcast};

// We may need the parser implementation later for cross-database operations
#[allow(unused_imports)]
use parser::ParserDatabaseImpl;

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
