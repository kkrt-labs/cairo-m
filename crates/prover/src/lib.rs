#![feature(iter_advance_by)]
#![feature(raw_slice_split)]
#![feature(portable_simd)]
#![feature(iter_array_chunks)]

pub mod adapter;
pub mod components;
pub mod errors;
pub mod preprocessed;
pub mod prover;
pub mod relations;
pub mod verifier;

use serde::{Deserialize, Serialize};
use stwo_prover::core::prover::StarkProof;
use stwo_prover::core::vcs::ops::MerkleHasher;

use crate::components::{Claim, InteractionClaim};

#[derive(Serialize, Deserialize)]
pub struct Proof<H: MerkleHasher> {
    pub claim: Claim,
    pub interaction_claim: InteractionClaim,
    pub stark_proof: StarkProof<H>,
    pub interaction_pow: u64,
}
