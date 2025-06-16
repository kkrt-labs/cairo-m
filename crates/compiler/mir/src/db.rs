//! Database traits and implementations for MIR generation with Salsa integration.

use cairo_m_compiler_parser::{SourceProgram, Upcast};
use cairo_m_compiler_semantic::SemanticDb;

use crate::{MirError, MirModule};

/// Database trait for MIR (Mid-level Intermediate Representation) queries.
///
/// This trait extends SemanticDb to provide MIR-specific queries that are cached
/// and incrementally recomputed by Salsa. All MIR generation should go through
/// these queries to benefit from incremental compilation.
#[salsa::db]
pub trait MirDb: SemanticDb + Upcast<dyn SemanticDb> {}

/// Generate MIR for a source file.
///
/// This is the main entry point for MIR generation. It uses the semantic index
/// to build the MIR module, with full incremental caching support.
#[salsa::tracked]
pub fn generate_mir(db: &dyn MirDb, file: SourceProgram) -> Option<MirModule> {
    // Get the parsed module
    let parsed = cairo_m_compiler_parser::parse_program(db.upcast(), file);

    if !parsed.diagnostics.is_empty() {
        // Don't generate MIR if there are parse errors
        return None;
    }

    // Delegate to the existing generate_mir function from ir_generation
    crate::ir_generation::generate_mir(db, file).map(|arc| (*arc).clone())
}

/// Track MIR-specific errors separately for better diagnostics.
///
/// This allows us to report MIR generation errors without blocking
/// other compilation phases.
#[salsa::tracked]
pub fn mir_errors(_db: &dyn MirDb, _file: SourceProgram) -> Vec<MirError> {
    // TODO
    // For now, we'll return an empty vector
    // In the future, this should collect errors from MIR generation
    vec![]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Test database that implements all required traits for MIR generation
    #[salsa::db]
    #[derive(Clone, Default)]
    pub struct TestDatabase {
        storage: salsa::Storage<Self>,
    }

    #[salsa::db]
    impl salsa::Database for TestDatabase {}

    #[salsa::db]
    impl cairo_m_compiler_parser::Db for TestDatabase {}

    #[salsa::db]
    impl SemanticDb for TestDatabase {}

    #[salsa::db]
    impl MirDb for TestDatabase {}

    impl Upcast<dyn cairo_m_compiler_parser::Db> for TestDatabase {
        fn upcast(&self) -> &(dyn cairo_m_compiler_parser::Db + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_parser::Db + 'static) {
            self
        }
    }

    impl Upcast<dyn SemanticDb> for TestDatabase {
        fn upcast(&self) -> &(dyn SemanticDb + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn SemanticDb + 'static) {
            self
        }
    }

    pub fn test_db() -> TestDatabase {
        TestDatabase::default()
    }

    #[test]
    fn test_mir_db_trait() {
        let db = TestDatabase::default();
        let source = SourceProgram::new(&db, "fn main() {}".to_string(), "test.cm".to_string());

        // This should trigger MIR generation through Salsa
        let _mir = generate_mir(&db, source);
    }
}
