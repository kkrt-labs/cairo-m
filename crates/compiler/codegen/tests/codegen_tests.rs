//! Main test runner for CASM code generation.

use std::fs;
use std::path::Path;

use cairo_m_compiler_codegen::{CodeGenerator, CodegenDb};
use cairo_m_compiler_mir::{MirDb, generate_mir};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::{File, SemanticDb};
use insta::assert_snapshot;

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
}

/// Runs the full compilation pipeline from source to CASM.
pub fn check_codegen(source: &str, path: &str) -> CodegenOutput {
    use std::collections::HashMap;

    use cairo_m_compiler_semantic::db::Project;

    let db = test_db();
    let file = File::new(&db, source.to_string(), path.to_string());

    // Create a single-file project for MIR generation
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    let project = Project::new(&db, modules, "main".to_string());

    // Generate MIR from source
    let mir_module = generate_mir(&db, project).expect("MIR generation failed");

    let mut generator = CodeGenerator::new();
    generator
        .generate_module(&mir_module)
        .expect("CASM generation failed");

    // Generate debug representation for testing
    let casm_code = generator.debug_instructions();

    CodegenOutput { casm_code }
}

/// Loads a test case from a file.
pub fn load_test_source(path_str: &str) -> String {
    let path = Path::new("tests/test_cases").join(path_str);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read test file {path:?}: {e}"))
}

/// A macro to define a codegen test case. It loads a source file,
/// runs the full compilation pipeline, and snapshots the CASM output.
macro_rules! codegen_test {
    ($test_name:ident, $subdir:expr) => {
        #[test]
        fn $test_name() {
            // Construct the path to the test source file.
            let path = concat!($subdir, "/", stringify!($test_name), ".cm");

            // Load the test source.
            let source = load_test_source(path);

            // Generate CASM from the source code.
            let codegen_output = check_codegen(&source, path);

            // Use insta to snapshot the entire compilation output.
            let snapshot_content = format!(
                "---\nsource: {}\nexpression: codegen_output\n---\nFixture: {}.cm\n============================================================\nSource code:\n{}\n============================================================\nGenerated CASM:\n{}\n",
                file!(),
                stringify!($test_name),
                source,
                codegen_output.casm_code,
            );
            assert_snapshot!(concat!($subdir, "_", stringify!($test_name)), snapshot_content);
        }
    };
}

// ====== Test Groups ======

// --- Simple Functions ---
codegen_test!(function_simple, "simple");
codegen_test!(function_with_params, "simple");
codegen_test!(function_with_return, "simple");

// --- Arithmetic ---
codegen_test!(add_two_numbers, "arithmetic");
codegen_test!(subtract_numbers, "arithmetic");
codegen_test!(equality, "arithmetic");
codegen_test!(not_equals, "arithmetic");
codegen_test!(and, "arithmetic");
codegen_test!(or, "arithmetic");
codegen_test!(left_imm, "arithmetic");
codegen_test!(unary, "arithmetic");

// --- Control Flow ---
codegen_test!(simple_if, "control_flow");
codegen_test!(if_else, "control_flow");
codegen_test!(if_else_with_merge, "control_flow");

codegen_test!(complex_condition, "control_flow");

// --- Functions ---
codegen_test!(simple_call, "functions");
codegen_test!(fib, "functions");
codegen_test!(fib_loop, "functions");
codegen_test!(return_values, "functions");

// --- Optimization ---
codegen_test!(in_place_update, "optimization");
codegen_test!(args_in_order, "optimization");
codegen_test!(single_arg_optimization, "optimization");

codegen_test!(random_instructions, "random_instructions");

// --- expressions ---
codegen_test!(tuple_destructuring, "expressions");
