use cairo_m_common::abi_codec::{CairoMValue, InputValue};
use cairo_m_common::program::AbiType;
/// These tests compare the output of the compiled cairo-m with result from the womir interpreter
use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::PassManager;
use cairo_m_runner::run_cairo_program;
use cairo_m_wasm::loader::BlocklessDagModule;
use cairo_m_wasm::lowering::lower_program_to_mir;

use womir::generic_ir::GenericIrSetting;
use womir::interpreter::ExternalFunctions;
use womir::interpreter::Interpreter;
use womir::loader::load_wasm;

use proptest::prelude::*;

use wat::parse_file;

mod test_utils;
use test_utils::ensure_rust_wasm_built;

struct DataInput {
    values: Vec<u32>,
}

impl DataInput {
    const fn new(values: Vec<u32>) -> Self {
        Self { values }
    }
}

impl ExternalFunctions for DataInput {
    fn call(&mut self, module: &str, function: &str, args: &[u32]) -> Vec<u32> {
        match (module, function) {
            ("env", "read_u32") => {
                vec![self.values[args[0] as usize]]
            }
            ("env", "abort") => {
                panic!("Abort called with args: {:?}", args);
            }
            _ => {
                panic!(
                    "External function not implemented: {module}.{function} with args: {:?}",
                    args
                );
            }
        }
    }
}

/// Convert CairoM return values to u32 following the ABI, mirroring runner tests behavior.
fn collect_u32s_by_abi(
    values: &[CairoMValue],
    abi_returns: &[cairo_m_common::program::AbiSlot],
) -> Vec<u32> {
    assert_eq!(
        values.len(),
        abi_returns.len(),
        "Return value count mismatch: got {} but ABI declares {}",
        values.len(),
        abi_returns.len()
    );
    values
        .iter()
        .zip(abi_returns.iter())
        .map(|(v, slot)| match (&slot.ty, v) {
            (AbiType::U32, CairoMValue::U32(n)) => *n,
            (AbiType::Bool, CairoMValue::Bool(b)) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            // For felt returns, WOMIR currently models i32 as u32; not expected in current WASM tests.
            (AbiType::Felt, CairoMValue::Felt(f)) => f.0,
            _ => panic!(
                "Type/value mismatch in return: ABI {:?}, value {:?}",
                slot.ty, v
            ),
        })
        .collect()
}

fn test_program_from_wat(path: &str, func_name: &str, inputs: Vec<u32>) {
    let wasm = parse_file(path).unwrap();
    test_program_from_wasm_bytes(&wasm, func_name, inputs);
}

fn test_program_from_wasm(path: &str, func_name: &str, inputs: Vec<u32>) {
    let wasm_file = std::fs::read(path).unwrap();
    test_program_from_wasm_bytes(&wasm_file, func_name, inputs);
}

fn test_program_from_wasm_bytes(wasm_bytes: &[u8], func_name: &str, inputs: Vec<u32>) {
    let womir_program = load_wasm(GenericIrSetting, wasm_bytes)
        .unwrap()
        .process_all_functions()
        .unwrap();

    let dag_module = BlocklessDagModule::from_bytes(wasm_bytes).unwrap();
    let mir_module = lower_program_to_mir(&dag_module, PassManager::standard_pipeline()).unwrap();
    let compiled_module = compile_module(&mir_module).unwrap();

    let data_input = DataInput::new(vec![]);
    let mut womir_interpreter = Interpreter::new(womir_program, data_input);

    let cairo_vm_inputs = inputs
        .iter()
        .map(|&v| InputValue::Number(v as i64))
        .collect::<Vec<_>>();

    // Test with the provided inputs in both the WOMIR interpreter and the Cairo-M runner.
    let result_womir_interpreter = womir_interpreter.run(func_name, &inputs);
    let result_cairo_m_interpreter = run_cairo_program(
        &compiled_module,
        func_name,
        &cairo_vm_inputs,
        Default::default(),
    )
    .unwrap();
    let entry = compiled_module
        .get_entrypoint(func_name)
        .expect("Entrypoint not found in compiled program");
    let cairo_u32s = collect_u32s_by_abi(&result_cairo_m_interpreter.return_values, &entry.returns);

    assert_eq!(result_womir_interpreter, cairo_u32s);
}

proptest! {
    #[test]
    fn run_i32_arithmetic(a: u32, b: u32) {
        test_program_from_wat("tests/test_cases/i32_arithmetic.wat", "i32_add", vec![a, b]);
        test_program_from_wat("tests/test_cases/i32_arithmetic.wat", "i32_sub", vec![a, b]);
        test_program_from_wat("tests/test_cases/i32_arithmetic.wat", "i32_mul", vec![a, b]);
        if b != 0 {
            test_program_from_wat("tests/test_cases/i32_arithmetic.wat", "i32_div_u", vec![a, b]);
        }
    }

    #[test]
    fn run_i32_bitwise(a: u32, b: u32) {
        test_program_from_wat("tests/test_cases/i32_bitwise.wat", "i32_and", vec![a, b]);
        test_program_from_wat("tests/test_cases/i32_bitwise.wat", "i32_or", vec![a, b]);
        test_program_from_wat("tests/test_cases/i32_bitwise.wat", "i32_xor", vec![a, b]);
    }

    #[test]
    fn run_fib(a in 0..10u32) {
        test_program_from_wat("tests/test_cases/fib.wat", "fib", vec![a]);
    }


    #[test]
    fn run_simple_if(a: u32) {
        test_program_from_wat("tests/test_cases/simple_if.wat", "simple_if", vec![a]);
    }

    #[test]
    fn run_if_statement(a: u32) {
        test_program_from_wat("tests/test_cases/if_statement.wat", "if_statement", vec![a]);
    }

    #[test]
    fn run_simple_loop(a in 0..10u32) {
        test_program_from_wat("tests/test_cases/simple_loop.wat", "simple_loop", vec![a]);
    }

    #[test]
    fn run_nested_loop(a in 0..10u32) {
        test_program_from_wat("tests/test_cases/nested_loop.wat", "nested_loop", vec![a]);
    }

    #[test]
    fn run_load_store_add(a: u32, b: u32) {
        test_program_from_wat("tests/test_cases/load_store.wat", "add", vec![a, b]);
    }


    #[test]
    fn run_fib_from_rust(a in 0..10u32) {
        let case_dir = format!("{}/sample-programs/fib", env!("CARGO_MANIFEST_DIR"));
        ensure_rust_wasm_built(&case_dir);
        test_program_from_wasm(
            &format!("{}/target/wasm32-unknown-unknown/release/fib.wasm", case_dir),
            "fib",
            vec![a],
        );
    }

    #[test]
    fn run_ackermann_from_rust(m in 0..3u32, n in 0..3u32) {
        let case_dir = format!("{}/sample-programs/ackermann", env!("CARGO_MANIFEST_DIR"));
        ensure_rust_wasm_built(&case_dir);
        test_program_from_wasm(
            &format!("{}/target/wasm32-unknown-unknown/release/ackermann.wasm", case_dir),
            "ackermann",
            vec![m, n],
        );
    }
}

#[test]
#[should_panic(expected = "integer divide by zero in I32DivU")]
fn run_div_by_zero() {
    test_program_from_wat(
        "tests/test_cases/i32_arithmetic.wat",
        "i32_div_u",
        vec![1, 0],
    );
}

#[test]
fn run_func_call() {
    test_program_from_wat("tests/test_cases/func_call.wat", "func_call", vec![]);
}

#[test]
fn run_locals() {
    test_program_from_wat("tests/test_cases/locals.wat", "locals", vec![]);
}

#[test]
fn run_load_store_sum() {
    test_program_from_wat(
        "tests/test_cases/load_store.wat",
        "load_store_sum",
        vec![100],
    );
}

#[test]
fn run_load_store_sum_3_with_offsets() {
    test_program_from_wat(
        "tests/test_cases/load_store.wat",
        "load_store_sum_3_with_offsets",
        vec![],
    );
}
