pub mod debug_tools;
pub mod prover_sha256;
#[cfg(test)]
mod tests;
pub mod verifier_sha256;

use num_traits::Zero;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_prover::core::air::{Component as ComponentVerifier, ComponentProver};
pub use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::prover::StarkProof;
use stwo_prover::core::vcs::ops::MerkleHasher;

#[derive(Serialize, Deserialize)]
pub struct Proof<H: MerkleHasher> {
    /// Claim about the execution trace (log sizes for each component)
    pub claim: Claim,
    /// Claim about interaction trace (claimed sums for each component)
    pub interaction_claim: InteractionClaim,
    /// The underlying STARK proof containing polynomial commitments and evaluations
    pub stark_proof: StarkProof<H>,
    /// Proof-of-work nonce
    pub interaction_pow: u64,
}

use crate::adapter::SHA256HashInput;
use crate::components::{sha256, Relations};
use crate::preprocessed::ch_maj::{
    ch_h0, ch_h1, ch_h2, ch_l0, ch_l1, ch_l2, maj_h0, maj_h1, maj_h2, maj_l0, maj_l1, maj_l2,
};
use crate::preprocessed::range_check::range_check_16;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub sha256: sha256::Claim,
    pub ch_l0: ch_l0::Claim,
    pub ch_l1: ch_l1::Claim,
    pub ch_l2: ch_l2::Claim,
    pub ch_h0: ch_h0::Claim,
    pub ch_h1: ch_h1::Claim,
    pub ch_h2: ch_h2::Claim,
    pub maj_l0: maj_l0::Claim,
    pub maj_l1: maj_l1::Claim,
    pub maj_l2: maj_l2::Claim,
    pub maj_h0: maj_h0::Claim,
    pub maj_h1: maj_h1::Claim,
    pub maj_h2: maj_h2::Claim,
    pub range_check_16: range_check_16::Claim,
}

pub struct InteractionClaimData {
    pub sha256: sha256::InteractionClaimData,
    pub ch_l0: ch_l0::InteractionClaimData,
    pub ch_l1: ch_l1::InteractionClaimData,
    pub ch_l2: ch_l2::InteractionClaimData,
    pub ch_h0: ch_h0::InteractionClaimData,
    pub ch_h1: ch_h1::InteractionClaimData,
    pub ch_h2: ch_h2::InteractionClaimData,
    pub maj_l0: maj_l0::InteractionClaimData,
    pub maj_l1: maj_l1::InteractionClaimData,
    pub maj_l2: maj_l2::InteractionClaimData,
    pub maj_h0: maj_h0::InteractionClaimData,
    pub maj_h1: maj_h1::InteractionClaimData,
    pub maj_h2: maj_h2::InteractionClaimData,
    pub range_check_16: range_check_16::InteractionClaimData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub sha256: sha256::InteractionClaim,
    pub ch_l0: ch_l0::InteractionClaim,
    pub ch_l1: ch_l1::InteractionClaim,
    pub ch_l2: ch_l2::InteractionClaim,
    pub ch_h0: ch_h0::InteractionClaim,
    pub ch_h1: ch_h1::InteractionClaim,
    pub ch_h2: ch_h2::InteractionClaim,
    pub maj_l0: maj_l0::InteractionClaim,
    pub maj_l1: maj_l1::InteractionClaim,
    pub maj_l2: maj_l2::InteractionClaim,
    pub maj_h0: maj_h0::InteractionClaim,
    pub maj_h1: maj_h1::InteractionClaim,
    pub maj_h2: maj_h2::InteractionClaim,
    pub range_check_16: range_check_16::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.sha256.log_sizes(),
            self.ch_l0.log_sizes(),
            self.ch_l1.log_sizes(),
            self.ch_l2.log_sizes(),
            self.ch_h0.log_sizes(),
            self.ch_h1.log_sizes(),
            self.ch_h2.log_sizes(),
            self.maj_l0.log_sizes(),
            self.maj_l1.log_sizes(),
            self.maj_l2.log_sizes(),
            self.maj_h0.log_sizes(),
            self.maj_h1.log_sizes(),
            self.maj_h2.log_sizes(),
            self.range_check_16.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.sha256.mix_into(channel);
        self.ch_l0.mix_into(channel);
        self.ch_l1.mix_into(channel);
        self.ch_l2.mix_into(channel);
        self.ch_h0.mix_into(channel);
        self.ch_h1.mix_into(channel);
        self.ch_h2.mix_into(channel);
        self.maj_l0.mix_into(channel);
        self.maj_l1.mix_into(channel);
        self.maj_l2.mix_into(channel);
        self.maj_h0.mix_into(channel);
        self.maj_h1.mix_into(channel);
        self.maj_h2.mix_into(channel);
        self.range_check_16.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        inputs: &Vec<SHA256HashInput>,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Write sha256 trace
        let (sha256_claim, sha256_trace, sha256_interaction_claim_data) =
            sha256::Claim::write_trace(inputs);

        // Write ch trace
        let (ch_l0_claim, ch_l0_trace, ch_l0_interaction_claim_data) = ch_l0::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch_l0
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (ch_l1_claim, ch_l1_trace, ch_l1_interaction_claim_data) = ch_l1::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch_l1
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (ch_l2_claim, ch_l2_trace, ch_l2_interaction_claim_data) = ch_l2::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch_l2
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (ch_h0_claim, ch_h0_trace, ch_h0_interaction_claim_data) = ch_h0::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch_h0
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (ch_h1_claim, ch_h1_trace, ch_h1_interaction_claim_data) = ch_h1::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch_h1
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (ch_h2_claim, ch_h2_trace, ch_h2_interaction_claim_data) = ch_h2::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch_h2
                .par_iter()
                .map(|v| v.as_slice()),
        );

        // Write maj trace
        let (maj_l0_claim, maj_l0_trace, maj_l0_interaction_claim_data) =
            maj_l0::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .maj_l0
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (maj_l1_claim, maj_l1_trace, maj_l1_interaction_claim_data) =
            maj_l1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .maj_l1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (maj_l2_claim, maj_l2_trace, maj_l2_interaction_claim_data) =
            maj_l2::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .maj_l2
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (maj_h0_claim, maj_h0_trace, maj_h0_interaction_claim_data) =
            maj_h0::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .maj_h0
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (maj_h1_claim, maj_h1_trace, maj_h1_interaction_claim_data) =
            maj_h1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .maj_h1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (maj_h2_claim, maj_h2_trace, maj_h2_interaction_claim_data) =
            maj_h2::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .maj_h2
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        // Write range_check components
        let range_check_16_data = sha256_interaction_claim_data
            .lookup_data
            .range_check_16
            .par_iter()
            .flat_map(|vec| vec.par_iter().map(|arr| &arr[0]));
        let (range_check_16_claim, range_check_16_trace, range_check_16_interaction_claim_data) =
            range_check_16::Claim::write_trace(range_check_16_data);

        // Gather all lookup data
        let interaction_claim_data = InteractionClaimData {
            sha256: sha256_interaction_claim_data,
            ch_l0: ch_l0_interaction_claim_data,
            ch_l1: ch_l1_interaction_claim_data,
            ch_l2: ch_l2_interaction_claim_data,
            ch_h0: ch_h0_interaction_claim_data,
            ch_h1: ch_h1_interaction_claim_data,
            ch_h2: ch_h2_interaction_claim_data,
            maj_l0: maj_l0_interaction_claim_data,
            maj_l1: maj_l1_interaction_claim_data,
            maj_l2: maj_l2_interaction_claim_data,
            maj_h0: maj_h0_interaction_claim_data,
            maj_h1: maj_h1_interaction_claim_data,
            maj_h2: maj_h2_interaction_claim_data,
            range_check_16: range_check_16_interaction_claim_data,
        };

        // Combine all traces
        let trace = sha256_trace
            .to_evals()
            .into_iter()
            .chain(ch_l0_trace)
            .chain(ch_l1_trace)
            .chain(ch_l2_trace)
            .chain(ch_h0_trace)
            .chain(ch_h1_trace)
            .chain(ch_h2_trace)
            .chain(maj_l0_trace)
            .chain(maj_l1_trace)
            .chain(maj_l2_trace)
            .chain(maj_h0_trace)
            .chain(maj_h1_trace)
            .chain(maj_h2_trace)
            .chain(range_check_16_trace);

        (
            Self {
                sha256: sha256_claim,
                ch_l0: ch_l0_claim,
                ch_l1: ch_l1_claim,
                ch_l2: ch_l2_claim,
                ch_h0: ch_h0_claim,
                ch_h1: ch_h1_claim,
                ch_h2: ch_h2_claim,
                maj_l0: maj_l0_claim,
                maj_l1: maj_l1_claim,
                maj_l2: maj_l2_claim,
                maj_h0: maj_h0_claim,
                maj_h1: maj_h1_claim,
                maj_h2: maj_h2_claim,
                range_check_16: range_check_16_claim,
            },
            trace,
            interaction_claim_data,
        )
    }
}

impl InteractionClaim {
    pub fn write_interaction_trace(
        relations: &Relations,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        Self,
    ) {
        let (sha256_interaction_claim, sha256_interaction_trace) =
            sha256::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.sha256,
            );

        let (ch_l0_interaction_claim, ch_l0_interaction_trace) =
            ch_l0::InteractionClaim::write_interaction_trace(
                &relations.ch_l0,
                &interaction_claim_data.ch_l0,
            );
        let (ch_l1_interaction_claim, ch_l1_interaction_trace) =
            ch_l1::InteractionClaim::write_interaction_trace(
                &relations.ch_l1,
                &interaction_claim_data.ch_l1,
            );
        let (ch_l2_interaction_claim, ch_l2_interaction_trace) =
            ch_l2::InteractionClaim::write_interaction_trace(
                &relations.ch_l2,
                &interaction_claim_data.ch_l2,
            );
        let (ch_h0_interaction_claim, ch_h0_interaction_trace) =
            ch_h0::InteractionClaim::write_interaction_trace(
                &relations.ch_h0,
                &interaction_claim_data.ch_h0,
            );
        let (ch_h1_interaction_claim, ch_h1_interaction_trace) =
            ch_h1::InteractionClaim::write_interaction_trace(
                &relations.ch_h1,
                &interaction_claim_data.ch_h1,
            );
        let (ch_h2_interaction_claim, ch_h2_interaction_trace) =
            ch_h2::InteractionClaim::write_interaction_trace(
                &relations.ch_h2,
                &interaction_claim_data.ch_h2,
            );

        let (maj_l0_interaction_claim, maj_l0_interaction_trace) =
            maj_l0::InteractionClaim::write_interaction_trace(
                &relations.maj_l0,
                &interaction_claim_data.maj_l0,
            );
        let (maj_l1_interaction_claim, maj_l1_interaction_trace) =
            maj_l1::InteractionClaim::write_interaction_trace(
                &relations.maj_l1,
                &interaction_claim_data.maj_l1,
            );
        let (maj_l2_interaction_claim, maj_l2_interaction_trace) =
            maj_l2::InteractionClaim::write_interaction_trace(
                &relations.maj_l2,
                &interaction_claim_data.maj_l2,
            );
        let (maj_h0_interaction_claim, maj_h0_interaction_trace) =
            maj_h0::InteractionClaim::write_interaction_trace(
                &relations.maj_h0,
                &interaction_claim_data.maj_h0,
            );
        let (maj_h1_interaction_claim, maj_h1_interaction_trace) =
            maj_h1::InteractionClaim::write_interaction_trace(
                &relations.maj_h1,
                &interaction_claim_data.maj_h1,
            );
        let (maj_h2_interaction_claim, maj_h2_interaction_trace) =
            maj_h2::InteractionClaim::write_interaction_trace(
                &relations.maj_h2,
                &interaction_claim_data.maj_h2,
            );

        let (range_check_16_interaction_claim, range_check_16_interaction_trace) =
            range_check_16::InteractionClaim::write_interaction_trace(
                &relations.range_check_16,
                &interaction_claim_data.range_check_16,
            );

        let trace = sha256_interaction_trace
            .into_iter()
            .chain(ch_l0_interaction_trace)
            .chain(ch_l1_interaction_trace)
            .chain(ch_l2_interaction_trace)
            .chain(ch_h0_interaction_trace)
            .chain(ch_h1_interaction_trace)
            .chain(ch_h2_interaction_trace)
            .chain(maj_l0_interaction_trace)
            .chain(maj_l1_interaction_trace)
            .chain(maj_l2_interaction_trace)
            .chain(maj_h0_interaction_trace)
            .chain(maj_h1_interaction_trace)
            .chain(maj_h2_interaction_trace)
            .chain(range_check_16_interaction_trace);
        (
            trace,
            Self {
                sha256: sha256_interaction_claim,
                ch_l0: ch_l0_interaction_claim,
                ch_l1: ch_l1_interaction_claim,
                ch_l2: ch_l2_interaction_claim,
                ch_h0: ch_h0_interaction_claim,
                ch_h1: ch_h1_interaction_claim,
                ch_h2: ch_h2_interaction_claim,
                maj_l0: maj_l0_interaction_claim,
                maj_l1: maj_l1_interaction_claim,
                maj_l2: maj_l2_interaction_claim,
                maj_h0: maj_h0_interaction_claim,
                maj_h1: maj_h1_interaction_claim,
                maj_h2: maj_h2_interaction_claim,
                range_check_16: range_check_16_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self) -> SecureField {
        let mut sum = SecureField::zero();
        sum += self.sha256.claimed_sum;
        sum += self.ch_l0.claimed_sum;
        sum += self.ch_l1.claimed_sum;
        sum += self.ch_l2.claimed_sum;
        sum += self.ch_h0.claimed_sum;
        sum += self.ch_h1.claimed_sum;
        sum += self.ch_h2.claimed_sum;
        sum += self.maj_l0.claimed_sum;
        sum += self.maj_l1.claimed_sum;
        sum += self.maj_l2.claimed_sum;
        sum += self.maj_h0.claimed_sum;
        sum += self.maj_h1.claimed_sum;
        sum += self.maj_h2.claimed_sum;
        sum += self.range_check_16.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.sha256.mix_into(channel);
        self.ch_l0.mix_into(channel);
        self.ch_l1.mix_into(channel);
        self.ch_l2.mix_into(channel);
        self.ch_h0.mix_into(channel);
        self.ch_h1.mix_into(channel);
        self.ch_h2.mix_into(channel);
        self.maj_l0.mix_into(channel);
        self.maj_l1.mix_into(channel);
        self.maj_l2.mix_into(channel);
        self.maj_h0.mix_into(channel);
        self.maj_h1.mix_into(channel);
        self.maj_h2.mix_into(channel);
        self.range_check_16.mix_into(channel);
    }
}

pub struct Components {
    pub sha256: sha256::Component,
    pub ch_l0: ch_l0::Component,
    pub ch_l1: ch_l1::Component,
    pub ch_l2: ch_l2::Component,
    pub ch_h0: ch_h0::Component,
    pub ch_h1: ch_h1::Component,
    pub ch_h2: ch_h2::Component,
    pub maj_l0: maj_l0::Component,
    pub maj_l1: maj_l1::Component,
    pub maj_l2: maj_l2::Component,
    pub maj_h0: maj_h0::Component,
    pub maj_h1: maj_h1::Component,
    pub maj_h2: maj_h2::Component,
    pub range_check_16: range_check_16::Component,
}

impl Components {
    pub fn new(
        location_allocator: &mut TraceLocationAllocator,
        claim: &Claim,
        interaction_claim: &InteractionClaim,
        relations: &Relations,
    ) -> Self {
        Self {
            sha256: sha256::Component::new(
                location_allocator,
                sha256::Eval {
                    claim: claim.sha256.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.sha256.claimed_sum,
            ),
            ch_l0: ch_l0::Component::new(
                location_allocator,
                ch_l0::Eval {
                    claim: claim.ch_l0,
                    relation: relations.ch_l0.clone(),
                },
                interaction_claim.ch_l0.claimed_sum,
            ),
            ch_l1: ch_l1::Component::new(
                location_allocator,
                ch_l1::Eval {
                    claim: claim.ch_l1,
                    relation: relations.ch_l1.clone(),
                },
                interaction_claim.ch_l1.claimed_sum,
            ),
            ch_l2: ch_l2::Component::new(
                location_allocator,
                ch_l2::Eval {
                    claim: claim.ch_l2,
                    relation: relations.ch_l2.clone(),
                },
                interaction_claim.ch_l2.claimed_sum,
            ),
            ch_h0: ch_h0::Component::new(
                location_allocator,
                ch_h0::Eval {
                    claim: claim.ch_h0,
                    relation: relations.ch_h0.clone(),
                },
                interaction_claim.ch_h0.claimed_sum,
            ),
            ch_h1: ch_h1::Component::new(
                location_allocator,
                ch_h1::Eval {
                    claim: claim.ch_h1,
                    relation: relations.ch_h1.clone(),
                },
                interaction_claim.ch_h1.claimed_sum,
            ),
            ch_h2: ch_h2::Component::new(
                location_allocator,
                ch_h2::Eval {
                    claim: claim.ch_h2,
                    relation: relations.ch_h2.clone(),
                },
                interaction_claim.ch_h2.claimed_sum,
            ),
            maj_l0: maj_l0::Component::new(
                location_allocator,
                maj_l0::Eval {
                    claim: claim.maj_l0,
                    relation: relations.maj_l0.clone(),
                },
                interaction_claim.maj_l0.claimed_sum,
            ),
            maj_l1: maj_l1::Component::new(
                location_allocator,
                maj_l1::Eval {
                    claim: claim.maj_l1,
                    relation: relations.maj_l1.clone(),
                },
                interaction_claim.maj_l1.claimed_sum,
            ),
            maj_l2: maj_l2::Component::new(
                location_allocator,
                maj_l2::Eval {
                    claim: claim.maj_l2,
                    relation: relations.maj_l2.clone(),
                },
                interaction_claim.maj_l2.claimed_sum,
            ),
            maj_h0: maj_h0::Component::new(
                location_allocator,
                maj_h0::Eval {
                    claim: claim.maj_h0,
                    relation: relations.maj_h0.clone(),
                },
                interaction_claim.maj_h0.claimed_sum,
            ),
            maj_h1: maj_h1::Component::new(
                location_allocator,
                maj_h1::Eval {
                    claim: claim.maj_h1,
                    relation: relations.maj_h1.clone(),
                },
                interaction_claim.maj_h1.claimed_sum,
            ),
            maj_h2: maj_h2::Component::new(
                location_allocator,
                maj_h2::Eval {
                    claim: claim.maj_h2,
                    relation: relations.maj_h2.clone(),
                },
                interaction_claim.maj_h2.claimed_sum,
            ),
            range_check_16: range_check_16::Component::new(
                location_allocator,
                range_check_16::Eval {
                    claim: claim.range_check_16,
                    relation: relations.range_check_16.clone(),
                },
                interaction_claim.range_check_16.claimed_sum,
            ),
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        vec![
            &self.sha256,
            &self.ch_l0,
            &self.ch_l1,
            &self.ch_l2,
            &self.ch_h0,
            &self.ch_h1,
            &self.ch_h2,
            &self.maj_l0,
            &self.maj_l1,
            &self.maj_l2,
            &self.maj_h0,
            &self.maj_h1,
            &self.maj_h2,
            &self.range_check_16,
        ]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![
            &self.sha256,
            &self.ch_l0,
            &self.ch_l1,
            &self.ch_l2,
            &self.ch_h0,
            &self.ch_h1,
            &self.ch_h2,
            &self.maj_l0,
            &self.maj_l1,
            &self.maj_l2,
            &self.maj_h0,
            &self.maj_h1,
            &self.maj_h2,
            &self.range_check_16,
        ]
    }
}
