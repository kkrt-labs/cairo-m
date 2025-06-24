pub mod memory;
pub mod multiple_constraints;
pub mod single_constraint;
pub mod single_constraint_with_relation;

use num_traits::Zero;
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::TraceLocationAllocator;
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

use crate::adapter::ProverInput;
use crate::preprocessed::range_check_20;
use crate::relations;

#[derive(Default)]
pub struct Claim {
    pub memory: memory::Claim,
    pub range_check_20: range_check_20::Claim,
}

pub struct Relations {
    pub memory: relations::Memory,
    pub range_check_20: relations::RangeCheck_20,
}

pub struct LookupData {
    pub memory: memory::LookupData,
    pub range_check_20: range_check_20::LookupData,
}

pub struct InteractionClaim {
    pub memory: memory::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
}

impl Claim {
    pub fn new() -> Self {
        Self {
            memory: memory::Claim::default(),
            range_check_20: range_check_20::Claim::new(20),
        }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![self.memory.log_sizes(), self.range_check_20.log_sizes()];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        // self.single_constraint.mix_into(channel);
        // self.multiple_constraints.mix_into(channel);
        // self.single_constraint_with_relation.mix_into(channel);
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        &mut self,
        input: ProverInput,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // TODO: Write opcode components

        // Write memory component from the prover input
        let (memory_trace, memory_lookup_data) = self.memory.write_trace(input.memory_boundaries);

        // Write range_check components
        // TODO: use memory and other components lookup data to generate multiplicity column
        let dummy_range_check_data = vec![];
        let (range_check_20_trace, range_check_20_lookup_data) =
            self.range_check_20.write_trace(&dummy_range_check_data);

        // Gather all lookup data
        let lookup_data = LookupData {
            memory: memory_lookup_data,
            range_check_20: range_check_20_lookup_data,
        };

        (
            memory_trace.to_evals().chain(range_check_20_trace),
            lookup_data,
        )
    }
}

impl InteractionClaim {
    pub fn write_interaction_trace(
        relations: &Relations,
        lookup_data: &LookupData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        Self,
    ) {
        let (memory_interaction_trace, memory_interaction_claim) =
            memory::InteractionClaim::write_interaction_trace(
                &relations.memory,
                &lookup_data.memory,
            );

        let (range_check_20_interaction_trace, range_check_20_interaction_claim) =
            range_check_20::InteractionClaim::write_interaction_trace(
                &relations.range_check_20,
                &lookup_data.range_check_20,
            );

        (
            memory_interaction_trace.chain(range_check_20_interaction_trace),
            Self {
                memory: memory_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self) -> SecureField {
        let mut sum = SecureField::zero();
        // sum += self.single_constraint_with_relation.claimed_sum;
        sum += self.memory.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        // self.single_constraint_with_relation.mix_into(channel);
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            memory: relations::Memory::draw(channel),
            range_check_20: relations::RangeCheck_20::draw(channel),
        }
    }
}

pub struct Components {
    pub memory: memory::Component,
    pub range_check_20: range_check_20::Component,
}

impl Components {
    pub fn new(
        location_allocator: &mut TraceLocationAllocator,
        claim: &Claim,
        interaction_claim: &InteractionClaim,
        relations: &Relations,
    ) -> Self {
        Self {
            memory: memory::Component::new(
                location_allocator,
                memory::Eval {
                    claim: claim.memory.clone(),
                    memory: relations.memory.clone(),
                },
                interaction_claim.memory.claimed_sum,
            ),
            range_check_20: range_check_20::Component::new(
                location_allocator,
                range_check_20::Eval {
                    claim: claim.range_check_20,
                    relation: relations.range_check_20.clone(),
                },
                interaction_claim.range_check_20.claimed_sum,
            ),
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        vec![&self.memory, &self.range_check_20]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![&self.memory, &self.range_check_20]
    }
}
