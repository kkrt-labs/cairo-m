//! Main test runner for CASM code generation.

use cairo_m_compiler_codegen::generate_casm;
use cairo_m_compiler_mir::generate_mir;
use cairo_m_compiler_semantic::{File, SemanticDatabaseImpl};
use insta::assert_snapshot;
use std::fs;
use std::path::Path;

/// The result of running code generation on a test source.
pub struct CodegenOutput {
    pub casm_code: String,
}

fn test_db() -> SemanticDatabaseImpl {
    cairo_m_compiler_semantic::SemanticDatabaseImpl::default()
}

/// Runs the full compilation pipeline from source to CASM.
pub fn check_codegen(source: &str, path: &str) -> CodegenOutput {
    let db = test_db();
    let file = File::new(&db, source.to_string(), path.to_string());

    // Generate MIR from source
    let mir_module = generate_mir(&db, file).expect("MIR generation failed");

    // Generate CASM from MIR
    let casm_code = generate_casm(&mir_module).expect("CASM generation failed");

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
                "---\nsource: {}\nexpression: codegen_output\n---\nFixture: {}.cm\n============================================================\nSource code:\n{}\n============================================================\nGenerated CASM:\n{}",
                file!(),
                stringify!($test_name),
                source,
                codegen_output.casm_code
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

// --- Control Flow ---
codegen_test!(simple_if, "control_flow");
codegen_test!(if_else, "control_flow");

// --- Functions ---
codegen_test!(simple_call, "functions");
codegen_test!(fib, "functions");
