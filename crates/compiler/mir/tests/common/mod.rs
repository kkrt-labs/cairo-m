//! Common test utilities for MIR tests

use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_mir::MirDb;
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::{File, SemanticDb};

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

/// Get the workspace root path
pub fn workspace_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // compiler/
        .and_then(|p| p.parent()) // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("Failed to find workspace root")
        .to_path_buf()
}

/// Compile a source string to a Crate for testing
pub fn create_test_crate(
    db: &TestDatabase,
    source: &str,
    path: &str,
    crate_name: &str,
) -> cairo_m_compiler_semantic::db::Crate {
    let file = File::new(db, source.to_string(), path.to_string());
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    cairo_m_compiler_semantic::db::Crate::new(
        db,
        modules,
        "main".to_string(),
        workspace_path(),
        crate_name.to_string(),
    )
}
