use std::collections::HashMap;
use std::fs;

use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::memory::Memory;
use cairo_m_prover::adapter::{import_from_runner_output, Instructions, ProverInput};
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_prover::verifier::verify_cairo_m;
use cairo_m_runner::run_cairo_program;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

#[test]
fn test_prove_and_verify_unchanged_memory() {
    let initial_memory_data = [
        (M31(0), QM31::from_u32_unchecked(1, 2, 3, 4), M31(0)),
        (M31(1), QM31::from_u32_unchecked(5, 6, 7, 8), M31(0)),
        (M31(2), QM31::from_u32_unchecked(9, 10, 11, 12), M31(0)),
        (M31(3), QM31::from_u32_unchecked(13, 14, 15, 16), M31(0)),
    ];

    // Create HashMap using first element (address) as key
    let initial_memory: HashMap<M31, (QM31, M31)> = initial_memory_data
        .iter()
        .map(|(address, value, clock)| (*address, (*value, *clock)))
        .collect();

    let memory_boundaries = Memory {
        initial_memory: initial_memory.clone(),
        final_memory: initial_memory,
    };

    let mut prover_input = ProverInput {
        memory_boundaries,
        instructions: Instructions::default(),
    };

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input).unwrap();

    let result = verify_cairo_m::<Blake2sMerkleChannel>(proof);
    assert!(result.is_ok());
}

#[test]
fn test_prove_and_verify_empty_memory() {
    let initial_memory: HashMap<M31, (QM31, M31)> = HashMap::new();

    let memory_boundaries = Memory {
        initial_memory: initial_memory.clone(),
        final_memory: initial_memory,
    };

    let mut prover_input = ProverInput {
        memory_boundaries,
        instructions: Instructions::default(),
    };

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input).unwrap();

    let result = verify_cairo_m::<Blake2sMerkleChannel>(proof);
    result.unwrap();
}

// Run this test with `cargo test -p cairo-m-prover test_prove_and_verify_fibonacci_program --release --features relation-tracker -- --nocapture`
// to see the relation tracker output.
#[test]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidLogupSum")]
fn test_prove_and_verify_fibonacci_program() {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        "fibonacci.cm"
    );
    let compiled_fib = compile_cairo(
        fs::read_to_string(&source_path).unwrap(),
        source_path,
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output = run_cairo_program(
        &compiled_fib.program,
        "fib",
        &[M31::from(1000)],
        Default::default(),
    )
    .unwrap();

    let mut prover_input = import_from_runner_output(&runner_output).unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof).unwrap();
}

#[test]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidLogupSum")]
fn test_prove_and_verify_recursive_fibonacci_program() {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        "recursive_fibonacci.cm"
    );
    let compiled_fib = compile_cairo(
        fs::read_to_string(&source_path).unwrap(),
        source_path,
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output = run_cairo_program(
        &compiled_fib.program,
        "fib",
        &[M31::from(5)],
        Default::default(),
    )
    .unwrap();

    let mut prover_input = import_from_runner_output(&runner_output).unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof).unwrap();
}
