use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::{backend::simd::SimdBackend, pcs::TreeVec};

use cairo_m_prover::poseidon2::T;

pub mod channel;
pub mod decommitments;

use cairo_m_prover::Proof;
pub use channel::{hints, ChannelHints};

use crate::{Poseidon31MerkleChannel, Poseidon31MerkleHasher};

pub type HashInput = [M31; T];

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ProverInput {
    pub poseidon2_inputs: Vec<HashInput>,
}

// TODO: remove this once we have a generic MerkleChannel trait
type MC = Poseidon31MerkleChannel;
type H = Poseidon31MerkleHasher;

/// Generate all hints for verification of a proof
pub fn generate_hints(proof: &Proof<H>) -> ChannelHints
where
    SimdBackend: BackendForChannel<MC>,
{
    // ╔══════════════════════════════════════════════════════════════════════════╗
    // ║                              Channel Hints                               ║
    // ╚══════════════════════════════════════════════════════════════════════════╝
    let channel_hints = channel::hints::<MC>(proof);

    // ╔══════════════════════════════════════════════════════════════════════════╗
    // ║                              Decommitments                               ║
    // ╚══════════════════════════════════════════════════════════════════════════╝
    let column_log_sizes: TreeVec<Vec<u32>> = get_column_log_sizes(&proof.claim.log_sizes());

    // NOTE: decommitment hints are Poseidon31MerkleHasher specific
    let _decommitment_hints =
        decommitments::hints(proof, &channel_hints.queries, &column_log_sizes)
            .expect("Failed to generate decommitment hints");

    channel_hints
}

/// Get the column log sizes for the decommitment hints (unextended)
fn get_column_log_sizes(log_sizes: &TreeVec<Vec<u32>>) -> TreeVec<Vec<u32>> {
    let execution_trace_log_size = log_sizes[1].clone();
    // TOOD: remove hardcoded max constraint log degree bound
    let composition_log_size = execution_trace_log_size.iter().max().unwrap() + 1;
    TreeVec::new(vec![
        // TODO: remove hardcoded preprocessed trace log_size
        vec![20],
        execution_trace_log_size,
        log_sizes[2].clone(),
        vec![composition_log_size; SECURE_EXTENSION_DEGREE],
    ])
}
