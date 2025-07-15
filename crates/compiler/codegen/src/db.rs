//! Database traits and implementations for code generation with Salsa integration.

use std::sync::Arc;

use cairo_m_common::Program;
use cairo_m_compiler_mir::{MirDb, MirModule};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::Crate;

use crate::CodegenError;

/// Database trait for code generation queries.
///
/// This trait extends MirDb to provide code generation queries that are cached
/// and incrementally recomputed by Salsa. All code generation should go through
/// these queries to benefit from incremental compilation.
#[salsa::db]
pub trait CodegenDb: MirDb + Upcast<dyn MirDb> {}

/// Compile a crate to a compiled program.
///
/// This is the main entry point for code generation. It takes a crate
/// and produces the compiled program with full incremental caching support.
#[salsa::tracked]
pub fn compile_project(db: &dyn CodegenDb, crate_id: Crate) -> Result<Arc<Program>, CodegenError> {
    // Get the MIR module
    let mir_module = cairo_m_compiler_mir::db::generate_mir(db.upcast(), crate_id)
        .ok_or_else(|| CodegenError::InvalidMir("No MIR module generated".to_string()))?;

    // Use the existing compile_module logic
    let compiled = crate::compile_module(&mir_module)?;

    Ok(Arc::new(compiled))
}

/// Get the MIR module for a crate (convenience re-export).
///
/// This allows code generation to access MIR without directly depending
/// on the MIR crate's internals.
#[salsa::tracked]
pub fn codegen_mir_module(db: &dyn CodegenDb, crate_id: Crate) -> Option<Arc<MirModule>> {
    cairo_m_compiler_mir::db::generate_mir(db.upcast(), crate_id).map(Arc::new)
}

/// Track code generation errors separately for better diagnostics.
///
/// This allows us to report codegen errors without blocking other phases.
#[salsa::tracked]
pub fn codegen_errors(db: &dyn CodegenDb, crate_id: Crate) -> Vec<CodegenError> {
    // Collect errors from code generation
    match compile_project(db, crate_id) {
        Ok(_) => vec![],
        Err(e) => vec![e],
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use cairo_m_compiler_mir::MirDb;
    use cairo_m_compiler_semantic::{File, SemanticDb};

    use super::*;

    /// Test database that implements all required traits for code generation
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

    #[salsa::db]
    impl CodegenDb for TestDatabase {}

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

    impl Upcast<dyn MirDb> for TestDatabase {
        fn upcast(&self) -> &(dyn MirDb + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn MirDb + 'static) {
            self
        }
    }

    #[test]
    fn test_codegen_db_trait() {
        let db = TestDatabase::default();
        let file = File::new(&db, "fn main() {}".to_string(), "test.cm".to_string());
        let mut modules = HashMap::new();
        modules.insert("main".to_string(), file);
        let crate_id = Crate::new(
            &db,
            modules,
            "main".to_string(),
            PathBuf::from("."),
            "test".to_string(),
        );

        // This should trigger code generation through Salsa
        let _result = compile_project(&db, crate_id);
    }
}
