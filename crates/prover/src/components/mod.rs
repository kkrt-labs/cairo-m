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
use crate::relations;

pub struct Claim<const N: usize> {
    pub memory: memory::Claim,
}

pub struct Relations {
    pub memory: relations::Memory,
}

pub struct LookupData {
    pub memory: memory::LookupData,
}

pub struct InteractionClaim<const N: usize> {
    pub memory: memory::InteractionClaim,
}

impl<const N: usize> Claim<N> {
    pub fn new(input: ProverInput) -> Self {
        Self {
            memory: memory::Claim::new(input.memory_boundaries),
        }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![self.memory.log_sizes()];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        // self.single_constraint.mix_into(channel);
        // self.multiple_constraints.mix_into(channel);
        // self.single_constraint_with_relation.mix_into(channel);
        self.memory.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        &mut self,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // TODO: Write opcode components

        // Write memory component from the prover input
        let (memory_trace, memory_lookup_data) = self.memory.write_trace();

        // Gather all lookup data
        let lookup_data = LookupData {
            memory: memory_lookup_data,
        };

        // Combine all traces
        let trace = memory_trace;

        (trace, lookup_data)
    }
}

impl<const N: usize> InteractionClaim<N> {
    pub fn write_interaction_trace(
        relations: &Relations,
        lookup_data: &LookupData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        Self,
    ) {
        let (memory_trace, memory_interaction_claim) =
            memory::InteractionClaim::write_interaction_trace(
                &relations.memory,
                &lookup_data.memory,
            );

        (
            memory_trace,
            Self {
                memory: memory_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self) -> SecureField {
        let mut sum = SecureField::zero();
        // sum += self.single_constraint_with_relation.claimed_sum;
        sum += self.memory.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        // self.single_constraint_with_relation.mix_into(channel);
        self.memory.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            memory: relations::Memory::draw(channel),
        }
    }
}

pub struct Components<const N: usize> {
    pub memory: memory::Component,
}

impl<const N: usize> Components<N> {
    pub fn new(
        location_allocator: &mut TraceLocationAllocator,
        claim: &Claim<N>,
        interaction_claim: &InteractionClaim<N>,
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
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        vec![&self.memory]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![&self.memory]
    }
}
