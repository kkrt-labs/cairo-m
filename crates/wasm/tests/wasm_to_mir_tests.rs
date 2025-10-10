//! Main test runner for WASM to MIR conversion.

use insta::assert_snapshot;

// Use the loader from our src module
use cairo_m_wasm::loader::BlocklessDagModule;
use cairo_m_wasm::lowering::lower_program_to_casm;
use wat::parse_file;

/// A macro to define a WASM to MIR conversion test case
macro_rules! wasm_test {
    ($(#[$attr:meta])* $test_name:ident, $file_name:expr) => {
        #[test]
        $(#[$attr])*
        fn $test_name() {

            let wat_file_path = &format!("tests/test_cases/{}", $file_name);

            // Load the WASM module
            let wasm_bytes = parse_file(wat_file_path).unwrap();
            let module = BlocklessDagModule::from_bytes(&wasm_bytes).unwrap();
            // Lower to MIR without any optimizations
            let program = lower_program_to_casm(&module).unwrap();

            // Create snapshot content
            let snapshot_content = format!(
                "WASM File: {}\n\nProgram:\n{}",
                $file_name,
                program.debug_instructions()
            );

            insta::with_settings!({
                description => format!("WASM to MIR snapshot: {}", $file_name).as_str(),
                omit_expression => true,
                prepend_module_to_snapshot => true,
                }, {
                assert_snapshot!(stringify!($test_name), snapshot_content);
            });
        }
    };
}

// ====== Test Cases ======

// --- Basic WASM to MIR Conversion Tests ---
wasm_test!(convert_fib_wasm, "fib.wat");
wasm_test!(convert_i32_arithmetic_wasm, "i32_arithmetic.wat");
wasm_test!(convert_simple_if_wasm, "simple_if.wat");
wasm_test!(convert_if_statement_wasm, "if_statement.wat");
wasm_test!(convert_func_call_wasm, "func_call.wat");
wasm_test!(convert_locals_wasm, "locals.wat");
wasm_test!(convert_i32_bitwise_wasm, "i32_bitwise.wat");
wasm_test!(convert_simple_loop_wasm, "simple_loop.wat");
wasm_test!(convert_nested_loop_wasm, "nested_loop.wat");
wasm_test!(convert_load_store_wasm, "load_store.wat");
