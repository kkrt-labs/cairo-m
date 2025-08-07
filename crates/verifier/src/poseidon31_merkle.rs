use itertools::Itertools;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Col, Column, ColumnOps};
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SecureField, SECURE_EXTENSION_DEGREE};
use stwo_prover::core::proof_of_work::GrindOps;
use stwo_prover::core::vcs::hash::Hash;
use stwo_prover::core::vcs::ops::{MerkleHasher, MerkleOps};
use zkhash::ark_ff::PrimeField;
use zkhash::fields::m31::FpM31;
use zkhash::poseidon2::poseidon2::Poseidon2;
use zkhash::poseidon2::poseidon2_instance_m31::POSEIDON2_M31_16_PARAMS;

use rayon::prelude::*;

const ELEMENTS_IN_BLOCK: usize = 8;
const T: usize = 16; // State size for Poseidon2

/// Wrapper type for M31 to implement Hash trait
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct M31Hash(pub M31);

impl Display for M31Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<M31> for M31Hash {
    fn from(value: M31) -> Self {
        Self(value)
    }
}

impl From<M31Hash> for M31 {
    fn from(value: M31Hash) -> Self {
        value.0
    }
}

impl Hash for M31Hash {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct Poseidon31MerkleHasher;

impl Poseidon31MerkleHasher {
    /// Hash two M31 values using Poseidon2
    fn hash_pair(left: M31, right: M31) -> M31 {
        let poseidon2 = Poseidon2::new(&POSEIDON2_M31_16_PARAMS);
        let mut input: Vec<FpM31> = vec![FpM31::zero(); T];
        input[0] = FpM31::from(left.0);
        input[1] = FpM31::from(right.0);
        let perm = poseidon2.permutation(&input);
        M31::from(perm[0].into_bigint().0[0] as u32)
    }

    /// Hash multiple M31 values by chaining pairs
    fn hash_many(values: &[M31]) -> M31 {
        if values.is_empty() {
            return M31::zero();
        }
        if values.len() == 1 {
            return values[0];
        }

        // Hash values in pairs and chain them
        let mut result = Self::hash_pair(values[0], values[1]);
        for value in values.iter().skip(2) {
            result = Self::hash_pair(result, *value);
        }
        result
    }
}

impl MerkleHasher for Poseidon31MerkleHasher {
    type Hash = M31Hash;

    fn hash_node(
        children_hashes: Option<(Self::Hash, Self::Hash)>,
        column_values: &[BaseField],
    ) -> Self::Hash {
        let n_column_blocks = column_values.len().div_ceil(ELEMENTS_IN_BLOCK);
        let mut values = Vec::with_capacity(2 + n_column_blocks);

        if let Some((left, right)) = children_hashes {
            values.push(left.0);
            values.push(right.0);
        }

        // Process column values in blocks of ELEMENTS_IN_BLOCK
        let padding_length = ELEMENTS_IN_BLOCK * n_column_blocks - column_values.len();
        let padded_values = column_values
            .iter()
            .copied()
            .chain(std::iter::repeat_n(BaseField::zero(), padding_length));

        for chunk in padded_values.collect::<Vec<_>>().chunks(ELEMENTS_IN_BLOCK) {
            // Hash each block of values
            let block_hash = Self::hash_many(chunk);
            values.push(block_hash);
        }

        // Hash all collected values together
        M31Hash(Self::hash_many(&values))
    }
}

/// A channel implementation using Poseidon31 hash
#[derive(Clone, Default, Debug)]
pub struct Poseidon31Channel {
    digest: M31,
    channel_time_n_challenges: usize,
    channel_time_n_sent: u32,
}

impl Poseidon31Channel {
    pub const fn digest(&self) -> M31 {
        self.digest
    }

    pub fn update_digest(&mut self, new_digest: M31) {
        self.digest = new_digest;
        self.channel_time_n_challenges += 1;
        self.channel_time_n_sent = 0;
    }

    fn draw_m31(&mut self) -> M31 {
        let res =
            Poseidon31MerkleHasher::hash_pair(self.digest, M31::from(self.channel_time_n_sent));
        self.channel_time_n_sent += 1;
        res
    }

    fn draw_base_felts(&mut self) -> [BaseField; 8] {
        // Draw 8 M31 values
        std::array::from_fn(|_| self.draw_m31())
    }
}

impl Channel for Poseidon31Channel {
    const BYTES_PER_HASH: usize = 4; // M31 is 4 bytes

    fn trailing_zeros(&self) -> u32 {
        self.digest.0.trailing_zeros()
    }

    fn mix_felts(&mut self, felts: &[SecureField]) {
        // Convert secure fields to M31 values and mix them
        let mut values = vec![self.digest];
        for felt in felts {
            values.extend_from_slice(&felt.to_m31_array());
        }

        self.update_digest(Poseidon31MerkleHasher::hash_many(&values));
    }

    fn mix_u32s(&mut self, data: &[u32]) {
        // Convert u32s to M31 values and mix them
        let mut values = vec![self.digest];
        values.extend(data.iter().map(|&x| M31::from(x)));

        self.update_digest(Poseidon31MerkleHasher::hash_many(&values));
    }

    fn mix_u64(&mut self, value: u64) {
        // Split u64 into two M31 values
        let low = M31::from((value & 0xFFFFFFFF) as u32);
        let high = M31::from(((value >> 32) & 0xFFFFFFFF) as u32);
        let new_digest = Poseidon31MerkleHasher::hash_many(&[self.digest, low, high]);
        self.update_digest(new_digest);
    }

    fn draw_secure_felt(&mut self) -> SecureField {
        let felts: [BaseField; 8] = self.draw_base_felts();
        SecureField::from_m31_array(felts[..SECURE_EXTENSION_DEGREE].try_into().unwrap())
    }

    fn draw_secure_felts(&mut self, n_felts: usize) -> Vec<SecureField> {
        let mut secure_felts = Vec::with_capacity(n_felts);
        for _ in 0..n_felts {
            secure_felts.push(self.draw_secure_felt());
        }
        secure_felts
    }

    fn draw_random_bytes(&mut self) -> Vec<u8> {
        let m31_value = self.draw_m31();
        m31_value.0.to_le_bytes().to_vec()
    }
}

#[derive(Default)]
pub struct Poseidon31MerkleChannel;

impl MerkleChannel for Poseidon31MerkleChannel {
    type C = Poseidon31Channel;
    type H = Poseidon31MerkleHasher;

    fn mix_root(channel: &mut Self::C, root: <Self::H as MerkleHasher>::Hash) {
        let new_digest = Poseidon31MerkleHasher::hash_pair(channel.digest(), root.0);
        channel.update_digest(new_digest);
    }
}

// Implement ColumnOps for M31Hash so we can use it with MerkleOps
impl ColumnOps<M31Hash> for SimdBackend {
    type Column = Vec<M31Hash>;

    fn bit_reverse_column(_column: &mut Self::Column) {
        unimplemented!("bit_reverse_column not needed for Merkle operations")
    }
}

// Implement MerkleOps for Poseidon31MerkleHasher
impl MerkleOps<Poseidon31MerkleHasher> for SimdBackend {
    fn commit_on_layer(
        log_size: u32,
        prev_layer: Option<&Vec<M31Hash>>,
        columns: &[&Col<Self, BaseField>],
    ) -> Vec<M31Hash> {
        let iter = (0..(1 << log_size)).into_par_iter();

        iter.map(|i| {
            Poseidon31MerkleHasher::hash_node(
                prev_layer.map(|prev_layer| (prev_layer[2 * i], prev_layer[2 * i + 1])),
                &columns.iter().map(|column| column.at(i)).collect_vec(),
            )
        })
        .collect()
    }
}

// Implement GrindOps for Poseidon31Channel
impl GrindOps<Poseidon31Channel> for SimdBackend {
    fn grind(channel: &Poseidon31Channel, pow_bits: u32) -> u64 {
        // Simple sequential implementation for proof of work
        let mut nonce = 0u64;
        loop {
            let mut test_channel = channel.clone();
            test_channel.mix_u64(nonce);
            if test_channel.trailing_zeros() >= pow_bits {
                return nonce;
            }
            nonce += 1;
            if nonce == u64::MAX {
                panic!("Failed to find proof of work");
            }
        }
    }
}

// Finally, implement BackendForChannel
impl BackendForChannel<Poseidon31MerkleChannel> for SimdBackend {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon31_hash_pair() {
        let left = M31::from(42);
        let right = M31::from(123);
        let hash1 = Poseidon31MerkleHasher::hash_pair(left, right);
        let hash2 = Poseidon31MerkleHasher::hash_pair(left, right);

        // Same inputs should produce same hash
        assert_eq!(hash1, hash2);

        // Different inputs should produce different hash
        let hash3 = Poseidon31MerkleHasher::hash_pair(right, left);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_node() {
        // Test with no children, just column values
        let values = vec![M31::from(1), M31::from(2), M31::from(3)];
        let hash1 = Poseidon31MerkleHasher::hash_node(None, &values);

        // Test with children
        let left_child = M31Hash(M31::from(10));
        let right_child = M31Hash(M31::from(20));
        let hash2 = Poseidon31MerkleHasher::hash_node(Some((left_child, right_child)), &values);

        // Different inputs should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_channel_basic() {
        let mut channel = Poseidon31Channel::default();

        // Test initial state
        assert_eq!(channel.digest(), M31::zero());
        assert_eq!(channel.channel_time_n_challenges, 0);
        assert_eq!(channel.channel_time_n_sent, 0);

        // Test drawing values
        let val1 = channel.draw_secure_felt();
        let val2 = channel.draw_secure_felt();
        assert_ne!(val1, val2);

        // Test mixing
        channel.mix_u64(0x123456789ABCDEF0);
        let digest_after_mix = channel.digest();
        assert_ne!(digest_after_mix, M31::zero());
    }
}
