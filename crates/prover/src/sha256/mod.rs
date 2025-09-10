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
use crate::preprocessed::ch_maj::{ch, maj};
use crate::preprocessed::range_check::range_check_16;
use crate::preprocessed::sigma::{
    big_sigma0_0, big_sigma0_1, big_sigma1_0, big_sigma1_1, small_sigma0_0, small_sigma0_1,
    small_sigma1_0, small_sigma1_1,
};
use crate::preprocessed::xor::{
    xor_big_sigma0_0, xor_big_sigma0_1, xor_big_sigma1, xor_small_sigma0, xor_small_sigma1,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub sha256: sha256::Claim,
    pub ch: ch::Claim,
    pub maj: maj::Claim,
    pub small_sigma0_0: small_sigma0_0::Claim,
    pub small_sigma0_1: small_sigma0_1::Claim,
    pub small_sigma1_0: small_sigma1_0::Claim,
    pub small_sigma1_1: small_sigma1_1::Claim,
    pub big_sigma0_0: big_sigma0_0::Claim,
    pub big_sigma0_1: big_sigma0_1::Claim,
    pub big_sigma1_0: big_sigma1_0::Claim,
    pub big_sigma1_1: big_sigma1_1::Claim,
    pub xor_small_sigma0: xor_small_sigma0::Claim,
    pub xor_small_sigma1: xor_small_sigma1::Claim,
    pub xor_big_sigma0_0: xor_big_sigma0_0::Claim,
    pub xor_big_sigma0_1: xor_big_sigma0_1::Claim,
    pub xor_big_sigma1: xor_big_sigma1::Claim,
    pub range_check_16: range_check_16::Claim,
}

pub struct InteractionClaimData {
    pub sha256: sha256::InteractionClaimData,
    pub ch: ch::InteractionClaimData,
    pub maj: maj::InteractionClaimData,
    pub small_sigma0_0: small_sigma0_0::InteractionClaimData,
    pub small_sigma0_1: small_sigma0_1::InteractionClaimData,
    pub small_sigma1_0: small_sigma1_0::InteractionClaimData,
    pub small_sigma1_1: small_sigma1_1::InteractionClaimData,
    pub big_sigma0_0: big_sigma0_0::InteractionClaimData,
    pub big_sigma0_1: big_sigma0_1::InteractionClaimData,
    pub big_sigma1_0: big_sigma1_0::InteractionClaimData,
    pub big_sigma1_1: big_sigma1_1::InteractionClaimData,
    pub xor_small_sigma0: xor_small_sigma0::InteractionClaimData,
    pub xor_small_sigma1: xor_small_sigma1::InteractionClaimData,
    pub xor_big_sigma0_0: xor_big_sigma0_0::InteractionClaimData,
    pub xor_big_sigma0_1: xor_big_sigma0_1::InteractionClaimData,
    pub xor_big_sigma1: xor_big_sigma1::InteractionClaimData,
    pub range_check_16: range_check_16::InteractionClaimData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub sha256: sha256::InteractionClaim,
    pub ch: ch::InteractionClaim,
    pub maj: maj::InteractionClaim,
    pub small_sigma0_0: small_sigma0_0::InteractionClaim,
    pub small_sigma0_1: small_sigma0_1::InteractionClaim,
    pub small_sigma1_0: small_sigma1_0::InteractionClaim,
    pub small_sigma1_1: small_sigma1_1::InteractionClaim,
    pub big_sigma0_0: big_sigma0_0::InteractionClaim,
    pub big_sigma0_1: big_sigma0_1::InteractionClaim,
    pub big_sigma1_0: big_sigma1_0::InteractionClaim,
    pub big_sigma1_1: big_sigma1_1::InteractionClaim,
    pub xor_small_sigma0: xor_small_sigma0::InteractionClaim,
    pub xor_small_sigma1: xor_small_sigma1::InteractionClaim,
    pub xor_big_sigma0_0: xor_big_sigma0_0::InteractionClaim,
    pub xor_big_sigma0_1: xor_big_sigma0_1::InteractionClaim,
    pub xor_big_sigma1: xor_big_sigma1::InteractionClaim,
    pub range_check_16: range_check_16::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.sha256.log_sizes(),
            self.ch.log_sizes(),
            self.maj.log_sizes(),
            self.small_sigma0_0.log_sizes(),
            self.small_sigma0_1.log_sizes(),
            self.small_sigma1_0.log_sizes(),
            self.small_sigma1_1.log_sizes(),
            self.big_sigma0_0.log_sizes(),
            self.big_sigma0_1.log_sizes(),
            self.big_sigma1_0.log_sizes(),
            self.big_sigma1_1.log_sizes(),
            self.xor_small_sigma0.log_sizes(),
            self.xor_small_sigma1.log_sizes(),
            self.xor_big_sigma0_0.log_sizes(),
            self.xor_big_sigma0_1.log_sizes(),
            self.xor_big_sigma1.log_sizes(),
            self.range_check_16.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.sha256.mix_into(channel);
        self.ch.mix_into(channel);
        self.maj.mix_into(channel);
        self.small_sigma0_0.mix_into(channel);
        self.small_sigma0_1.mix_into(channel);
        self.small_sigma1_0.mix_into(channel);
        self.small_sigma1_1.mix_into(channel);
        self.big_sigma0_0.mix_into(channel);
        self.big_sigma0_1.mix_into(channel);
        self.big_sigma1_0.mix_into(channel);
        self.big_sigma1_1.mix_into(channel);
        self.xor_small_sigma0.mix_into(channel);
        self.xor_small_sigma1.mix_into(channel);
        self.xor_big_sigma0_0.mix_into(channel);
        self.xor_big_sigma0_1.mix_into(channel);
        self.xor_big_sigma1.mix_into(channel);
        self.range_check_16.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        inputs: &Vec<SHA256HashInput>,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        Box<InteractionClaimData>,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Write sha256 trace
        let (sha256_claim, sha256_trace, sha256_interaction_claim_data) =
            sha256::Claim::write_trace(inputs);

        // Create vectors to collect traces and claims to avoid large stack allocation
        let mut all_traces = Vec::new();
        // Start with SHA256 trace
        all_traces.extend(sha256_trace.to_evals());

        // Helper macro to reduce code duplication and avoid stack allocation
        macro_rules! write_trace_component {
            ($component:ident) => {{
                let (claim, trace, interaction_claim_data) = $component::Claim::write_trace(
                    sha256_interaction_claim_data
                        .lookup_data
                        .$component
                        .par_iter()
                        .map(|v| v.as_slice()),
                );
                all_traces.extend(trace);
                (claim, interaction_claim_data)
            }};
        }

        // Write ch traces
        let (ch_claim, ch_interaction_claim_data) = write_trace_component!(ch);

        // Write maj traces
        let (maj_claim, maj_interaction_claim_data) = write_trace_component!(maj);

        // Write sigma traces
        let (small_sigma0_0_claim, small_sigma0_0_interaction_claim_data) =
            write_trace_component!(small_sigma0_0);
        let (small_sigma0_1_claim, small_sigma0_1_interaction_claim_data) =
            write_trace_component!(small_sigma0_1);
        let (small_sigma1_0_claim, small_sigma1_0_interaction_claim_data) =
            write_trace_component!(small_sigma1_0);
        let (small_sigma1_1_claim, small_sigma1_1_interaction_claim_data) =
            write_trace_component!(small_sigma1_1);
        let (big_sigma0_0_claim, big_sigma0_0_interaction_claim_data) =
            write_trace_component!(big_sigma0_0);
        let (big_sigma0_1_claim, big_sigma0_1_interaction_claim_data) =
            write_trace_component!(big_sigma0_1);
        let (big_sigma1_0_claim, big_sigma1_0_interaction_claim_data) =
            write_trace_component!(big_sigma1_0);
        let (big_sigma1_1_claim, big_sigma1_1_interaction_claim_data) =
            write_trace_component!(big_sigma1_1);

        // Write xor traces
        let (xor_small_sigma0_claim, xor_small_sigma0_interaction_claim_data) =
            write_trace_component!(xor_small_sigma0);
        let (xor_small_sigma1_claim, xor_small_sigma1_interaction_claim_data) =
            write_trace_component!(xor_small_sigma1);
        let (xor_big_sigma0_0_claim, xor_big_sigma0_0_interaction_claim_data) =
            write_trace_component!(xor_big_sigma0_0);
        let (xor_big_sigma0_1_claim, xor_big_sigma0_1_interaction_claim_data) =
            write_trace_component!(xor_big_sigma0_1);
        let (xor_big_sigma1_claim, xor_big_sigma1_interaction_claim_data) =
            write_trace_component!(xor_big_sigma1);

        // Write range_check components
        let range_check_16_data = sha256_interaction_claim_data
            .lookup_data
            .range_check_16
            .par_iter()
            .flat_map(|vec| vec.par_iter().map(|arr| &arr[0]));
        let (range_check_16_claim, range_check_16_trace, range_check_16_interaction_claim_data) =
            range_check_16::Claim::write_trace(range_check_16_data);
        all_traces.extend(range_check_16_trace);

        // Gather all lookup data - use Box to avoid large stack allocation
        let interaction_claim_data = Box::new(InteractionClaimData {
            sha256: sha256_interaction_claim_data,
            ch: ch_interaction_claim_data,
            maj: maj_interaction_claim_data,
            small_sigma0_0: small_sigma0_0_interaction_claim_data,
            small_sigma0_1: small_sigma0_1_interaction_claim_data,
            small_sigma1_0: small_sigma1_0_interaction_claim_data,
            small_sigma1_1: small_sigma1_1_interaction_claim_data,
            big_sigma0_0: big_sigma0_0_interaction_claim_data,
            big_sigma0_1: big_sigma0_1_interaction_claim_data,
            big_sigma1_0: big_sigma1_0_interaction_claim_data,
            big_sigma1_1: big_sigma1_1_interaction_claim_data,
            xor_small_sigma0: xor_small_sigma0_interaction_claim_data,
            xor_small_sigma1: xor_small_sigma1_interaction_claim_data,
            xor_big_sigma0_0: xor_big_sigma0_0_interaction_claim_data,
            xor_big_sigma0_1: xor_big_sigma0_1_interaction_claim_data,
            xor_big_sigma1: xor_big_sigma1_interaction_claim_data,
            range_check_16: range_check_16_interaction_claim_data,
        });

        (
            Self {
                sha256: sha256_claim,
                ch: ch_claim,
                maj: maj_claim,
                small_sigma0_0: small_sigma0_0_claim,
                small_sigma0_1: small_sigma0_1_claim,
                small_sigma1_0: small_sigma1_0_claim,
                small_sigma1_1: small_sigma1_1_claim,
                big_sigma0_0: big_sigma0_0_claim,
                big_sigma0_1: big_sigma0_1_claim,
                big_sigma1_0: big_sigma1_0_claim,
                big_sigma1_1: big_sigma1_1_claim,
                xor_small_sigma0: xor_small_sigma0_claim,
                xor_small_sigma1: xor_small_sigma1_claim,
                xor_big_sigma0_0: xor_big_sigma0_0_claim,
                xor_big_sigma0_1: xor_big_sigma0_1_claim,
                xor_big_sigma1: xor_big_sigma1_claim,
                range_check_16: range_check_16_claim,
            },
            all_traces,
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

        let (ch_interaction_claim, ch_interaction_trace) =
            ch::InteractionClaim::write_interaction_trace(
                &relations.ch,
                &interaction_claim_data.ch,
            );
        let (maj_interaction_claim, maj_interaction_trace) =
            maj::InteractionClaim::write_interaction_trace(
                &relations.maj,
                &interaction_claim_data.maj,
            );

        let (small_sigma0_0_interaction_claim, small_sigma0_0_interaction_trace) =
            small_sigma0_0::InteractionClaim::write_interaction_trace(
                &relations.small_sigma0_0,
                &interaction_claim_data.small_sigma0_0,
            );
        let (small_sigma0_1_interaction_claim, small_sigma0_1_interaction_trace) =
            small_sigma0_1::InteractionClaim::write_interaction_trace(
                &relations.small_sigma0_1,
                &interaction_claim_data.small_sigma0_1,
            );
        let (small_sigma1_0_interaction_claim, small_sigma1_0_interaction_trace) =
            small_sigma1_0::InteractionClaim::write_interaction_trace(
                &relations.small_sigma1_0,
                &interaction_claim_data.small_sigma1_0,
            );
        let (small_sigma1_1_interaction_claim, small_sigma1_1_interaction_trace) =
            small_sigma1_1::InteractionClaim::write_interaction_trace(
                &relations.small_sigma1_1,
                &interaction_claim_data.small_sigma1_1,
            );
        let (big_sigma0_0_interaction_claim, big_sigma0_0_interaction_trace) =
            big_sigma0_0::InteractionClaim::write_interaction_trace(
                &relations.big_sigma0_0,
                &interaction_claim_data.big_sigma0_0,
            );
        let (big_sigma0_1_interaction_claim, big_sigma0_1_interaction_trace) =
            big_sigma0_1::InteractionClaim::write_interaction_trace(
                &relations.big_sigma0_1,
                &interaction_claim_data.big_sigma0_1,
            );
        let (big_sigma1_0_interaction_claim, big_sigma1_0_interaction_trace) =
            big_sigma1_0::InteractionClaim::write_interaction_trace(
                &relations.big_sigma1_0,
                &interaction_claim_data.big_sigma1_0,
            );
        let (big_sigma1_1_interaction_claim, big_sigma1_1_interaction_trace) =
            big_sigma1_1::InteractionClaim::write_interaction_trace(
                &relations.big_sigma1_1,
                &interaction_claim_data.big_sigma1_1,
            );
        let (xor_small_sigma0_interaction_claim, xor_small_sigma0_interaction_trace) =
            xor_small_sigma0::InteractionClaim::write_interaction_trace(
                &relations.xor_small_sigma0,
                &interaction_claim_data.xor_small_sigma0,
            );
        let (xor_small_sigma1_interaction_claim, xor_small_sigma1_interaction_trace) =
            xor_small_sigma1::InteractionClaim::write_interaction_trace(
                &relations.xor_small_sigma1,
                &interaction_claim_data.xor_small_sigma1,
            );
        let (xor_big_sigma0_0_interaction_claim, xor_big_sigma0_0_interaction_trace) =
            xor_big_sigma0_0::InteractionClaim::write_interaction_trace(
                &relations.xor_big_sigma0_0,
                &interaction_claim_data.xor_big_sigma0_0,
            );
        let (xor_big_sigma0_1_interaction_claim, xor_big_sigma0_1_interaction_trace) =
            xor_big_sigma0_1::InteractionClaim::write_interaction_trace(
                &relations.xor_big_sigma0_1,
                &interaction_claim_data.xor_big_sigma0_1,
            );
        let (xor_big_sigma1_interaction_claim, xor_big_sigma1_interaction_trace) =
            xor_big_sigma1::InteractionClaim::write_interaction_trace(
                &relations.xor_big_sigma1,
                &interaction_claim_data.xor_big_sigma1,
            );
        let (range_check_16_interaction_claim, range_check_16_interaction_trace) =
            range_check_16::InteractionClaim::write_interaction_trace(
                &relations.range_check_16,
                &interaction_claim_data.range_check_16,
            );

        let trace = sha256_interaction_trace
            .into_iter()
            .chain(ch_interaction_trace)
            .chain(maj_interaction_trace)
            .chain(small_sigma0_0_interaction_trace)
            .chain(small_sigma0_1_interaction_trace)
            .chain(small_sigma1_0_interaction_trace)
            .chain(small_sigma1_1_interaction_trace)
            .chain(big_sigma0_0_interaction_trace)
            .chain(big_sigma0_1_interaction_trace)
            .chain(big_sigma1_0_interaction_trace)
            .chain(big_sigma1_1_interaction_trace)
            .chain(xor_small_sigma0_interaction_trace)
            .chain(xor_small_sigma1_interaction_trace)
            .chain(xor_big_sigma0_0_interaction_trace)
            .chain(xor_big_sigma0_1_interaction_trace)
            .chain(xor_big_sigma1_interaction_trace)
            .chain(range_check_16_interaction_trace);
        (
            trace,
            Self {
                sha256: sha256_interaction_claim,
                ch: ch_interaction_claim,
                maj: maj_interaction_claim,
                small_sigma0_0: small_sigma0_0_interaction_claim,
                small_sigma0_1: small_sigma0_1_interaction_claim,
                small_sigma1_0: small_sigma1_0_interaction_claim,
                small_sigma1_1: small_sigma1_1_interaction_claim,
                big_sigma0_0: big_sigma0_0_interaction_claim,
                big_sigma0_1: big_sigma0_1_interaction_claim,
                big_sigma1_0: big_sigma1_0_interaction_claim,
                big_sigma1_1: big_sigma1_1_interaction_claim,
                xor_small_sigma0: xor_small_sigma0_interaction_claim,
                xor_small_sigma1: xor_small_sigma1_interaction_claim,
                xor_big_sigma0_0: xor_big_sigma0_0_interaction_claim,
                xor_big_sigma0_1: xor_big_sigma0_1_interaction_claim,
                xor_big_sigma1: xor_big_sigma1_interaction_claim,
                range_check_16: range_check_16_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self) -> SecureField {
        let mut sum = SecureField::zero();
        sum += self.sha256.claimed_sum;
        sum += self.ch.claimed_sum;
        sum += self.maj.claimed_sum;
        sum += self.small_sigma0_0.claimed_sum;
        sum += self.small_sigma0_1.claimed_sum;
        sum += self.small_sigma1_0.claimed_sum;
        sum += self.small_sigma1_1.claimed_sum;
        sum += self.big_sigma0_0.claimed_sum;
        sum += self.big_sigma0_1.claimed_sum;
        sum += self.big_sigma1_0.claimed_sum;
        sum += self.big_sigma1_1.claimed_sum;
        sum += self.xor_small_sigma0.claimed_sum;
        sum += self.xor_small_sigma1.claimed_sum;
        sum += self.xor_big_sigma0_0.claimed_sum;
        sum += self.xor_big_sigma0_1.claimed_sum;
        sum += self.xor_big_sigma1.claimed_sum;
        sum += self.range_check_16.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.sha256.mix_into(channel);
        self.ch.mix_into(channel);
        self.maj.mix_into(channel);
        self.small_sigma0_0.mix_into(channel);
        self.small_sigma0_1.mix_into(channel);
        self.small_sigma1_0.mix_into(channel);
        self.small_sigma1_1.mix_into(channel);
        self.big_sigma0_0.mix_into(channel);
        self.big_sigma0_1.mix_into(channel);
        self.big_sigma1_0.mix_into(channel);
        self.big_sigma1_1.mix_into(channel);
        self.xor_small_sigma0.mix_into(channel);
        self.xor_small_sigma1.mix_into(channel);
        self.xor_big_sigma0_0.mix_into(channel);
        self.xor_big_sigma0_1.mix_into(channel);
        self.xor_big_sigma1.mix_into(channel);
        self.range_check_16.mix_into(channel);
    }
}

pub struct Components {
    pub sha256: sha256::Component,
    pub ch: ch::Component,
    pub maj: maj::Component,
    pub small_sigma0_0: small_sigma0_0::Component,
    pub small_sigma0_1: small_sigma0_1::Component,
    pub small_sigma1_0: small_sigma1_0::Component,
    pub small_sigma1_1: small_sigma1_1::Component,
    pub big_sigma0_0: big_sigma0_0::Component,
    pub big_sigma0_1: big_sigma0_1::Component,
    pub big_sigma1_0: big_sigma1_0::Component,
    pub big_sigma1_1: big_sigma1_1::Component,
    pub xor_small_sigma0: xor_small_sigma0::Component,
    pub xor_small_sigma1: xor_small_sigma1::Component,
    pub xor_big_sigma0_0: xor_big_sigma0_0::Component,
    pub xor_big_sigma0_1: xor_big_sigma0_1::Component,
    pub xor_big_sigma1: xor_big_sigma1::Component,
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
            ch: ch::Component::new(
                location_allocator,
                ch::Eval {
                    claim: claim.ch,
                    relation: relations.ch.clone(),
                },
                interaction_claim.ch.claimed_sum,
            ),
            maj: maj::Component::new(
                location_allocator,
                maj::Eval {
                    claim: claim.maj,
                    relation: relations.maj.clone(),
                },
                interaction_claim.maj.claimed_sum,
            ),
            small_sigma0_0: small_sigma0_0::Component::new(
                location_allocator,
                small_sigma0_0::Eval {
                    claim: claim.small_sigma0_0,
                    relation: relations.small_sigma0_0.clone(),
                },
                interaction_claim.small_sigma0_0.claimed_sum,
            ),
            small_sigma0_1: small_sigma0_1::Component::new(
                location_allocator,
                small_sigma0_1::Eval {
                    claim: claim.small_sigma0_1,
                    relation: relations.small_sigma0_1.clone(),
                },
                interaction_claim.small_sigma0_1.claimed_sum,
            ),
            small_sigma1_0: small_sigma1_0::Component::new(
                location_allocator,
                small_sigma1_0::Eval {
                    claim: claim.small_sigma1_0,
                    relation: relations.small_sigma1_0.clone(),
                },
                interaction_claim.small_sigma1_0.claimed_sum,
            ),
            small_sigma1_1: small_sigma1_1::Component::new(
                location_allocator,
                small_sigma1_1::Eval {
                    claim: claim.small_sigma1_1,
                    relation: relations.small_sigma1_1.clone(),
                },
                interaction_claim.small_sigma1_1.claimed_sum,
            ),
            big_sigma0_0: big_sigma0_0::Component::new(
                location_allocator,
                big_sigma0_0::Eval {
                    claim: claim.big_sigma0_0,
                    relation: relations.big_sigma0_0.clone(),
                },
                interaction_claim.big_sigma0_0.claimed_sum,
            ),
            big_sigma0_1: big_sigma0_1::Component::new(
                location_allocator,
                big_sigma0_1::Eval {
                    claim: claim.big_sigma0_1,
                    relation: relations.big_sigma0_1.clone(),
                },
                interaction_claim.big_sigma0_1.claimed_sum,
            ),
            big_sigma1_0: big_sigma1_0::Component::new(
                location_allocator,
                big_sigma1_0::Eval {
                    claim: claim.big_sigma1_0,
                    relation: relations.big_sigma1_0.clone(),
                },
                interaction_claim.big_sigma1_0.claimed_sum,
            ),
            big_sigma1_1: big_sigma1_1::Component::new(
                location_allocator,
                big_sigma1_1::Eval {
                    claim: claim.big_sigma1_1,
                    relation: relations.big_sigma1_1.clone(),
                },
                interaction_claim.big_sigma1_1.claimed_sum,
            ),
            xor_small_sigma0: xor_small_sigma0::Component::new(
                location_allocator,
                xor_small_sigma0::Eval {
                    claim: claim.xor_small_sigma0,
                    relation: relations.xor_small_sigma0.clone(),
                },
                interaction_claim.xor_small_sigma0.claimed_sum,
            ),
            xor_small_sigma1: xor_small_sigma1::Component::new(
                location_allocator,
                xor_small_sigma1::Eval {
                    claim: claim.xor_small_sigma1,
                    relation: relations.xor_small_sigma1.clone(),
                },
                interaction_claim.xor_small_sigma1.claimed_sum,
            ),
            xor_big_sigma0_0: xor_big_sigma0_0::Component::new(
                location_allocator,
                xor_big_sigma0_0::Eval {
                    claim: claim.xor_big_sigma0_0,
                    relation: relations.xor_big_sigma0_0.clone(),
                },
                interaction_claim.xor_big_sigma0_0.claimed_sum,
            ),
            xor_big_sigma0_1: xor_big_sigma0_1::Component::new(
                location_allocator,
                xor_big_sigma0_1::Eval {
                    claim: claim.xor_big_sigma0_1,
                    relation: relations.xor_big_sigma0_1.clone(),
                },
                interaction_claim.xor_big_sigma0_1.claimed_sum,
            ),
            xor_big_sigma1: xor_big_sigma1::Component::new(
                location_allocator,
                xor_big_sigma1::Eval {
                    claim: claim.xor_big_sigma1,
                    relation: relations.xor_big_sigma1.clone(),
                },
                interaction_claim.xor_big_sigma1.claimed_sum,
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
            &self.ch,
            &self.maj,
            &self.small_sigma0_0,
            &self.small_sigma0_1,
            &self.small_sigma1_0,
            &self.small_sigma1_1,
            &self.big_sigma0_0,
            &self.big_sigma0_1,
            &self.big_sigma1_0,
            &self.big_sigma1_1,
            &self.xor_small_sigma0,
            &self.xor_small_sigma1,
            &self.xor_big_sigma0_0,
            &self.xor_big_sigma0_1,
            &self.xor_big_sigma1,
            &self.range_check_16,
        ]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![
            &self.sha256,
            &self.ch,
            &self.maj,
            &self.small_sigma0_0,
            &self.small_sigma0_1,
            &self.small_sigma1_0,
            &self.small_sigma1_1,
            &self.big_sigma0_0,
            &self.big_sigma0_1,
            &self.big_sigma1_0,
            &self.big_sigma1_1,
            &self.xor_small_sigma0,
            &self.xor_small_sigma1,
            &self.xor_big_sigma0_0,
            &self.xor_big_sigma0_1,
            &self.xor_big_sigma1,
            &self.range_check_16,
        ]
    }
}
