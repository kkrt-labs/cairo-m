use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::MerkleChannel;
use stwo_prover::core::fields::m31::M31;

use cairo_m_prover::poseidon2::T;

pub mod channel;

use cairo_m_prover::Proof;
pub use channel::{hints, ChannelHints};

pub type HashInput = [M31; T];

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ProverInput {
    pub poseidon2_inputs: Vec<HashInput>,
}

/// Generate all hints from a proof by rebuilding the channel transcript
/// This function is generic over any MerkleChannel implementation
pub fn generate_hints<MC: MerkleChannel>(proof: &Proof<MC::H>) -> ChannelHints
where
    SimdBackend: BackendForChannel<MC>,
{
    channel::hints::<MC>(proof)
}
