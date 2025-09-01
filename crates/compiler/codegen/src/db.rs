//! Database traits and implementations for code generation with Salsa integration.

use std::sync::Arc;

use cairo_m_common::Program;
use cairo_m_compiler_mir::{pipeline::PipelineConfig, MirDb};
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
    compile_project_with_config(db, crate_id, PipelineConfig::default())
}

/// Compile a crate to a compiled program using a custom MIR pipeline configuration.
pub fn compile_project_with_config(
    db: &dyn CodegenDb,
    crate_id: Crate,
    pipeline: PipelineConfig,
) -> Result<Arc<Program>, CodegenError> {
    // Get the MIR module using provided pipeline config
    let mir_module =
        cairo_m_compiler_mir::generate_mir_with_config(db.upcast(), crate_id, pipeline).map_err(
            |err| {
                CodegenError::InvalidMir(
                    err.iter()
                        .map(|diag| diag.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            },
        )?;

    // Use the existing compile_module logic
    let compiled = crate::compile_module(&mir_module)?;

    Ok(Arc::new(compiled))
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
