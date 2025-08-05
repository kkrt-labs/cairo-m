//! Main test runner for WASM module loading.

use insta::assert_snapshot;
use std::path::Path;

// Use the loader from our src module
use cairo_m_wasm::loader::{format_womir_program, load_module};

/// A macro to define a WASM loading test case
macro_rules! wasm_test {
    ($test_name:ident, $file_name:expr) => {
        #[test]
        fn $test_name() {
            let file_path = format!("tests/test_cases/{}", $file_name);

            // Verify file exists
            assert!(Path::new(&file_path).exists(), "WASM file should exist: {}", file_path);

            // Load the WASM module
            let result = load_module(&file_path);

            // Create snapshot content
            let snapshot_content = match result {
                Ok(ref program) => {
                    // Use the format function from the loader
                    let program_output = format_womir_program(&program);

                    format!(
                        "---\nsource: {}\nexpression: wasm_load_result\n---\nWASM File: {}\n============================================================\nSuccess: true\nFunctions loaded: {}\n============================================================\nProgram Output:\n{}",
                        file!(),
                        $file_name,
                        program.functions.len(),
                        if program_output.is_empty() { "No functions found" } else { &program_output }
                    )
                }
                Err(ref e) => {
                    format!(
                        "---\nsource: {}\nexpression: wasm_load_result\n---\nWASM File: {}\n============================================================\nSuccess: false\nError: {}\n============================================================",
                        file!(),
                        $file_name,
                        e
                    )
                }
            };

            // Snapshot the result
            assert_snapshot!(stringify!($test_name), snapshot_content);

            // Basic assertion - should successfully load
            assert!(result.is_ok(), "Should successfully load WASM file: {:?}", result.err());
        }
    };
}

// ====== Test Cases ======

// --- Basic WASM Loading Tests ---
wasm_test!(load_add_wasm, "add.wasm");
wasm_test!(load_fib_wasm, "fib.wasm");
