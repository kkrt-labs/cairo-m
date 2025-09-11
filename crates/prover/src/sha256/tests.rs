use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

use crate::adapter::SHA256HashInput;
use crate::sha256::debug_tools::assert_constraints::assert_constraints;
use crate::sha256::prover_sha256::prove_sha256;

#[test]
fn test_sha256_constraints_empty_input() {
    let mut inputs: Vec<SHA256HashInput> = vec![];
    assert_constraints(&mut inputs);
}

#[test]
fn test_sha256_constraints_single_block() {
    // Create a single SHA256 block input (32 M31 elements = 512 bits)
    let mut inputs: Vec<SHA256HashInput> = vec![[M31::from(42); 32]];
    assert_constraints(&mut inputs);
}

#[test]
fn test_sha256_prove_empty_input() {
    let inputs: Vec<SHA256HashInput> = vec![];
    prove_sha256::<Blake2sMerkleChannel>(&inputs, None).unwrap();
}

#[test]
fn test_sha256_prove_single_block() {
    let inputs: Vec<SHA256HashInput> = vec![[M31::from(42); 32]];
    prove_sha256::<Blake2sMerkleChannel>(&inputs, None).unwrap();
}

#[test]
fn test_sha256_prove_2_pow_x_input() {
    let inputs: Vec<SHA256HashInput> = (0..1 << 16)
        .map(|i| {
            let mut message = [M31::zero(); 32];
            for (j, element) in message.iter_mut().enumerate() {
                *element = M31::from((i + j) as u32 & 0xFFFF);
            }
            message
        })
        .collect();
    prove_sha256::<Blake2sMerkleChannel>(&inputs, None).unwrap();
}
