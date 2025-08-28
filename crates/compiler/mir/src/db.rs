//! Database traits and implementations for MIR generation with Salsa integration.

use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::SemanticDb;

/// Database trait for MIR (Mid-level Intermediate Representation) queries.
///
/// This trait extends SemanticDb to provide MIR-specific queries that are cached
/// and incrementally recomputed by Salsa. All MIR generation should go through
/// these queries to benefit from incremental compilation.
#[salsa::db]
pub trait MirDb: SemanticDb + Upcast<dyn SemanticDb> {}

#[cfg(test)]
pub mod tests {
    use crate::generate_mir;
    use cairo_m_compiler_semantic::db::Crate;
    use std::path::PathBuf;

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

    #[test]
    fn test_mir_db_trait() {
        use std::collections::HashMap;

        use cairo_m_compiler_semantic::File;

        let db = TestDatabase::default();
        let file = File::new(&db, "fn main() {}".to_string(), "test.cm".to_string());
        let mut modules = HashMap::new();
        modules.insert("main".to_string(), file);
        let crate_id = Crate::new(
            &db,
            modules,
            "main".to_string(),
            PathBuf::from("."),
            "crate_test".to_string(),
        );

        // This should trigger MIR generation through Salsa
        let _mir = generate_mir(&db, crate_id);
    }
}
