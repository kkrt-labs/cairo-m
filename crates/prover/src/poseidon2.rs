use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use zkhash::ark_ff::PrimeField;
use zkhash::fields::m31::FpM31;
use zkhash::poseidon2::poseidon2::Poseidon2;
use zkhash::poseidon2::poseidon2_instance_m31::POSEIDON2_M31_16_PARAMS;

use crate::adapter::merkle::MerkleHasher;

/// T: State size
pub const T: usize = 16;
pub const FULL_ROUNDS: usize = 8;
pub const PARTIAL_ROUNDS: usize = 14;

// Include the auto-generated constants from the build script
include!(concat!(env!("OUT_DIR"), "/poseidon2_constants.rs"));

#[derive(Clone)]
pub struct Poseidon2Hash;

impl MerkleHasher for Poseidon2Hash {
    /// Uses reference implementation to compute the hash.
    /// The initial state is built by padding [left, right] with 0 to size T.
    /// Digest is the first element of the final state
    fn hash(left: M31, right: M31) -> M31 {
        let poseidon2 = Poseidon2::new(&POSEIDON2_M31_16_PARAMS);
        let mut input: Vec<FpM31> = vec![FpM31::zero(); T];
        input[0] = FpM31::from(left.0);
        input[1] = FpM31::from(right.0);
        let perm = poseidon2.permutation(&input);
        M31::from(perm[0].into_bigint().0[0] as u32)
    }

    /// Default hash computation
    fn default_hashes() -> &'static [M31] {
        use std::sync::OnceLock;

        use crate::adapter::merkle::TREE_HEIGHT;

        static DEFAULT_HASHES: OnceLock<Vec<M31>> = OnceLock::new();

        DEFAULT_HASHES.get_or_init(|| {
            let mut defaults = vec![M31::zero(); (TREE_HEIGHT + 1) as usize];

            // Depth 30 (leaves): zero values
            defaults[TREE_HEIGHT as usize] = M31::zero();

            // Compute default hashes for each depth from leaves to root
            for depth in (0..TREE_HEIGHT).rev() {
                let child_default = defaults[(depth + 1) as usize];
                defaults[depth as usize] = Self::hash(child_default, child_default);
            }

            defaults
        })
    }
}
