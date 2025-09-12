#![feature(iter_advance_by)]
#![feature(raw_slice_split)]
#![feature(portable_simd)]
#![feature(iter_array_chunks)]

pub mod components;
pub mod debug_tools;
pub mod errors;
pub mod hints;
pub mod poseidon31_merkle;
pub mod preprocessed;
pub mod prover;
pub mod prover_config;
pub mod public_data;
pub mod relations;
pub mod utils;
pub mod verifier;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
pub use poseidon31_merkle::{Poseidon31Channel, Poseidon31MerkleChannel, Poseidon31MerkleHasher};

use serde::{Deserialize, Serialize};
use stwo_prover::core::prover::StarkProof;
use stwo_prover::core::vcs::ops::MerkleHasher;

use crate::components::{Claim, InteractionClaim};
use crate::public_data::PublicData;

#[derive(Serialize, Deserialize)]
pub struct Proof<H: MerkleHasher> {
    pub claim: Claim,
    pub interaction_claim: InteractionClaim,
    pub public_data: PublicData,
    pub stark_proof: StarkProof<H>,
    pub interaction_pow: u64,
}

impl<H> Proof<H>
where
    H: MerkleHasher + for<'de> Deserialize<'de>,
{
    /// Load a proof from a JSON file at the given path
    pub fn from_json_file(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let json_content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            format!(
                "Failed to read proof file '{}': {}",
                path.as_ref().display(),
                e
            )
        })?;

        serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse proof JSON: {}", e))
    }
}
