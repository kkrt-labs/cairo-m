use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_prover::verifier::verify_cairo_m;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

#[test]
fn test_prove_cairo_m() {
    let result = prove_cairo_m::<Blake2sMerkleChannel, 3>(13);
    assert!(result.is_ok());
}

#[test]
fn test_verify_cairo_m() {
    let proof = prove_cairo_m::<Blake2sMerkleChannel, 3>(13).unwrap();
    let result = verify_cairo_m::<Blake2sMerkleChannel, 3>(proof);
    assert!(result.is_ok());
}
