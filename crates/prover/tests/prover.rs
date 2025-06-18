use cairo_m_prover::adapter::instructions::Instructions;
use cairo_m_prover::adapter::memory::MemoryBoundaries;
use cairo_m_prover::adapter::ProverInput;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_prover::verifier::verify_cairo_m;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

#[test]
fn test_prove_and_verify_unchanged_memory() {
    let initial_memory = vec![
        (0, [1, 2, 3, 4], 0),
        (1, [5, 6, 7, 8], 0),
        (2, [9, 10, 11, 12], 0),
        (3, [13, 14, 15, 16], 0),
    ];

    let memory_boundaries = MemoryBoundaries {
        initial_memory: initial_memory.clone(),
        final_memory: initial_memory,
    };

    let prover_input = ProverInput {
        memory_boundaries,
        instructions: Instructions::default(),
    };

    let proof = prove_cairo_m::<Blake2sMerkleChannel, 1>(prover_input).unwrap();

    let result = verify_cairo_m::<Blake2sMerkleChannel, 1>(proof);
    assert!(result.is_ok());
}

#[test]
fn test_prove_and_verify_empty_memory() {
    let initial_memory = vec![];

    let memory_boundaries = MemoryBoundaries {
        initial_memory: initial_memory.clone(),
        final_memory: initial_memory,
    };

    let prover_input = ProverInput {
        memory_boundaries,
        instructions: Instructions::default(),
    };

    let proof = prove_cairo_m::<Blake2sMerkleChannel, 1>(prover_input).unwrap();

    let result = verify_cairo_m::<Blake2sMerkleChannel, 1>(proof);
    assert!(result.is_ok());
}
