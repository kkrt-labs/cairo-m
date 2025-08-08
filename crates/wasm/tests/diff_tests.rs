/// These tests compare the output of the compiled cairo-m with result from the womir interpreter
use cairo_m_compiler_codegen::compile_module;
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
fn u32_to_m31(values: Vec<u32>) -> Vec<M31> {
    let mut result = Vec::new();
    for value in values {
        let high = value >> 16;
        let low = value & 0xFFFF;
        result.push(M31::from(high));
        result.push(M31::from(low));
    }
    result
}

/// Convert a vector of M31 to a vector of u32, combining each 2 u16 limbs into a u32
fn m31_to_u32(values: Vec<M31>) -> Vec<u32> {
    let mut result = Vec::new();
    for i in (0..values.len()).step_by(2) {
        let high = values[i].0;
        let low = values[i + 1].0;
        result.push((high << 16) | low);
    }
    result
}

fn test_program(path: &str, func_name: &str, inputs: Vec<u32>) {
    let wasm_file = std::fs::read(path).unwrap();

    let womir_program = load_wasm(GenericIrSetting, &wasm_file).unwrap();

    let dag_module = BlocklessDagModule::from_file(path).unwrap();
    let mir_module = DagToMir::new(dag_module).to_mir().unwrap();
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
}

#[test]
fn test_arithmetic() {
    test_program("tests/test_cases/arithmetic.wasm", "f", vec![1, 2]);
}
