// Poseidon2 hash parameters shared between the implementation and build script
// This is the single source of truth for all Poseidon2 constants

use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use zkhash::ark_ff::PrimeField;
use zkhash::fields::m31::FpM31;
use zkhash::poseidon2::poseidon2::Poseidon2;
use zkhash::poseidon2::poseidon2_instance_m31::POSEIDON2_M31_16_PARAMS;

use crate::adapter::merkle::MerkleHasher;
type Scalar = FpM31;

/// ALPHA: power in the S-Box. The paper suggest x**5 as S-Box.
///
/// Note that stwo-cairo uses alpha=3.
/// We pick 5 so that gcd(PRIME-1, 5) = 1 holds, indeed gcd(PRIME-1, 3) = 3.
/// This const is exclusively used in the build script not for the hash computation nor the AIR (where x**5 s-box is hardcoded)
pub const ALPHA: u32 = 5;

/// P: The prime field modulus (2^31 - 1)
pub const P: u32 = 2_147_483_647;

/// Prime bit length
pub const PRIME_BIT_LEN: usize = 31;

/// T: State size
pub const T: usize = 16;

/// FULL_ROUNDS
/// The poseidon2 paper tests use 8 full rounds.
pub const FULL_ROUNDS: usize = 8;

/// PARTIAL_ROUNDS
/// The paper exposes different critieras for different attacks. The tests use 35 (M=80), 60 (M=128), 120 (M=256).
/// Stwo-cairo uses 31.
pub const PARTIAL_ROUNDS: usize = 14;

// Include the auto-generated constants from the build script
include!(concat!(env!("OUT_DIR"), "/poseidon2_constants.rs"));

#[derive(Clone)]
pub struct Poseidon2Hash;

impl MerkleHasher for Poseidon2Hash {
    fn hash(left: M31, right: M31) -> M31 {
        let poseidon2 = Poseidon2::new(&POSEIDON2_M31_16_PARAMS);
        let mut input: Vec<Scalar> = vec![Scalar::zero(); T];
        input[0] = Scalar::from(left.0);
        input[1] = Scalar::from(right.0);
        let perm = poseidon2.permutation(&input);
        M31::from(perm[0].into_bigint().0[0] as u32)
    }

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
