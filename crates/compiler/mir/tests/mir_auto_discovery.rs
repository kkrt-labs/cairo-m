//! Auto-discovery test runner for MIR generation.
//! This file automatically generates tests for all Cairo-M files in the test_data directory.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cairo_m_compiler_mir::{generate_mir, MirModule, PrettyPrint};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::File;

// Test database that implements all required traits for MIR generation
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
impl cairo_m_compiler_semantic::SemanticDb for TestDatabase {}

#[salsa::db]
impl cairo_m_compiler_mir::MirDb for TestDatabase {}

impl Upcast<dyn cairo_m_compiler_parser::Db> for TestDatabase {
    fn upcast(&self) -> &(dyn cairo_m_compiler_parser::Db + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_parser::Db + 'static) {
        self
    }
}

impl Upcast<dyn cairo_m_compiler_semantic::SemanticDb> for TestDatabase {
    fn upcast(&self) -> &(dyn cairo_m_compiler_semantic::SemanticDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_semantic::SemanticDb + 'static) {
        self
    }
}

pub fn test_db() -> TestDatabase {
    TestDatabase::default()
}

/// The result of running MIR generation on a test source.
pub struct MirOutput {
    pub module: Option<Arc<MirModule>>,
    pub mir_string: String,
    pub had_errors: bool,
}

/// Runs the full lowering pipeline on a source string.
pub fn check_mir(source: &str, path: &str) -> MirOutput {
    let db = test_db();
    let file = File::new(&db, source.to_string(), path.to_string());
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    match generate_mir(&db, crate_id) {
        Ok(module) => {
            let mir_string = module.pretty_print(0);
            MirOutput {
                module: Some(module),
                mir_string,
                had_errors: false,
            }
        }
        Err(diagnostics) => MirOutput {
            module: None,
            mir_string: format!(
                "MIR generation failed with diagnostics:\n{:#?}",
                diagnostics
            ),
            had_errors: true,
        },
    }
}

#[test]
fn test_all_fixtures_mir() {
    use cairo_m_test_utils::test_data_path;
    use insta::{assert_snapshot, glob, with_settings};

    // Use insta's glob! macro to discover and test all .cm files
    // This generates a separate test case for each file, allowing all snapshots
    // to be generated in a single test run
    let test_data = test_data_path();

    glob!(test_data.to_str().unwrap(), "**/*.cm", |path| {
        let source = std::fs::read_to_string(path).unwrap();

        // Extract the relative path from test_data for a cleaner name
        let relative_path = path
            .strip_prefix(&test_data)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Generate MIR from the source code
        let mir_output = check_mir(&source, &relative_path);

        // Create the snapshot content
        let snapshot_content = if mir_output.had_errors {
            format!(
                "Fixture: {}\n============================================================\nSource code:\n{}\n============================================================\nResult: ERRORS\n{}",
                relative_path,
                source,
                mir_output.mir_string
            )
        } else {
            format!(
                "Fixture: {}\n============================================================\nSource code:\n{}\n============================================================\nGenerated MIR:\n{}",
                relative_path,
                source,
                mir_output.mir_string
            )
        };

        // Use with_settings to ensure consistent snapshot behavior
        with_settings!({
            description => format!("MIR snapshot for {}", relative_path).as_str(),
            omit_expression => true
        }, {
            assert_snapshot!(snapshot_content);
        });
    });
}
