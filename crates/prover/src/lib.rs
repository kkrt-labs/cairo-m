#![feature(raw_slice_split)]

pub mod components;
pub mod errors;
pub mod preprocessed;
pub mod prover;
pub mod relations;
pub mod verifier;

use stwo_prover::core::prover::StarkProof;
use stwo_prover::core::vcs::ops::MerkleHasher;

use crate::components::{Claim, InteractionClaim};

pub struct Proof<const N: usize, H: MerkleHasher> {
    pub claim: Claim<N>,
    pub interaction_claim: InteractionClaim<N>,
    pub stark_proof: StarkProof<H>,
    pub interaction_pow: u64,
}
