use cairo_m_common::abi_codec::{CairoMValue, InputValue};
/// These tests compare the output of the compiled cairo-m with result from the womir interpreter
use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::PassManager;
use cairo_m_runner::run_cairo_program;
use cairo_m_wasm::flattening::DagToMir;
use cairo_m_wasm::loader::BlocklessDagModule;

use womir::generic_ir::GenericIrSetting;
use womir::interpreter::ExternalFunctions;
use womir::interpreter::Interpreter;
use womir::loader::load_wasm;

use proptest::prelude::*;
use std::path::PathBuf;
use std::process::Command;

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

/// Convert a vector of CairoMValue to a vector of u32, assuming each CairoMValue is a u32
fn collect_u32s(values: Vec<CairoMValue>) -> Vec<u32> {
    values
        .iter()
        .map(|v| match v {
            CairoMValue::U32(n) => *n,
            _ => panic!("Expected u32, got {:?}", v),
        })
        .collect::<Vec<_>>()
}

fn test_program(path: &str, func_name: &str, inputs: Vec<u32>) {
    let wasm_file = std::fs::read(path).unwrap();

    let womir_program = load_wasm(GenericIrSetting, &wasm_file).unwrap();

    let dag_module = BlocklessDagModule::from_file(path).unwrap();
    let mir_module = DagToMir::new(dag_module)
        .to_mir(PassManager::standard_pipeline())
        .unwrap();

    let compiled_module = compile_module(&mir_module).unwrap();

    let data_input = DataInput::new(vec![]);
    let mut womir_interpreter = Interpreter::new(womir_program, data_input);

    let cairo_vm_inputs = inputs
        .iter()
        .map(|&v| InputValue::Number(v as i64))
        .collect::<Vec<_>>();

    // Test with the provided inputs
    let result_womir_interpreter = womir_interpreter.run(func_name, &inputs);
    let result_cairo_m_interpreter = run_cairo_program(
        &compiled_module,
        func_name,
        &cairo_vm_inputs,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        result_womir_interpreter,
        collect_u32s(result_cairo_m_interpreter.return_values)
    );
}

fn build_wasm(path: &PathBuf) {
    assert!(path.exists(), "Target directory does not exist: {path:?}",);

    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .current_dir(path)
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(), "cargo build failed for {path:?}",);
}

proptest! {

    #[test]
    fn run_arithmetic(a: u32, b: u32) {
        test_program("tests/test_cases/arithmetic.wasm", "test_add", vec![a, b]);
        test_program("tests/test_cases/arithmetic.wasm", "test_sub", vec![a, b]);
        test_program("tests/test_cases/arithmetic.wasm", "test_mul", vec![a, b]);
        test_program("tests/test_cases/arithmetic.wasm", "test_div_u", vec![a, b]);
        test_program("tests/test_cases/arithmetic.wasm", "test_rem_u", vec![a, b]);
    }
    #[test]
    fn run_bitwise(a: u32, b: u32) {
        test_program("tests/test_cases/bitwise.wasm", "and", vec![a, b]);
        test_program("tests/test_cases/bitwise.wasm", "or", vec![a, b]);
        test_program("tests/test_cases/bitwise.wasm", "xor", vec![a, b]);
        test_program("tests/test_cases/bitwise.wasm", "shl", vec![a, b]);
        test_program("tests/test_cases/bitwise.wasm", "shr_u", vec![a, b]);
        test_program("tests/test_cases/bitwise.wasm", "rotl", vec![a, b]);
        test_program("tests/test_cases/bitwise.wasm", "rotr", vec![a, b]);
    }

    #[test]
    fn run_fib(a in 0..10u32) {
        test_program("tests/test_cases/fib.wasm", "fib", vec![a]);
    }


    #[test]
    fn run_simple_if(a: u32) {
        test_program("tests/test_cases/simple_if.wasm", "simple_if", vec![a]);
    }

    #[test]
    fn run_if_statement(a: u32) {
        test_program("tests/test_cases/if_statement.wasm", "main", vec![a]);
    }

    #[test]
    fn run_nested_loop(a in 0..10u32) {
        test_program("tests/test_cases/nested_loop.wasm", "nested_loop", vec![a]);
    }

    #[test]
    fn run_fib_from_rust(a in 0..10u32) {
        let case_dir = format!("{}/sample-programs/fib", env!("CARGO_MANIFEST_DIR"));
        build_wasm(&PathBuf::from(&case_dir));
        test_program(
            &format!("{}/target/wasm32-unknown-unknown/release/fib.wasm", case_dir),
            "fib",
            vec![a],
        );
    }

    #[test]
    fn run_ackermann_from_rust(m in 0..3u32, n in 0..3u32) {
        let case_dir = format!("{}/sample-programs/ackermann", env!("CARGO_MANIFEST_DIR"));
        build_wasm(&PathBuf::from(&case_dir));
        test_program(
            &format!("{}/target/wasm32-unknown-unknown/release/ackermann.wasm", case_dir),
            "ackermann",
            vec![m, n],
        );
    }

    #[test]
    fn run_calls(a: u32) {
        // Void callee
        test_program("tests/test_cases/calls.wasm", "call_noop", vec![]);
        // Single-return callee
        test_program("tests/test_cases/calls.wasm", "call_ret1", vec![a]);
    }

}

#[test]
fn run_simple_loop() {
    test_program("tests/test_cases/simple_loop.wasm", "main", vec![]);
}

#[test]
fn run_func_call() {
    test_program("tests/test_cases/func_call.wasm", "main", vec![]);
}

#[test]
fn run_variables() {
    test_program("tests/test_cases/variables.wasm", "main", vec![]);
}

#[test]
fn run_load_store_sum() {
    test_program(
        "tests/test_cases/load_store_sum.wasm",
        "load_store_sum",
        vec![10],
    );
}

// For some reason proptest runs forever on that one
#[test]
fn run_globals() {
    test_program("tests/test_cases/globals.wasm", "main", vec![42]);
    test_program("tests/test_cases/globals.wasm", "main", vec![0xFFFFFFFF]);
}
