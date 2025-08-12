use cairo_m_verifier::{
    hints::ProverInput, prover::prove_verifier, verifier::verify_verifier, Poseidon31MerkleChannel,
};

#[test]
fn test_prove_dummy() {
    let prover_input = ProverInput::default();

    let proof = prove_verifier::<Poseidon31MerkleChannel>(prover_input, None)
        .expect("Failed to generate proof");

    // Verify the proof using Poseidon31MerkleChannel
    verify_verifier::<Poseidon31MerkleChannel>(proof, None).expect("Failed to verify proof");

    println!("Proof verified successfully!");
}
