pub mod poseidon_constants;
pub mod poseidon_params;

use num_traits::Zero;
use poseidon_constants::{mds_matrix, round_constants};
pub use poseidon_params::*;
use stwo_prover::core::fields::m31::M31;

use crate::adapter::merkle::MerkleHasher;

/// PoseidonHash implementation for M31 field.
///
/// All documentation on Poseidon : https://www.poseidon-hash.info
/// Poseidon paper: https://eprint.iacr.org/2019/458.pdf
/// Ingonyama Python implementation: https://github.com/ingonyama-zk/poseidon-hash
#[derive(Clone)]
pub struct PoseidonHash;

impl PoseidonHash {
    fn sbox(x: M31) -> M31 {
        x * x * x * x * x
    }

    /// Apply MDS matrix multiplication
    fn mds_multiply(state: &mut [M31; T]) {
        let mds = mds_matrix();
        let mut new_state = [M31::zero(); T];

        new_state
            .iter_mut()
            .zip(mds.iter())
            .for_each(|(new_elem, mds_row)| {
                *new_elem = mds_row
                    .iter()
                    .zip(state.iter())
                    .map(|(mds_elem, state_elem)| *mds_elem * *state_elem)
                    .fold(M31::zero(), |acc, val| acc + val);
            });

        *state = new_state;
    }

    /// Add round constants
    fn add_round_constants(state: &mut [M31; T], round: usize) {
        let offset = round * T;
        for (i, elem) in state.iter_mut().enumerate() {
            *elem += round_constants()[offset + i];
        }
    }

    /// Full round
    fn full_round(state: &mut [M31; T], round: usize) {
        Self::add_round_constants(state, round);

        // Apply S-box to all elements
        for elem in state.iter_mut() {
            *elem = Self::sbox(*elem);
        }

        Self::mds_multiply(state);
    }

    /// Partial round
    fn partial_round(state: &mut [M31; T], round: usize) {
        Self::add_round_constants(state, round);

        // Apply S-box only to first element
        state[0] = Self::sbox(state[0]);

        Self::mds_multiply(state);
    }

    /// Poseidon permutation
    fn permutation(input: [M31; T]) -> [M31; T] {
        let mut state = input;
        let mut round_counter = 0;

        // First half of full rounds
        for _ in 0..(FULL_ROUNDS / 2) {
            Self::full_round(&mut state, round_counter);
            round_counter += 1;
        }

        // Partial rounds
        for _ in 0..PARTIAL_ROUNDS {
            Self::partial_round(&mut state, round_counter);
            round_counter += 1;
        }

        // Second half of full rounds
        for _ in 0..(FULL_ROUNDS / 2) {
            Self::full_round(&mut state, round_counter);
            round_counter += 1;
        }

        state
    }
}

impl MerkleHasher for PoseidonHash {
    fn hash(left: M31, right: M31) -> M31 {
        let mut input = [M31::zero(); T];
        input[0] = left;
        input[1] = right;

        // Apply Poseidon permutation
        let output = Self::permutation(input);

        // Return first element as hash output
        output[0]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon_hash_basic() {
        // Basic test to ensure hash function works
        let left = M31::from(0);
        let right = M31::from(1);
        let hash1 = PoseidonHash::hash(left, right);

        // Hash should be deterministic
        let hash2 = PoseidonHash::hash(left, right);
        assert_eq!(hash1, hash2);

        // Different inputs should produce different outputs
        let hash3 = PoseidonHash::hash(right, left);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_poseidon_permutation_full_state() {
        // Test the full Poseidon permutation with initial state (1,1,0,0...)
        let mut input = [M31::zero(); T];
        input[0] = M31::from(1);
        input[1] = M31::from(1);

        // Apply Poseidon permutation
        let output = PoseidonHash::permutation(input);

        // Expected output state from the reference implementation, run with Python implementation
        //
        // poseidon.Poseidon(
        //     p=2**31 - 1,
        //     security_level=96,
        //     alpha=5,
        //     input_rate=2,
        //     t=2+7,
        //     full_round=8,
        //     partial_round=56,
        // )
        //
        // input_vec = [0 for _ in range(0, t)]
        // input_vec[0] = 1
        // input_vec[1] = 1
        //
        // poseidon_output = poseidon_new.run_hash(input_vec)
        let expected = [
            281984366, 1639677230, 1668030855, 1789277404, 369911947, 1865901295, 1243316563,
            1172538544, 151553736,
        ];

        // Verify all T elements of the output state
        for i in 0..T {
            assert_eq!(
                output[i],
                M31::from(expected[i]),
                "Mismatch at position {}: expected {}, {}",
                i,
                expected[i],
                output[i].0
            );
        }
    }
}
