pub mod decommitments;
pub mod poseidon2;

use num_traits::Zero;
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

use crate::hints::ProverInput;
use crate::public_data::PublicData;
use crate::relations;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub decommitments: decommitments::Claim,
    pub poseidon2: poseidon2::Claim,
}

#[derive(Debug, Clone)]
pub struct Relations {
    pub poseidon2: relations::Poseidon2,
    pub merkle: relations::Merkle,
}

pub struct InteractionClaimData {
    pub decommitments: decommitments::InteractionClaimData,
    pub poseidon2: poseidon2::InteractionClaimData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub decommitments: decommitments::InteractionClaim,
    pub poseidon2: poseidon2::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![self.decommitments.log_sizes(), self.poseidon2.log_sizes()];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.decommitments.mix_into(channel);
        self.poseidon2.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        inputs: ProverInput,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Write decommitments trace
        let (decommitments_claim, decommitments_trace, decommitments_interaction_claim_data) =
            decommitments::Claim::write_trace(&inputs.decommitment_hints);

        // Write poseidon2 trace
        let (poseidon2_claim, poseidon2_trace, poseidon2_interaction_claim_data) =
            poseidon2::Claim::write_trace(&inputs.decommitment_hints);

        // Gather all lookup data
        let interaction_claim_data = InteractionClaimData {
            decommitments: decommitments_interaction_claim_data,
            poseidon2: poseidon2_interaction_claim_data,
        };

        // Combine all traces
        let trace = decommitments_trace
            .to_evals()
            .into_iter()
            .chain(poseidon2_trace.to_evals());

        (
            Self {
                decommitments: decommitments_claim,
                poseidon2: poseidon2_claim,
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
        let (decommitments_interaction_claim, decommitments_interaction_trace) =
            decommitments::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.decommitments,
            );

        let (poseidon2_interaction_claim, poseidon2_interaction_trace) =
            poseidon2::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.poseidon2,
            );

        (
            decommitments_interaction_trace
                .into_iter()
                .chain(poseidon2_interaction_trace),
            Self {
                decommitments: decommitments_interaction_claim,
                poseidon2: poseidon2_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self, relations: &Relations, public_data: PublicData) -> SecureField {
        let mut sum = SecureField::zero();
        sum += public_data.initial_logup_sum(relations);
        sum += self.decommitments.claimed_sum;
        sum += self.poseidon2.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.decommitments.mix_into(channel);
        self.poseidon2.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            poseidon2: relations::Poseidon2::draw(channel),
            merkle: relations::Merkle::draw(channel),
        }
    }
}

pub struct Components {
    pub decommitments: decommitments::Component,
    pub poseidon2: poseidon2::Component,
}

impl Components {
    pub fn new(
        location_allocator: &mut TraceLocationAllocator,
        claim: &Claim,
        interaction_claim: &InteractionClaim,
        relations: &Relations,
    ) -> Self {
        Self {
            decommitments: decommitments::Component::new(
                location_allocator,
                decommitments::Eval {
                    claim: claim.decommitments.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.decommitments.claimed_sum,
            ),
            poseidon2: poseidon2::Component::new(
                location_allocator,
                poseidon2::Eval {
                    claim: claim.poseidon2.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.poseidon2.claimed_sum,
            ),
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        vec![&self.decommitments, &self.poseidon2]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![&self.decommitments, &self.poseidon2]
    }
}
