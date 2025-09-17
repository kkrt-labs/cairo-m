use std::{fs, path::PathBuf};

use cairo_m_verifier::{
    debug_tools::assert_constraints::assert_constraints,
    hints::{generate_hints, ProverInput},
    prover::prove_verifier,
    verifier::verify_verifier,
    Poseidon31MerkleChannel, Poseidon31MerkleHasher,
};

fn get_proof_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_data")
        .join(filename)
}

#[test]
fn test_prove_dummy() {
    let prover_input = ProverInput::default();

    let proof = prove_verifier::<Poseidon31MerkleChannel>(prover_input, None)
        .expect("Failed to generate proof");

    // Verify the proof using Poseidon31MerkleChannel
    verify_verifier::<Poseidon31MerkleChannel>(proof, None).expect("Failed to verify proof");

    println!("Proof verified successfully!");
}

#[test]
fn test_verify_fibonacci_proof() {
    let proof_path = get_proof_path("fibonacci_proof.json");
    let proof_json = fs::read_to_string(&proof_path).expect("Failed to read proof file.");
    let cairo_m_proof: cairo_m_prover::Proof<Poseidon31MerkleHasher> =
        serde_json::from_str(&proof_json).unwrap();

    let prover_input = generate_hints(&cairo_m_proof);
    let proof = prove_verifier::<Poseidon31MerkleChannel>(prover_input, None).unwrap();

    // Verify the proof using Poseidon31MerkleChannel
    verify_verifier::<Poseidon31MerkleChannel>(proof, None).unwrap();
}

#[test]
fn test_constraints() {
    let proof_path = get_proof_path("fibonacci_proof.json");
    let proof_json = fs::read_to_string(&proof_path).expect("Failed to read proof file.");
    let proof: cairo_m_prover::Proof<Poseidon31MerkleHasher> =
        serde_json::from_str(&proof_json).unwrap();

    let prover_input = generate_hints(&proof);
    assert_constraints(prover_input);
    panic!("Constraints passed");
}
