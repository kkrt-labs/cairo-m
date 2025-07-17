use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;

use super::merkle::MerkleHasher;

/// PoseidonHash implementation for M31 field
/// This is a simplified version adapted for the M31 field (2^31 - 1)
///
/// ## Usage
///
/// To use PoseidonHash instead of MockHasher in the prover, replace:
/// ```rust
/// build_partial_merkle_tree::<MockHasher>(&mut memory)
/// ```
///
/// With:
/// ```rust
/// build_partial_merkle_tree::<PoseidonHash>(&mut memory)
/// ```
///
/// Similarly, when calling `Claim::write_trace`, specify PoseidonHash:
/// ```rust
/// Claim::write_trace::<MC, PoseidonHash>(merkle_trees)
/// ```
#[derive(Clone)]
pub struct PoseidonHash;

/// Constants for Poseidon hash in M31
/// These would typically be generated using the Poseidon paper's algorithm
/// For now, using placeholder values that should be replaced with properly generated constants
const T: usize = 3; // State size (2 inputs + 1 capacity)
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 22;

/// Round constants - these should be properly generated
/// Using placeholder values for now
const ROUND_CONSTANTS: [M31; (FULL_ROUNDS + PARTIAL_ROUNDS) * T] = [
    M31(0x00000001),
    M31(0x00000002),
    M31(0x00000003),
    M31(0x00000004),
    M31(0x00000005),
    M31(0x00000006),
    M31(0x00000007),
    M31(0x00000008),
    M31(0x00000009),
    M31(0x0000000a),
    M31(0x0000000b),
    M31(0x0000000c),
    M31(0x0000000d),
    M31(0x0000000e),
    M31(0x0000000f),
    M31(0x00000010),
    M31(0x00000011),
    M31(0x00000012),
    M31(0x00000013),
    M31(0x00000014),
    M31(0x00000015),
    M31(0x00000016),
    M31(0x00000017),
    M31(0x00000018),
    M31(0x00000019),
    M31(0x0000001a),
    M31(0x0000001b),
    M31(0x0000001c),
    M31(0x0000001d),
    M31(0x0000001e),
    M31(0x0000001f),
    M31(0x00000020),
    M31(0x00000021),
    M31(0x00000022),
    M31(0x00000023),
    M31(0x00000024),
    M31(0x00000025),
    M31(0x00000026),
    M31(0x00000027),
    M31(0x00000028),
    M31(0x00000029),
    M31(0x0000002a),
    M31(0x0000002b),
    M31(0x0000002c),
    M31(0x0000002d),
    M31(0x0000002e),
    M31(0x0000002f),
    M31(0x00000030),
    M31(0x00000031),
    M31(0x00000032),
    M31(0x00000033),
    M31(0x00000034),
    M31(0x00000035),
    M31(0x00000036),
    M31(0x00000037),
    M31(0x00000038),
    M31(0x00000039),
    M31(0x0000003a),
    M31(0x0000003b),
    M31(0x0000003c),
    M31(0x0000003d),
    M31(0x0000003e),
    M31(0x0000003f),
    M31(0x00000040),
    M31(0x00000041),
    M31(0x00000042),
    M31(0x00000043),
    M31(0x00000044),
    M31(0x00000045),
    M31(0x00000046),
    M31(0x00000047),
    M31(0x00000048),
    M31(0x00000049),
    M31(0x0000004a),
    M31(0x0000004b),
    M31(0x0000004c),
    M31(0x0000004d),
    M31(0x0000004e),
    M31(0x0000004f),
    M31(0x00000050),
    M31(0x00000051),
    M31(0x00000052),
    M31(0x00000053),
    M31(0x00000054),
    M31(0x00000055),
    M31(0x00000056),
    M31(0x00000057),
    M31(0x00000058),
    M31(0x00000059),
    M31(0x0000005a),
];

/// MDS matrix for mixing - simplified 3x3 matrix
/// These values should be properly generated for cryptographic security
const MDS_MATRIX: [[M31; 3]; 3] = [
    [M31(0x00000003), M31(0x00000001), M31(0x00000001)],
    [M31(0x00000001), M31(0x00000003), M31(0x00000001)],
    [M31(0x00000001), M31(0x00000001), M31(0x00000003)],
];

impl PoseidonHash {
    /// S-box: raise to the power of ALPHA
    fn sbox(x: M31) -> M31 {
        // x^5 in M31 field
        let x2 = x * x;
        let x4 = x2 * x2;
        x4 * x
    }

    /// Apply MDS matrix multiplication
    #[allow(clippy::needless_range_loop)]
    fn mds_multiply(state: &mut [M31; T]) {
        let mut new_state = [M31::zero(); T];

        for i in 0..T {
            for j in 0..T {
                new_state[i] += MDS_MATRIX[i][j] * state[j];
            }
        }

        *state = new_state;
    }

    /// Add round constants
    fn add_round_constants(state: &mut [M31; T], round: usize) {
        let offset = round * T;
        for i in 0..T {
            state[i] += ROUND_CONSTANTS[offset + i];
        }
    }

    /// Full round: AddRoundConstants -> SubWords (S-box) -> MixLayer (MDS)
    fn full_round(state: &mut [M31; T], round: usize) {
        Self::add_round_constants(state, round);

        // Apply S-box to all elements
        for elem in state.iter_mut() {
            *elem = Self::sbox(*elem);
        }

        Self::mds_multiply(state);
    }

    /// Partial round: AddRoundConstants -> SubWords (S-box on first element only) -> MixLayer (MDS)
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
        // Initialize state with [left, right, 0] (capacity element)
        let input = [left, right, M31::zero()];

        // Apply Poseidon permutation
        let output = Self::permutation(input);

        // Return first element as hash output
        output[0]
    }

    fn default_hashes() -> &'static [M31] {
        use std::sync::OnceLock;

        use super::merkle::TREE_HEIGHT;

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
        let left = M31::from(42);
        let right = M31::from(123);
        let hash1 = PoseidonHash::hash(left, right);

        // Hash should be deterministic
        let hash2 = PoseidonHash::hash(left, right);
        assert_eq!(hash1, hash2);

        // Different inputs should produce different outputs
        let hash3 = PoseidonHash::hash(right, left);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_poseidon_default_hashes() {
        let defaults = PoseidonHash::default_hashes();

        // Should have correct length
        assert_eq!(defaults.len(), 31); // TREE_HEIGHT + 1

        // Leaf default should be zero
        assert_eq!(defaults[30], M31::zero());

        // Each level should be hash of two children from level below
        for depth in (0..30).rev() {
            let child_default = defaults[depth + 1];
            let expected = PoseidonHash::hash(child_default, child_default);
            assert_eq!(defaults[depth as usize], expected);
        }
    }

    #[test]
    fn test_sbox() {
        // Test S-box function
        let x = M31::from(7);
        let result = PoseidonHash::sbox(x);

        // Should be x^5
        let expected = x * x * x * x * x;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_poseidon_merkle_tree() {
        use std::collections::HashMap;

        use stwo_prover::core::fields::qm31::QM31;

        use crate::adapter::merkle::build_partial_merkle_tree;

        // Test building a merkle tree with Poseidon hash
        let mut memory = HashMap::new();
        memory.insert(
            (M31::from(0), M31::from(super::super::merkle::TREE_HEIGHT)),
            (QM31::from(42), M31::zero(), M31::zero()),
        );
        memory.insert(
            (M31::from(1), M31::from(super::super::merkle::TREE_HEIGHT)),
            (QM31::from(123), M31::zero(), M31::zero()),
        );

        let mut memory_clone = memory.clone();
        let (tree, root) = build_partial_merkle_tree::<PoseidonHash>(&mut memory);

        // Should have a tree and root
        assert!(!tree.is_empty());
        assert!(root.is_some());

        // Root should be deterministic when building from same initial memory
        let (_, root2) = build_partial_merkle_tree::<PoseidonHash>(&mut memory_clone);
        assert_eq!(root, root2);
    }
}
