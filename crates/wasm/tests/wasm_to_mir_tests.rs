//! Main test runner for WASM to MIR conversion.

use insta::assert_snapshot;
use std::path::Path;

// Use the loader from our src module
use cairo_m_compiler_mir::PrettyPrint;
use cairo_m_wasm::flattening::DagToMir;
use cairo_m_wasm::loader::BlocklessDagModule;

/// A macro to define a WASM to MIR conversion test case
macro_rules! wasm_test {
    ($(#[$attr:meta])* $test_name:ident, $file_name:expr) => {
        #[test]
        $(#[$attr])*
        fn $test_name() {
            let file_path = format!("tests/test_cases/{}", $file_name);

            // Verify file exists
            assert!(Path::new(&file_path).exists(), "WASM file should exist: {}", file_path);

            // Load the WASM module
            let module = BlocklessDagModule::from_file(&file_path).unwrap();
            let mir_module = DagToMir::new(module).to_mir().unwrap();

            // Create snapshot content
            let snapshot_content = {
                let module_output = mir_module.pretty_print(0);

                    format!(
                    "---\nsource: {}\nexpression: wasm_load_result\n---\nWASM File: {}\n============================================================\nSuccess: true\nFunctions loaded: {}\n============================================================\nModule Output:\n{}",
                    file!(),
                    $file_name,
                    mir_module.function_count(),
                    if module_output.is_empty() { "No functions found" } else { &module_output }
                )
            };

            // Snapshot the result
            assert_snapshot!(stringify!($test_name), snapshot_content);
        }
    };
}

// ====== Test Cases ======

// --- Basic WASM to MIR Conversion Tests ---
wasm_test!(convert_add_wasm, "add.wasm");
// TODO : loops, u32 boolean operations
wasm_test!(
    #[ignore]
    convert_fib_wasm,
    "fib.wasm"
);
wasm_test!(convert_arithmetic_wasm, "arithmetic.wasm");
// TODO : u32 boolean operations
wasm_test!(
    #[ignore]
    convert_if_statement_wasm,
    "if_statement.wasm"
);
wasm_test!(convert_func_call_wasm, "func_call.wasm");
wasm_test!(convert_var_manipulation_wasm, "var_manipulation.wasm");
