//! Auto-discovery test runner for CASM code generation.
//! This file automatically generates tests for all Cairo-M files in the test_data directory.

use std::path::PathBuf;

use cairo_m_compiler_codegen::{CodeGenerator, CodegenDb};
use cairo_m_compiler_mir::{generate_mir, MirDb};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::project_validate_semantics;
use cairo_m_compiler_semantic::{File, SemanticDb};

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

pub fn test_db() -> TestDatabase {
    TestDatabase::default()
}

/// The result of running code generation on a test source.
pub struct CodegenOutput {
    pub casm_code: String,
    pub had_semantic_errors: bool,
}

/// Runs the full compilation pipeline from source to CASM.
pub fn check_codegen(source: &str, path: &str) -> CodegenOutput {
    use cairo_m_compiler_semantic::db::Crate;
    use std::collections::HashMap;

    let db = test_db();
    let file = File::new(&db, source.to_string(), path.to_string());

    // Create a single-file crate for MIR generation
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // Check for semantic errors but don't panic - we want to see what happens
    let semantic_errors = project_validate_semantics(&db, crate_id);
    let had_semantic_errors = !semantic_errors.is_empty();

    if had_semantic_errors {
        // Return early with error indication
        return CodegenOutput {
            casm_code: format!("Semantic errors found:\n{:#?}", semantic_errors),
            had_semantic_errors: true,
        };
    }

    // Generate MIR from source
    match generate_mir(&db, crate_id) {
        Ok(mir_module) => {
            let mut generator = CodeGenerator::new();
            match generator.generate_module(&mir_module) {
                Ok(_) => {
                    let casm_code = generator.debug_instructions();
                    CodegenOutput {
                        casm_code,
                        had_semantic_errors: false,
                    }
                }
                Err(e) => CodegenOutput {
                    casm_code: format!("CASM generation failed: {:?}", e),
                    had_semantic_errors: false,
                },
            }
        }
        Err(diagnostics) => CodegenOutput {
            casm_code: format!("MIR generation failed:\n{:#?}", diagnostics),
            had_semantic_errors: false,
        },
    }
}

#[test]
fn test_all_fixtures_codegen() {
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

        // Generate CASM from the source code
        let codegen_output = check_codegen(&source, &relative_path);

        // Create the snapshot content
        let snapshot_content = if codegen_output.had_semantic_errors {
            format!(
                "Fixture: {}\n============================================================\nSource code:\n{}\n============================================================\nResult: SEMANTIC ERRORS\n{}\n",
                relative_path,
                source,
                codegen_output.casm_code,
            )
        } else {
            format!(
                "Fixture: {}\n============================================================\nSource code:\n{}\n============================================================\nGenerated CASM:\n{}\n",
                relative_path,
                source,
                codegen_output.casm_code,
            )
        };

        // Use with_settings to ensure consistent snapshot behavior
        with_settings!({
            description => format!("Codegen snapshot for {}", relative_path).as_str(),
            omit_expression => true
        }, {
            assert_snapshot!(snapshot_content);
        });
    });
}
