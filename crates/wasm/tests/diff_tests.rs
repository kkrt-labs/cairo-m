/// These tests compare the output of the compiled cairo-m with result from the womir interpreter
use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::PassManager;
use cairo_m_runner::run_cairo_program;
use cairo_m_wasm::flattening::DagToMir;
use cairo_m_wasm::loader::BlocklessDagModule;

use stwo_prover::core::fields::m31::M31;

use womir::generic_ir::GenericIrSetting;
use womir::interpreter::ExternalFunctions;
use womir::interpreter::Interpreter;
use womir::loader::load_wasm;

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

/// Convert a vector of u32 to a vector of M31, splitting each u32 into 2 u16 limbs
/// Following Cairo-M VM convention: [low_16_bits, high_16_bits]
fn u32_to_m31(values: Vec<u32>) -> Vec<M31> {
    let mut result = Vec::new();
    for value in values {
        let low = value & 0xFFFF; // Low 16 bits go first
        let high = value >> 16; // High 16 bits go second
        result.push(M31::from(low));
        result.push(M31::from(high));
    }
    result
}

/// Convert a vector of M31 to a vector of u32, combining each 2 u16 limbs into a u32
/// Following Cairo-M VM convention: [low_16_bits, high_16_bits]
fn m31_to_u32(values: Vec<M31>) -> Vec<u32> {
    let mut result = Vec::new();
    for i in (0..values.len()).step_by(2) {
        let low = values[i].0; // First M31 is low 16 bits
        let high = values[i + 1].0; // Second M31 is high 16 bits
        result.push((high << 16) | low);
    }
    result
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

    // Test with the provided inputs
    let result_womir_interpreter = womir_interpreter.run(func_name, &inputs);
    let result_cairo_m_interpreter = run_cairo_program(
        &compiled_module,
        func_name,
        &u32_to_m31(inputs),
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        result_womir_interpreter,
        m31_to_u32(result_cairo_m_interpreter.return_values)
    );
}

#[test]
fn test_add() {
    test_program("tests/test_cases/add.wasm", "add", vec![1, 2]);
    test_program("tests/test_cases/add.wasm", "add", vec![42, 69]);
    test_program("tests/test_cases/add.wasm", "add", vec![0xFFFF, 0xFFFF]);
    test_program(
        "tests/test_cases/add.wasm",
        "add",
        vec![0xFFFFFFFF, 0xFFFFFFFF],
    );
}

#[test]
fn test_arithmetic() {
    test_program("tests/test_cases/arithmetic.wasm", "f", vec![1, 2]);
    test_program("tests/test_cases/arithmetic.wasm", "f", vec![42, 69]);
    test_program(
        "tests/test_cases/arithmetic.wasm",
        "f",
        vec![0xFFFF, 0xFFFF],
    );
    test_program(
        "tests/test_cases/arithmetic.wasm",
        "f",
        vec![0xFFFFFFFF, 0xFFFFFFFF],
    );
}

#[test]
fn run_fib() {
    for i in 0..10 {
        test_program("tests/test_cases/fib.wasm", "fib", vec![i]);
    }
}

#[test]
fn run_func_call() {
    test_program("tests/test_cases/func_call.wasm", "main", vec![]);
}

#[test]
fn run_simple_if() {
    for i in 0..10 {
        test_program("tests/test_cases/simple_if.wasm", "simple_if", vec![i]);
    }
}

#[test]
fn run_if_statement() {
    for i in 0..10 {
        test_program("tests/test_cases/if_statement.wasm", "main", vec![i]);
    }
}

#[test]
fn run_simple_loop() {
    test_program("tests/test_cases/simple_loop.wasm", "main", vec![]);
}

#[test]
fn run_variables() {
    test_program("tests/test_cases/variables.wasm", "main", vec![]);
}

#[test]
fn run_nested_loop() {
    for i in 0..10 {
        test_program("tests/test_cases/nested_loop.wasm", "nested_loop", vec![i]);
    }
}

#[test]
fn run_bitwise() {
    test_program("tests/test_cases/bitwise.wasm", "and", vec![42, 69]);
    test_program("tests/test_cases/bitwise.wasm", "or", vec![42, 69]);
    test_program("tests/test_cases/bitwise.wasm", "xor", vec![42, 69]);
}

#[test]
#[ignore]
fn run_sha256() {
    test_program("tests/test_cases/sha256.wasm", "main", vec![]);
}
