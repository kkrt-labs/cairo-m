//! Main test runner for WASM module loading.

use insta::assert_snapshot;
use std::path::Path;

// Use the loader from our src module
use cairo_m_wasm::loader::BlocklessDagModule;

/// A macro to define a WASM loading test case
macro_rules! wasm_test {
    ($test_name:ident, $file_name:expr) => {
        #[test]
        fn $test_name() {
            let file_path = format!("tests/test_cases/{}", $file_name);

            // Verify file exists
            assert!(Path::new(&file_path).exists(), "WASM file should exist: {}", file_path);

            // Load the WASM module
            let result = BlocklessDagModule::from_file(&file_path);

            // Create snapshot content
            let snapshot_content = match result {
                Ok(ref module) => {
                    // Use the format function from the loader
                    let module_output = module.to_string();

                    // Get the program to access function count
                    let function_count = module.with_program(|program| program.functions.len());

                    format!(
                        "---\nsource: {}\nexpression: wasm_load_result\n---\nWASM File: {}\n============================================================\nSuccess: true\nFunctions loaded: {}\n============================================================\nModule Output:\n{}",
                        file!(),
                        $file_name,
                        function_count,
                        if module_output.is_empty() { "No functions found" } else { &module_output }
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
wasm_test!(load_arithmetic_wasm, "arithmetic.wasm");
wasm_test!(load_if_statement_wasm, "if_statement.wasm");
wasm_test!(load_func_call_wasm, "func_call.wasm");
wasm_test!(load_var_manipulation_wasm, "var_manipulation.wasm");
wasm_test!(load_simple_loop_wasm, "simple_loop.wasm");

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_file_existence() {
        // Test that our test files exist
        assert!(Path::new("tests/test_cases/add.wasm").exists());
        assert!(Path::new("tests/test_cases/fib.wasm").exists());
    }

    #[test]
    fn test_loader_basic() {
        // Test basic loading functionality
        let result = BlocklessDagModule::from_file("tests/test_cases/add.wasm");
        assert!(result.is_ok(), "Should load add.wasm successfully");

        let module = result.unwrap();
        let program = module.with_program(|program| program);
        assert!(
            !program.functions.is_empty(),
            "Should have at least one function"
        );
    }
}
