//! Run these tests with feature `relation-tracker` to see the relation tracker output.
use std::collections::HashMap;
use std::fs;

use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::memory::Memory;
use cairo_m_prover::adapter::merkle::{build_partial_merkle_tree, TREE_HEIGHT};
use cairo_m_prover::adapter::{
    import_from_runner_output, HashInput, Instructions, MerkleTrees, ProverInput,
};
use cairo_m_prover::debug_tools::assert_constraints::assert_constraints;
use cairo_m_prover::poseidon2::Poseidon2Hash;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_prover::verifier::verify_cairo_m;
use cairo_m_runner::{run_cairo_program, RunnerOptions};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn test_prove_and_verify_unchanged_memory() {
    let initial_memory_data = [
        (M31(0), QM31::from_u32_unchecked(1, 2, 3, 4), M31(0), M31(0)),
        (M31(1), QM31::from_u32_unchecked(5, 6, 7, 8), M31(0), M31(0)),
        (
            M31(2),
            QM31::from_u32_unchecked(9, 10, 11, 12),
            M31(0),
            M31(0),
        ),
        (
            M31(3),
            QM31::from_u32_unchecked(13, 14, 15, 16),
            M31(0),
            M31(0),
        ),
    ];

    // Create HashMap using address and depth as key
    let initial_memory: HashMap<(M31, M31), (QM31, M31, M31)> = initial_memory_data
        .iter()
        .map(|(address, value, clock, multiplicity)| {
            (
                (*address, M31::from(TREE_HEIGHT)),
                (*value, *clock, *multiplicity),
            )
        })
        .collect();

    let mut memory = Memory {
        initial_memory: initial_memory.clone(),
        final_memory: initial_memory,
        clock_update_data: vec![],
    };

    let (initial_tree, initial_root) =
        build_partial_merkle_tree::<Poseidon2Hash>(&mut memory.initial_memory);
    let (final_tree, final_root) =
        build_partial_merkle_tree::<Poseidon2Hash>(&mut memory.final_memory);

    let mut poseidon2_inputs =
        Vec::<HashInput>::with_capacity(initial_tree.len() + final_tree.len());
    initial_tree.iter().for_each(|node| {
        poseidon2_inputs.push(node.to_hash_input());
    });
    final_tree.iter().for_each(|node| {
        poseidon2_inputs.push(node.to_hash_input());
    });

    let mut prover_input = ProverInput {
        merkle_trees: MerkleTrees {
            initial_tree,
            final_tree,
            initial_root,
            final_root,
        },
        public_addresses: vec![],
        memory,
        instructions: Instructions::default(),
        poseidon2_inputs,
    };

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    let result = verify_cairo_m::<Blake2sMerkleChannel>(proof, None);
    if let Err(e) = &result {
        eprintln!("Verification failed: {:?}", e);
    }
    assert!(result.is_ok());
}

#[test]
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
        &[M31::from(5)],
        Default::default(),
    )
    .unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_addresses,
    )
    .unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

#[test]
fn test_prove_and_verify_large_fibonacci_program() {
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
        &[M31::from(1_000_000)],
        RunnerOptions {
            max_steps: 2_usize.pow(30),
        },
    )
    .unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_addresses,
    )
    .unwrap();

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

#[test]
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

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_addresses,
    )
    .unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

#[test]
fn test_hash_continuity_fibonacci() {
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

    let runner_options = RunnerOptions { max_steps: 10 };

    let runner_output = run_cairo_program(
        &compiled_fib.program,
        "fib",
        &[M31::from(5)],
        runner_options,
    )
    .unwrap();

    let public_addresses = runner_output.public_addresses.clone();

    let mut previous_final_root: Option<M31> = None;

    for segment in runner_output.vm.segments {
        let mut prover_input =
            import_from_runner_output(segment, public_addresses.clone()).unwrap();

        let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

        if let Some(final_root) = previous_final_root {
            assert_eq!(
                final_root, proof.public_data.initial_root,
                "Initial root of current segment should match final root of previous segment"
            );
        }
        previous_final_root = Some(proof.public_data.final_root);

        verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
    }
}

#[test]
fn test_prove_and_verify_all_opcodes() {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        "all_opcodes.cm"
    );
    let compiled_fib = compile_cairo(
        fs::read_to_string(&source_path).unwrap(),
        source_path,
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output =
        run_cairo_program(&compiled_fib.program, "main", &[], Default::default()).unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_addresses,
    )
    .unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

#[test]
fn test_all_opcodes_constraints() {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        "all_opcodes.cm"
    );
    let compiled_fib = compile_cairo(
        fs::read_to_string(&source_path).unwrap(),
        source_path,
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output =
        run_cairo_program(&compiled_fib.program, "main", &[], Default::default()).unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_addresses,
    )
    .unwrap();
    assert_constraints(&mut prover_input);
}

#[cfg(feature = "dhat-heap")]
#[test]
fn test_memory_profile_fibonacci_prover() {
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
        &[M31::from(100000)],
        Default::default(),
    )
    .unwrap();

    let _profiler = dhat::Profiler::new_heap();

    let mut prover_input = import_from_runner_output(
        runner_output.segments.into_iter().next().unwrap(),
        runner_output.public_addresses,
    )
    .unwrap();
    let _proof: cairo_m_prover::Proof<stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleHasher> =
        prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();
}
