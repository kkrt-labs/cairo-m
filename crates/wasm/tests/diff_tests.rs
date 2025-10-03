//! These tests compare the output of the compiled cairo-m with result from WASMtime runner.

use cairo_m_common::abi_codec::InputValue;
use cairo_m_runner::run_cairo_program;

use proptest::prelude::*;

use wasmtime::{Instance, Store, Val, ValType};
use wat::parse_file;

mod test_utils;
use test_utils::{
    collect_u32s_by_abi, ensure_rust_wasm_built, get_or_build_cairo_program,
    get_or_build_wasmtime_module, WASMTIME_ENGINE,
};

/// Test a program from a .wat file, given a function name and inputs
/// Asserts results from the Cairo-M interpreter and the WASMtime interpreter are the same
fn test_program_from_wat(path: &str, func_name: &str, inputs: Vec<u32>) {
    let wasm = parse_file(path).unwrap();
    test_program_from_wasm_bytes(&wasm, func_name, inputs);
}

/// Test a program from a .wasm file, given a function name and inputs
/// Asserts results from the Cairo-M interpreter and the WASMtime interpreter are the same
fn test_program_from_wasm(path: &str, func_name: &str, inputs: Vec<u32>) {
    let wasm_file = std::fs::read(path).unwrap();
    test_program_from_wasm_bytes(&wasm_file, func_name, inputs);
}

/// Test a program from wasm bytes, given a function name and inputs
/// Asserts results from the Cairo-M interpreter and the WASMtime interpreter are the same
fn test_program_from_wasm_bytes(wasm_bytes: &[u8], func_name: &str, inputs: Vec<u32>) {
    // Build or fetch cached Cairo-M compiled program
    let prog = get_or_build_cairo_program(wasm_bytes);

    let cairo_vm_inputs = inputs
        .iter()
        .map(|&v| InputValue::Number(v as i64))
        .collect::<Vec<_>>();

    let result_cairo_m_interpreter =
        run_cairo_program(&prog, func_name, &cairo_vm_inputs, Default::default()).unwrap();
    let entry = prog
        .get_entrypoint(func_name)
        .expect("Entrypoint not found in compiled program");
    let cairo_u32s = collect_u32s_by_abi(&result_cairo_m_interpreter.return_values, &entry.returns);

    // Run the original WASM with wasmtime (module cached per wasm)
    let module = get_or_build_wasmtime_module(wasm_bytes);
    let mut store = Store::new(&WASMTIME_ENGINE, ());
    let instance = Instance::new(&mut store, &module, &[]).unwrap();

    let func = instance
        .get_func(&mut store, func_name)
        .unwrap_or_else(|| panic!("Function '{}' not found in WASM module", func_name));

    let ty = func.ty(&store);
    let param_tys: Vec<ValType> = ty.params().collect();
    assert_eq!(
        param_tys.len(),
        inputs.len(),
        "Parameter count mismatch: wasm expects {} params, got {}",
        param_tys.len(),
        inputs.len()
    );

    let mut params: Vec<Val> = Vec::with_capacity(inputs.len());
    for (i, pty) in param_tys.iter().enumerate() {
        match pty {
            ValType::I32 => params.push(Val::I32(inputs[i] as i32)),
            // Extend here if tests introduce other types
            other => panic!("Unsupported WASM param type in tests: {:?}", other),
        }
    }

    let result_tys: Vec<ValType> = ty.results().collect();
    let mut results: Vec<Val> = result_tys
        .iter()
        .map(|rty| match rty {
            ValType::I32 => Val::I32(0),
            other => panic!("Unsupported WASM result type in tests: {:?}", other),
        })
        .collect();

    func.call(&mut store, &params, &mut results).unwrap();

    let wasm_u32s: Vec<u32> = results
        .into_iter()
        .map(|v| match v {
            Val::I32(n) => n as u32,
            other => panic!("Unsupported WASM result type in tests: {:?}", other),
        })
        .collect();

    assert_eq!(wasm_u32s, cairo_u32s);
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
#[should_panic]
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
