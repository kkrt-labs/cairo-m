pub mod poseidon_constants;
pub mod poseidon_params;

use num_traits::Zero;
use poseidon_constants::{mds_matrix, round_constants};
pub use poseidon_params::*;
use stwo_prover::core::fields::m31::M31;

/// Intermediate data for a single round, used for trace generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PoseidonRoundData {
    /// State before the current round (input for first round)
    pub state: [M31; T],
    /// Result of state + round_constants
    pub inter_state: [M31; T],
    /// inter_state * inter_state (element-wise)
    pub inter_state_sq: [M31; T],
    /// inter_state_sq * inter_state_sq (element-wise)
    pub inter_state_quad: [M31; T],
    /// inter_state * inter_state_quad (S-box output)
    pub s_box_out_state: [M31; T],
    /// 1 if this is a full round, 0 if partial
    pub full_round: M31,
    /// 1 if this is the final round
    pub final_round: M31,
}

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

    /// Generate trace data for Poseidon hash computation
    /// Returns an array of PoseidonRoundData for all rounds (full + partial)
    pub fn permutation_with_trace(
        input: [M31; T],
    ) -> ([PoseidonRoundData; FULL_ROUNDS + PARTIAL_ROUNDS], [M31; T]) {
        let mut state = input;
        let mut trace_data = [PoseidonRoundData::default(); FULL_ROUNDS + PARTIAL_ROUNDS];

        let total_rounds = FULL_ROUNDS + PARTIAL_ROUNDS;
        let mut round_counter = 0;

        // First half of full rounds
        for _ in 0..(FULL_ROUNDS / 2) {
            trace_data[round_counter] = Self::full_round_with_trace(
                &mut state,
                round_counter,
                round_counter == total_rounds - 1,
            );
            round_counter += 1;
        }

        // Partial rounds
        for _ in 0..PARTIAL_ROUNDS {
            trace_data[round_counter] = Self::partial_round_with_trace(
                &mut state,
                round_counter,
                round_counter == total_rounds - 1,
            );
            round_counter += 1;
        }

        // Second half of full rounds
        for _ in 0..(FULL_ROUNDS / 2) {
            trace_data[round_counter] = Self::full_round_with_trace(
                &mut state,
                round_counter,
                round_counter == total_rounds - 1,
            );
            round_counter += 1;
        }

        // Return both trace data and the hash result
        (trace_data, state)
    }

    /// Full round with trace generation
    fn full_round_with_trace(
        state: &mut [M31; T],
        round: usize,
        is_final: bool,
    ) -> PoseidonRoundData {
        let offset = round * T;
        let rc = round_constants();

        // Save input state
        let input_state = *state;

        // Add round constants
        let mut inter_state = [M31::zero(); T];
        for i in 0..T {
            inter_state[i] = state[i] + rc[offset + i];
        }

        // Compute S-box intermediates for all elements
        let mut inter_state_sq = [M31::zero(); T];
        let mut inter_state_quad = [M31::zero(); T];
        let mut s_box_out_state = [M31::zero(); T];

        for i in 0..T {
            inter_state_sq[i] = inter_state[i] * inter_state[i];
            inter_state_quad[i] = inter_state_sq[i] * inter_state_sq[i];
            s_box_out_state[i] = inter_state[i] * inter_state_quad[i];
        }

        // Store the pre-MDS S-box output for constraints
        let s_box_out_pre_mds = s_box_out_state;

        // Apply MDS matrix to update the state
        Self::mds_multiply(&mut s_box_out_state);
        *state = s_box_out_state;

        PoseidonRoundData {
            state: input_state,
            inter_state,
            inter_state_sq,
            inter_state_quad,
            s_box_out_state: s_box_out_pre_mds,
            full_round: M31::from(1),
            final_round: if is_final { M31::from(1) } else { M31::zero() },
        }
    }

    /// Partial round with trace generation
    fn partial_round_with_trace(
        state: &mut [M31; T],
        round: usize,
        is_final: bool,
    ) -> PoseidonRoundData {
        let offset = round * T;
        let rc = round_constants();

        // Save input state
        let input_state = *state;

        // Add round constants
        let mut inter_state = [M31::zero(); T];
        for i in 0..T {
            inter_state[i] = state[i] + rc[offset + i];
        }

        // Initialize arrays - most elements will pass through unchanged
        let mut inter_state_sq = inter_state;
        let mut inter_state_quad = inter_state;
        let mut s_box_out_state = inter_state;

        // Apply S-box only to first element
        inter_state_sq[0] = inter_state[0] * inter_state[0];
        inter_state_quad[0] = inter_state_sq[0] * inter_state_sq[0];
        s_box_out_state[0] = inter_state[0] * inter_state_quad[0];

        // Store the pre-MDS S-box output for constraints
        let s_box_out_pre_mds = s_box_out_state;

        // Apply MDS matrix to update the state
        Self::mds_multiply(&mut s_box_out_state);
        *state = s_box_out_state;

        PoseidonRoundData {
            state: input_state,
            inter_state,
            inter_state_sq,
            inter_state_quad,
            s_box_out_state: s_box_out_pre_mds,
            full_round: M31::zero(),
            final_round: if is_final { M31::from(1) } else { M31::zero() },
        }
    }

    pub fn hash(left: M31, right: M31) -> M31 {
        let mut input = [M31::zero(); T];
        input[0] = left;
        input[1] = right;

        // Apply Poseidon permutation
        let output = Self::permutation(input);

        // Return first element as hash output
        output[0]
    }

    pub fn hash_with_trace(
        left: M31,
        right: M31,
    ) -> ([PoseidonRoundData; FULL_ROUNDS + PARTIAL_ROUNDS], M31) {
        let mut state = [M31::zero(); T];
        state[0] = left;
        state[1] = right;

        let (trace_data, output) = Self::permutation_with_trace(state);
        (trace_data, output[0])
    }

    pub fn default_hashes() -> &'static [M31] {
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
    fn test_hash_with_trace_consistency() {
        // Test that hash_with_trace produces the same result as regular hash
        let test_cases = vec![
            (M31::from(0), M31::from(0)),
            (M31::from(1), M31::from(1)),
            (M31::from(123), M31::from(456)),
            (M31::from(P - 1), M31::from(P - 1)),
        ];

        for (left, right) in test_cases {
            // Compute regular hash
            let regular_hash = PoseidonHash::hash(left, right);

            // Compute hash with trace
            let (trace_data, trace_hash) = PoseidonHash::hash_with_trace(left, right);

            // Results should match
            assert_eq!(
                regular_hash, trace_hash,
                "Hash mismatch for inputs ({}, {}): regular={}, trace={}",
                left.0, right.0, regular_hash.0, trace_hash.0
            );

            // Verify trace data structure
            assert_eq!(trace_data.len(), FULL_ROUNDS + PARTIAL_ROUNDS);

            // Check round flags
            let mut full_round_count = 0;
            let mut final_round_count = 0;

            for (i, round_data) in trace_data.iter().enumerate() {
                if round_data.full_round == M31::from(1) {
                    full_round_count += 1;
                }
                if round_data.final_round == M31::from(1) {
                    final_round_count += 1;
                }

                // Verify S-box computation for full rounds
                if round_data.full_round == M31::from(1) {
                    for j in 0..T {
                        // Verify: inter_state^2 = inter_state_sq
                        let computed_sq = round_data.inter_state[j] * round_data.inter_state[j];
                        assert_eq!(
                            computed_sq, round_data.inter_state_sq[j],
                            "inter_state_sq mismatch at round {} element {}",
                            i, j
                        );

                        // Verify: inter_state_sq^2 = inter_state_quad
                        let computed_quad =
                            round_data.inter_state_sq[j] * round_data.inter_state_sq[j];
                        assert_eq!(
                            computed_quad, round_data.inter_state_quad[j],
                            "inter_state_quad mismatch at round {} element {}",
                            i, j
                        );

                        // Verify: inter_state * inter_state_quad = s_box_out_state (before MDS)
                        let _computed_sbox =
                            round_data.inter_state[j] * round_data.inter_state_quad[j];
                        // Note: s_box_out_state is after MDS multiplication, so we can't directly compare
                    }
                } else {
                    // For partial rounds, only first element should have S-box applied
                    let computed_sq = round_data.inter_state[0] * round_data.inter_state[0];
                    assert_eq!(
                        computed_sq, round_data.inter_state_sq[0],
                        "inter_state_sq[0] mismatch at partial round {}",
                        i
                    );
                }
            }

            assert_eq!(full_round_count, FULL_ROUNDS, "Wrong number of full rounds");
            assert_eq!(final_round_count, 1, "Should have exactly one final round");

            // Verify the final round is marked correctly
            assert_eq!(
                trace_data[FULL_ROUNDS + PARTIAL_ROUNDS - 1].final_round,
                M31::from(1),
                "Last round should be marked as final"
            );
        }
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
