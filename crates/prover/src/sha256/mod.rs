pub mod debug_tools;
pub mod prover_sha256;
#[cfg(test)]
mod tests;
pub mod verifier_sha256;

use crate::preprocessed::sigma::{
    big_sigma0_0, big_sigma0_1, big_sigma1_0, big_sigma1_1, small_sigma0_0, small_sigma0_1,
    small_sigma1_0, small_sigma1_1,
};
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
    pub small_sigma0_0: small_sigma0_0::Claim,
    pub small_sigma0_1: small_sigma0_1::Claim,
    pub small_sigma1_0: small_sigma1_0::Claim,
    pub small_sigma1_1: small_sigma1_1::Claim,
    pub big_sigma0_0: big_sigma0_0::Claim,
    pub big_sigma0_1: big_sigma0_1::Claim,
    pub big_sigma1_0: big_sigma1_0::Claim,
    pub big_sigma1_1: big_sigma1_1::Claim,
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
    pub small_sigma0_0: small_sigma0_0::InteractionClaimData,
    pub small_sigma0_1: small_sigma0_1::InteractionClaimData,
    pub small_sigma1_0: small_sigma1_0::InteractionClaimData,
    pub small_sigma1_1: small_sigma1_1::InteractionClaimData,
    pub big_sigma0_0: big_sigma0_0::InteractionClaimData,
    pub big_sigma0_1: big_sigma0_1::InteractionClaimData,
    pub big_sigma1_0: big_sigma1_0::InteractionClaimData,
    pub big_sigma1_1: big_sigma1_1::InteractionClaimData,
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
    pub small_sigma0_0: small_sigma0_0::InteractionClaim,
    pub small_sigma0_1: small_sigma0_1::InteractionClaim,
    pub small_sigma1_0: small_sigma1_0::InteractionClaim,
    pub small_sigma1_1: small_sigma1_1::InteractionClaim,
    pub big_sigma0_0: big_sigma0_0::InteractionClaim,
    pub big_sigma0_1: big_sigma0_1::InteractionClaim,
    pub big_sigma1_0: big_sigma1_0::InteractionClaim,
    pub big_sigma1_1: big_sigma1_1::InteractionClaim,
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
            self.small_sigma0_0.log_sizes(),
            self.small_sigma0_1.log_sizes(),
            self.small_sigma1_0.log_sizes(),
            self.small_sigma1_1.log_sizes(),
            self.big_sigma0_0.log_sizes(),
            self.big_sigma0_1.log_sizes(),
            self.big_sigma1_0.log_sizes(),
            self.big_sigma1_1.log_sizes(),
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
        self.small_sigma0_0.mix_into(channel);
        self.small_sigma0_1.mix_into(channel);
        self.small_sigma1_0.mix_into(channel);
        self.small_sigma1_1.mix_into(channel);
        self.big_sigma0_0.mix_into(channel);
        self.big_sigma0_1.mix_into(channel);
        self.big_sigma1_0.mix_into(channel);
        self.big_sigma1_1.mix_into(channel);
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
        let (ch_l0_claim, ch_l0_interaction_claim_data) = write_trace_component!(ch_l0);
        let (ch_l1_claim, ch_l1_interaction_claim_data) = write_trace_component!(ch_l1);
        let (ch_l2_claim, ch_l2_interaction_claim_data) = write_trace_component!(ch_l2);
        let (ch_h0_claim, ch_h0_interaction_claim_data) = write_trace_component!(ch_h0);
        let (ch_h1_claim, ch_h1_interaction_claim_data) = write_trace_component!(ch_h1);
        let (ch_h2_claim, ch_h2_interaction_claim_data) = write_trace_component!(ch_h2);

        // Write maj traces
        let (maj_l0_claim, maj_l0_interaction_claim_data) = write_trace_component!(maj_l0);
        let (maj_l1_claim, maj_l1_interaction_claim_data) = write_trace_component!(maj_l1);
        let (maj_l2_claim, maj_l2_interaction_claim_data) = write_trace_component!(maj_l2);
        let (maj_h0_claim, maj_h0_interaction_claim_data) = write_trace_component!(maj_h0);
        let (maj_h1_claim, maj_h1_interaction_claim_data) = write_trace_component!(maj_h1);
        let (maj_h2_claim, maj_h2_interaction_claim_data) = write_trace_component!(maj_h2);

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
            small_sigma0_0: small_sigma0_0_interaction_claim_data,
            small_sigma0_1: small_sigma0_1_interaction_claim_data,
            small_sigma1_0: small_sigma1_0_interaction_claim_data,
            small_sigma1_1: small_sigma1_1_interaction_claim_data,
            big_sigma0_0: big_sigma0_0_interaction_claim_data,
            big_sigma0_1: big_sigma0_1_interaction_claim_data,
            big_sigma1_0: big_sigma1_0_interaction_claim_data,
            big_sigma1_1: big_sigma1_1_interaction_claim_data,
            range_check_16: range_check_16_interaction_claim_data,
        });

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
                small_sigma0_0: small_sigma0_0_claim,
                small_sigma0_1: small_sigma0_1_claim,
                small_sigma1_0: small_sigma1_0_claim,
                small_sigma1_1: small_sigma1_1_claim,
                big_sigma0_0: big_sigma0_0_claim,
                big_sigma0_1: big_sigma0_1_claim,
                big_sigma1_0: big_sigma1_0_claim,
                big_sigma1_1: big_sigma1_1_claim,
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
            .chain(small_sigma0_0_interaction_trace)
            .chain(small_sigma0_1_interaction_trace)
            .chain(small_sigma1_0_interaction_trace)
            .chain(small_sigma1_1_interaction_trace)
            .chain(big_sigma0_0_interaction_trace)
            .chain(big_sigma0_1_interaction_trace)
            .chain(big_sigma1_0_interaction_trace)
            .chain(big_sigma1_1_interaction_trace)
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
                small_sigma0_0: small_sigma0_0_interaction_claim,
                small_sigma0_1: small_sigma0_1_interaction_claim,
                small_sigma1_0: small_sigma1_0_interaction_claim,
                small_sigma1_1: small_sigma1_1_interaction_claim,
                big_sigma0_0: big_sigma0_0_interaction_claim,
                big_sigma0_1: big_sigma0_1_interaction_claim,
                big_sigma1_0: big_sigma1_0_interaction_claim,
                big_sigma1_1: big_sigma1_1_interaction_claim,
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
        sum += self.small_sigma0_0.claimed_sum;
        sum += self.small_sigma0_1.claimed_sum;
        sum += self.small_sigma1_0.claimed_sum;
        sum += self.small_sigma1_1.claimed_sum;
        sum += self.big_sigma0_0.claimed_sum;
        sum += self.big_sigma0_1.claimed_sum;
        sum += self.big_sigma1_0.claimed_sum;
        sum += self.big_sigma1_1.claimed_sum;
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
        self.small_sigma0_0.mix_into(channel);
        self.small_sigma0_1.mix_into(channel);
        self.small_sigma1_0.mix_into(channel);
        self.small_sigma1_1.mix_into(channel);
        self.big_sigma0_0.mix_into(channel);
        self.big_sigma0_1.mix_into(channel);
        self.big_sigma1_0.mix_into(channel);
        self.big_sigma1_1.mix_into(channel);
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
    pub small_sigma0_0: small_sigma0_0::Component,
    pub small_sigma0_1: small_sigma0_1::Component,
    pub small_sigma1_0: small_sigma1_0::Component,
    pub small_sigma1_1: small_sigma1_1::Component,
    pub big_sigma0_0: big_sigma0_0::Component,
    pub big_sigma0_1: big_sigma0_1::Component,
    pub big_sigma1_0: big_sigma1_0::Component,
    pub big_sigma1_1: big_sigma1_1::Component,
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
            &self.small_sigma0_0,
            &self.small_sigma0_1,
            &self.small_sigma1_0,
            &self.small_sigma1_1,
            &self.big_sigma0_0,
            &self.big_sigma0_1,
            &self.big_sigma1_0,
            &self.big_sigma1_1,
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
            &self.small_sigma0_0,
            &self.small_sigma0_1,
            &self.small_sigma1_0,
            &self.small_sigma1_1,
            &self.big_sigma0_0,
            &self.big_sigma0_1,
            &self.big_sigma1_0,
            &self.big_sigma1_1,
            &self.range_check_16,
        ]
    }
}
