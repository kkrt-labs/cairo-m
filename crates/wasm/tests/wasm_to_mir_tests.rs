//! Main test runner for WASM to MIR conversion.

use insta::assert_snapshot;
use std::path::PathBuf;

// Use the loader from our src module
use cairo_m_compiler_mir::{PassManager, PrettyPrint};
use cairo_m_wasm::loader::BlocklessDagModule;
use cairo_m_wasm::lowering::lower_program_to_mir;

mod test_utils;
use test_utils::build_wasm;

/// A macro to define a WASM to MIR conversion test case
macro_rules! wasm_test {
    ($(#[$attr:meta])* $test_name:ident, $file_name:expr) => {
        #[test]
        $(#[$attr])*
        fn $test_name() {
            let wat_file_path = PathBuf::from(&format!("tests/test_cases/{}", $file_name));
            let wasm_file_path = wat_file_path.with_extension("wasm");

            if !wasm_file_path.exists() {
                build_wasm(&wat_file_path);
            }

            // Load the WASM module
            let wasm_file = std::fs::read(&wasm_file_path).unwrap();
            let module = BlocklessDagModule::from_bytes(&wasm_file).unwrap();
            // Lower to MIR without any optimizations
            let mir_module = lower_program_to_mir(&module, PassManager::no_opt_pipeline()).unwrap();

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
wasm_test!(convert_add_wasm, "add.wat");
wasm_test!(convert_fib_wasm, "fib.wat");
wasm_test!(convert_arithmetic_wasm, "arithmetic.wat");
wasm_test!(convert_simple_if_wasm, "simple_if.wat");
wasm_test!(convert_if_statement_wasm, "if_statement.wat");
wasm_test!(convert_func_call_wasm, "func_call.wat");
wasm_test!(convert_variables_wasm, "variables.wat");
wasm_test!(convert_bitwise_wasm, "bitwise.wat");
wasm_test!(convert_simple_loop_wasm, "simple_loop.wat");
wasm_test!(convert_nested_loop_wasm, "nested_loop.wat");
wasm_test!(convert_load_store_wasm, "load_store.wat");
wasm_test!(
    #[ignore]
    convert_sha256_wasm,
    "sha256.wat"
);
