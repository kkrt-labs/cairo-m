//! Run these tests with feature `relation-tracker` to see the relation tracker output.
use std::collections::HashMap;

use cairo_m_common::InputValue;
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::memory::Memory;
use cairo_m_prover::adapter::merkle::{build_partial_merkle_tree, TreeType};
use cairo_m_prover::adapter::{
    import_from_runner_output, Instructions, MerkleTrees, PoseidonHashInput, ProverInput,
};
use cairo_m_prover::debug_tools::assert_constraints::assert_constraints;
use cairo_m_prover::poseidon2::Poseidon2Hash;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_prover::verifier::verify_cairo_m;
use cairo_m_runner::{run_cairo_program, RunnerOptions};
use cairo_m_test_utils::read_fixture;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn init_tracing() {
    tracing_subscriber::fmt().try_init().ok();
}

/// Tests proof generation and verification with static memory (no program execution).
///
/// This test creates a minimal proof scenario with only initial memory entries
/// that remain unchanged throughout execution.
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

    // Create HashMap using address as key
    let initial_memory: HashMap<M31, (QM31, M31, M31)> = initial_memory_data
        .iter()
        .map(|(address, value, clock, multiplicity)| (*address, (*value, *clock, *multiplicity)))
        .collect();

    let memory = Memory {
        initial_memory: initial_memory.clone(),
        final_memory: initial_memory,
        clock_update_data: vec![],
    };

    let public_address_ranges = cairo_m_common::PublicAddressRanges::default();
    let (initial_tree, initial_root) = build_partial_merkle_tree::<Poseidon2Hash>(
        &memory.initial_memory,
        TreeType::Initial,
        &public_address_ranges,
    );
    let (final_tree, final_root) = build_partial_merkle_tree::<Poseidon2Hash>(
        &memory.final_memory,
        TreeType::Final,
        &public_address_ranges,
    );

    let mut poseidon2_inputs =
        Vec::<PoseidonHashInput>::with_capacity(initial_tree.len() + final_tree.len());
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
        public_address_ranges: cairo_m_common::PublicAddressRanges {
            program: 0..0,
            input: 0..0,
            output: 0..0,
        },
        memory,
        instructions: Instructions::default(),
        poseidon2_inputs,
        sha256_inputs: (0..1 << 16)
            .map(|i| {
                let mut message = [M31::zero(); 32];
                for (j, element) in message.iter_mut().enumerate() {
                    *element = M31::from((i + j) as u32 & 0xFFFF);
                }
                message
            })
            .collect(),
    };

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    let result = verify_cairo_m::<Blake2sMerkleChannel>(proof, None);
    if let Err(e) = &result {
        eprintln!("Verification failed: {:?}", e);
    }
    assert!(result.is_ok());
}

/// Tests end-to-end proof generation for a Fibonacci(5) program.
///
/// This test compiles and executes a Cairo-M Fibonacci program, then generates
/// and verifies a STARK proof of correct execution.
#[test]
fn test_prove_and_verify_fibonacci_program() {
    init_tracing();
    let source = read_fixture("functions/fibonacci.cm");
    let compiled = compile_cairo(
        source,
        "fibonacci.cm".to_string(),
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output =
        run_cairo_program(&compiled.program, "fib", &[5.into()], Default::default()).unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

/// Tests proof generation for a Fibonacci(1M) calculation.
///
/// This test validates that the prover can handle larger execution traces
/// by computing Fibonacci of a much larger number (1,000,000). It tests the clock update component.
#[test]
fn test_prove_and_verify_large_fibonacci_program() {
    let source = read_fixture("functions/fib_loop.cm");
    let compiled = compile_cairo(
        source,
        "fib_loop.cm".to_string(),
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output = run_cairo_program(
        &compiled.program,
        "fibonacci_loop",
        &[InputValue::Number(1_000_000)],
        RunnerOptions {
            max_steps: 2_usize.pow(30),
        },
    )
    .unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

/// Tests proof generation for recursive Fibonacci implementation.
#[test]
fn test_prove_and_verify_recursive_fibonacci_program() {
    let source = read_fixture("functions/fibonacci.cm"); // Using same file as recursive version has same logic
    let compiled = compile_cairo(
        source,
        "recursive_fibonacci.cm".to_string(),
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output =
        run_cairo_program(&compiled.program, "fib", &[5.into()], Default::default()).unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

/// Tests Merkle root continuity across execution segments.
///
/// This test verifies that when execution is segmented (due to provided step limits),
/// the final memory root of one segment matches the initial memory root of
/// the next segment. This ensures proper continuity in segmented proofs.
#[test]
fn test_hash_continuity_fibonacci() {
    let source = read_fixture("functions/fib_loop.cm");
    let compiled = compile_cairo(
        source,
        "fibonacci.cm".to_string(),
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_options = RunnerOptions { max_steps: 10 };

    let runner_output = run_cairo_program(
        &compiled.program,
        "fibonacci_loop",
        &[5.into()],
        runner_options,
    )
    .unwrap();

    let public_address_ranges = runner_output.public_address_ranges.clone();

    let mut previous_final_root: Option<M31> = None;

    for segment in runner_output.vm.segments {
        let mut prover_input =
            import_from_runner_output(segment, public_address_ranges.clone()).unwrap();

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

/// Test proof generation for sha256 program.
#[test]
fn test_prove_and_verify_sha256_program() {
    // Include SHA256 source at compile time
    let source = include_str!("../../../examples/sha256-cairo-m/src/sha256.cm").to_string();

    let compiled = compile_cairo(
        source,
        "sha256.cm".to_string(),
        CompilerOptions::no_opts(), // Use no_opts like all_opcodes test
    )
    .unwrap();

    // Prepare input for SHA256 - test with "abc"
    const MAX_CHUNKS: usize = 2;
    const PADDED_BUFFER_U32_SIZE: usize = (MAX_CHUNKS * 64) / 4;

    let msg = b"abc";

    // Perform standard SHA-256 padding
    let mut padded_bytes = msg.to_vec();
    padded_bytes.push(0x80);

    // Pad to 56 bytes (448 bits) within the last chunk
    while padded_bytes.len() % 64 != 56 {
        padded_bytes.push(0x00);
    }

    // Append message length as 64-bit big-endian
    let bit_len = (msg.len() as u64) * 8;
    padded_bytes.extend_from_slice(&bit_len.to_be_bytes());

    let num_chunks = padded_bytes.len() / 64;

    // Convert bytes to u32 words (big-endian)
    let mut padded_words: Vec<u32> = padded_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().expect("Chunk size mismatch")))
        .collect();

    // Pad to fixed buffer size
    padded_words.resize(PADDED_BUFFER_U32_SIZE, 0);

    // Convert to InputValue format
    let padded_buffer = padded_words
        .into_iter()
        .map(|word| InputValue::Number(i64::from(word)))
        .collect();

    let args = vec![
        InputValue::List(padded_buffer),
        InputValue::Number(num_chunks as i64),
    ];

    let runner_output =
        run_cairo_program(&compiled.program, "sha256_hash", &args, Default::default())
            .expect("Failed to run SHA256 program");

    let mut prover_input = import_from_runner_output(
        runner_output
            .vm
            .segments
            .into_iter()
            .next()
            .expect("No segments in runner output"),
        runner_output.public_address_ranges,
    )
    .expect("Failed to import runner output for SHA256");
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

/// Tests proof generation for comprehensive opcode coverage.
///
/// This test executes a Cairo-M program that exercises all available
/// opcodes in the instruction set with as most operand configuration as possible.
/// To be updated if new opcodes/new functionalities are added.
#[test]
fn test_prove_and_verify_all_opcodes() {
    let source = read_fixture("functions/all_opcodes.cm");
    let compiled = compile_cairo(
        source,
        "all_opcodes.cm".to_string(),
        CompilerOptions::no_opts(),
    )
    .unwrap();

    let runner_output =
        run_cairo_program(&compiled.program, "main", &[], Default::default()).unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();
    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();

    verify_cairo_m::<Blake2sMerkleChannel>(proof, None).unwrap();
}

/// Tests constraint satisfaction for all opcode types without full proving.
///
/// This test validates that all opcode constraint systems are satisfied
/// by the execution trace, without generating a complete STARK proof.
/// The constraints are evaluated with the trace values (no interpolation).
#[test]
fn test_all_opcodes_constraints() {
    let source = read_fixture("functions/all_opcodes.cm");
    let compiled = compile_cairo(
        source,
        "all_opcodes.cm".to_string(),
        CompilerOptions::no_opts(),
    )
    .unwrap();

    let runner_output =
        run_cairo_program(&compiled.program, "main", &[], Default::default()).unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();
    assert_constraints(&mut prover_input);
}

#[test]
fn test_fibonacci_public_memory_contents() {
    let source = read_fixture("functions/fibonacci.cm");
    let compiled_fib = compile_cairo(
        source,
        "fibonacci.cm".to_string(),
        CompilerOptions::default(),
    )
    .unwrap();

    let input_arg = M31::from(5);
    let runner_output = run_cairo_program(
        &compiled_fib.program,
        "fib",
        &[input_arg.into()],
        Default::default(),
    )
    .unwrap();

    let expected_return_value: M31 = runner_output.return_values[0].clone().try_into().unwrap();

    let mut prover_input = import_from_runner_output(
        runner_output.vm.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();

    let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();
    let public_data = &proof.public_data;

    // Test 1: Verify return value in final public memory output
    let output_values = public_data.public_memory.get_output_values();
    assert_eq!(output_values.len(), 1, "Expected 1 return value");
    assert_eq!(
        output_values[0].unwrap(),
        expected_return_value.into(),
        "Output should match runner output"
    );

    // Test 2: Verify input argument in initial and final public memory
    let input_values = public_data.public_memory.get_input_values();

    assert_eq!(input_values.len(), 1, "Expected 1 initial input");
    assert_eq!(
        input_values[0].unwrap(),
        input_arg.into(),
        "Input should be 5"
    );

    // Test 3: Compare program in public memory to compiled program
    let program_values = public_data.public_memory.get_program_values();

    // Convert compiled program instructions to QM31 for comparison
    let compiled_instructions: Vec<QM31> = compiled_fib
        .program
        .data
        .iter()
        .flat_map(|pg_data| pg_data.to_qm31_vec())
        .collect();

    assert_eq!(
        program_values.len(),
        compiled_instructions.len(),
        "Program length should match compiled program"
    );

    // Verify each instruction matches
    for (i, &expected_instruction) in compiled_instructions.iter().enumerate() {
        assert_eq!(
            program_values[i].unwrap(),
            expected_instruction,
            "Program instruction {} should match compiled program",
            i
        );
    }
}

/// Memory profiling test for Fibonacci proof generation (requires dhat-heap feature).
///
/// This test profiles memory usage during STARK proof generation for
/// performance analysis and optimization. It's conditionally compiled
/// and only runs when the `dhat-heap` feature is enabled.
#[cfg(feature = "dhat-heap")]
#[test]
fn test_memory_profile_fibonacci_prover() {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        "fibonacci.cm"
    );
    let compiled = compile_cairo(
        fs::read_to_string(&source_path).unwrap(),
        source_path,
        CompilerOptions::default(),
    )
    .unwrap();

    let runner_output = run_cairo_program(
        &compiled.program,
        "fib",
        &[M31::from(100000)],
        Default::default(),
    )
    .unwrap();

    let _profiler = dhat::Profiler::new_heap();

    let mut prover_input = import_from_runner_output(
        runner_output.segments.into_iter().next().unwrap(),
        runner_output.public_address_ranges,
    )
    .unwrap();
    let _proof: cairo_m_prover::Proof<stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleHasher> =
        prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).unwrap();
}
