pub mod poseidon_constants;
pub mod poseidon_params;

use num_traits::Zero;
use poseidon_constants::{mds_matrix, round_constants};
pub use poseidon_params::*;
use stwo_prover::core::fields::m31::M31;

use crate::adapter::merkle::MerkleHasher;

/// PoseidonHash implementation for M31 field.
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

        // Expected output state from the reference implementation
        let expected = vec![
            726263122, 957689298, 2036206593, 1764508413, 1245579051, 91393672, 531792363,
            1582245613, 1776993172, 808634707, 329411827, 56677712, 640777984, 1346440999,
            1657063337, 1514060674, 1384028111, 626872333, 255049479, 1725476706, 940988290,
            45023827, 1678616669, 950008430, 855071155, 60721405, 1359420246, 1590505522,
            1141526820, 1611463331, 176280160, 295752685, 999783273, 1894484964, 1506220100,
            130758595, 746656033, 953495320, 1604756131, 55013281, 358870948, 2032782305,
            852333936, 1065173688, 479774515, 1286757598, 1016308652, 518709161, 720948568,
            388090815, 1729964471, 1359397972, 422637561, 127618366, 1955330307, 1174425502,
            1813616408, 376226661, 1411690022, 1911353739, 1908001006, 972383294, 2041023954,
            878650785, 311885391, 1352067309, 65122091, 1951828088, 447503564, 1844008348,
            729068768, 2013397322, 1188120095, 743531509, 1650609759, 37441950, 1368079710,
            1895188044, 1940847007, 1434465218, 942529204, 68011596, 1320934514, 1464669324,
            1808092057, 24666307, 51877649, 1590165295, 1425096678, 2055616401, 315946774,
            1409278787, 1500568658, 599879583, 1975800593, 1496803412, 1808546593, 217695710,
            184038670, 2006661361, 59029936, 407358127, 348346468, 2117906603, 880118893,
            201564520, 886377344, 1503075513, 88268629, 1563888778, 1912508269, 2008632016,
            1232191606, 802570877, 1258114684, 2118037356, 1066885360, 452006731, 1382342394,
            388334157, 605106474, 1059987636, 1961181274, 1034589877, 971748508, 425566501,
            1265695682, 931212042, 2011532259, 497915763, 2140270007, 1472818401, 785746637,
            1086290397, 496176558, 1530793621, 240915469, 1579503355, 1146918133, 1676475666,
            1538111268, 1134143787, 209946602, 546928425, 993060466, 1249249366, 1646310973,
            903624492, 221757093, 388017881, 1225255456, 883361745, 700641486, 1931357000,
            1748705476, 877953043, 1274922587, 741982435, 716906, 1642756789, 2045776671,
            660089101, 720749305, 1509037324, 1265977196, 716879983, 1114819052, 350266063,
            561597191, 580002245, 1988311925, 1598763802, 1620737069, 911668555, 1940549337,
            1070005304, 1944565324, 1848685154, 988912193, 670723965, 1846546359, 186494889,
            695489598, 1859785709, 1706822535, 1865180010, 36756379, 425618350, 1636708711,
            1671760843, 1706947255, 1767652316, 1620899387, 221479379,
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
