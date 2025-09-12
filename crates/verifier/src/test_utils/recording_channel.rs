use serde::{Deserialize, Serialize};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Col};
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::BaseField;
use stwo_prover::core::fields::qm31::{SecureField, QM31};
use stwo_prover::core::proof_of_work::GrindOps;
use stwo_prover::core::vcs::ops::{MerkleHasher, MerkleOps};

use crate::poseidon31_merkle::{
    M31Hash, Poseidon31Channel, Poseidon31MerkleChannel, Poseidon31MerkleHasher,
};

/// Recording version of Poseidon31MerkleHasher - just delegates to the original
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct RecordingPoseidon31MerkleHasher;

impl MerkleHasher for RecordingPoseidon31MerkleHasher {
    type Hash = M31Hash;

    fn hash_node(
        children_hashes: Option<(Self::Hash, Self::Hash)>,
        column_values: &[BaseField],
    ) -> Self::Hash {
        // Delegate to the original Poseidon31MerkleHasher
        Poseidon31MerkleHasher::hash_node(children_hashes, column_values)
    }
}

/// A wrapper around Poseidon31Channel that records all channel operations
#[derive(Clone, Debug, Default)]
pub struct RecordingPoseidon31Channel {
    /// The underlying channel
    pub channel: Poseidon31Channel,
    /// Log of returned values
    pub log: Vec<QM31>,
}

impl Channel for RecordingPoseidon31Channel {
    const BYTES_PER_HASH: usize = Poseidon31Channel::BYTES_PER_HASH;

    fn trailing_zeros(&self) -> u32 {
        self.channel.trailing_zeros()
    }

    fn mix_felts(&mut self, felts: &[SecureField]) {
        self.channel.mix_felts(felts);
    }

    fn mix_u32s(&mut self, data: &[u32]) {
        self.channel.mix_u32s(data);
    }

    fn mix_u64(&mut self, value: u64) {
        self.channel.mix_u64(value);
    }

    fn draw_secure_felt(&mut self) -> SecureField {
        let value = self.channel.draw_secure_felt();
        self.log.push(value);
        value
    }

    fn draw_secure_felts(&mut self, n_felts: usize) -> Vec<SecureField> {
        let values = self.channel.draw_secure_felts(n_felts);
        self.log.extend(values.iter());
        values
    }

    fn draw_random_bytes(&mut self) -> Vec<u8> {
        let values = self.channel.draw_random_bytes();
        self.log.extend(
            values
                .iter()
                .map(|value| QM31::from_u32_unchecked(*value as u32, 0, 0, 0)),
        );
        values
    }
}

/// Recording MerkleChannel implementation
#[derive(Default)]
pub struct RecordingPoseidon31MerkleChannel;

impl MerkleChannel for RecordingPoseidon31MerkleChannel {
    type C = RecordingPoseidon31Channel;
    type H = RecordingPoseidon31MerkleHasher;

    fn mix_root(channel: &mut Self::C, root: <Self::H as MerkleHasher>::Hash) {
        // Delegate to the inner channel
        Poseidon31MerkleChannel::mix_root(&mut channel.channel, root);
    }
}

// Implement GrindOps for RecordingPoseidon31Channel by delegating to the inner channel
impl GrindOps<RecordingPoseidon31Channel> for SimdBackend {
    fn grind(channel: &RecordingPoseidon31Channel, pow_bits: u32) -> u64 {
        // Delegate to the inner Poseidon31Channel
        <Self as GrindOps<Poseidon31Channel>>::grind(&channel.channel, pow_bits)
    }
}

// Implement MerkleOps for RecordingPoseidon31MerkleHasher - delegate to original
impl MerkleOps<RecordingPoseidon31MerkleHasher> for SimdBackend {
    fn commit_on_layer(
        log_size: u32,
        prev_layer: Option<&Vec<M31Hash>>,
        columns: &[&Col<Self, BaseField>],
    ) -> Vec<M31Hash> {
        // Delegate to the original Poseidon31MerkleHasher
        <Self as MerkleOps<Poseidon31MerkleHasher>>::commit_on_layer(log_size, prev_layer, columns)
    }
}

// Implement BackendForChannel
impl BackendForChannel<RecordingPoseidon31MerkleChannel> for SimdBackend {}
