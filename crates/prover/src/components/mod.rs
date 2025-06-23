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

use crate::preprocessed::range_check_20;
use crate::relations;

pub struct Claim<const N: usize> {
    pub single_constraint: single_constraint::Claim<N>,
    pub multiple_constraints: multiple_constraints::Claim<N>,
    pub single_constraint_with_relation: single_constraint_with_relation::Claim,
    pub memory: memory::Claim,
    pub range_check_20: range_check_20::Claim,
}

pub struct Relations {
    pub memory: relations::Memory,
    pub range_check_20: relations::RangeCheck_20,
}

pub struct LookupData {
    pub single_constraint_with_relation: single_constraint_with_relation::LookupData,
    pub memory: memory::LookupData,
    pub range_check_20: range_check_20::LookupData,
}

pub struct InteractionClaim<const N: usize> {
    pub single_constraint_with_relation: single_constraint_with_relation::InteractionClaim,
    pub memory: memory::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
}

impl<const N: usize> Claim<N> {
    pub fn new(log_size: u32) -> Self {
        Self {
            single_constraint: single_constraint::Claim::new(log_size),
            multiple_constraints: multiple_constraints::Claim::new(log_size),
            single_constraint_with_relation: single_constraint_with_relation::Claim::new(log_size),
            memory: memory::Claim::default(),
            range_check_20: range_check_20::Claim::default(),
        }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.single_constraint.log_sizes(),
            self.multiple_constraints.log_sizes(),
            self.single_constraint_with_relation.log_sizes(),
            self.memory.log_sizes(),
            self.range_check_20.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.single_constraint.mix_into(channel);
        self.multiple_constraints.mix_into(channel);
        self.single_constraint_with_relation.mix_into(channel);
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
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
        // Write opcode components
        let single_trace = self.single_constraint.write_trace();
        let multiple_trace = self.multiple_constraints.write_trace();
        let (single_constraint_with_relation_trace, single_constraint_with_relation_lookup_data) =
            self.single_constraint_with_relation.write_trace();

        // Write data components based on opcode components lookup data
        let (memory_trace, memory_lookup_data) = self
            .memory
            .write_trace(&single_constraint_with_relation_lookup_data.memory);

        // Write range_check components
        // dummy lookup data
        let dummy_range_check_data = vec![];
        let (range_check_20_trace, range_check_20_lookup_data) =
            self.range_check_20.write_trace(&dummy_range_check_data);

        // Gather all lookup data
        let lookup_data = LookupData {
            single_constraint_with_relation: single_constraint_with_relation_lookup_data,
            memory: memory_lookup_data,
            range_check_20: range_check_20_lookup_data,
        };

        // Combine all traces
        let trace = single_trace
            .to_evals()
            .into_iter()
            .chain(multiple_trace.to_evals())
            .chain(single_constraint_with_relation_trace.to_evals())
            .chain(memory_trace)
            .chain(range_check_20_trace);

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
        let (
            single_constraint_with_relation_trace,
            single_constraint_with_relation_interaction_claim,
        ) = single_constraint_with_relation::InteractionClaim::write_interaction_trace(
            &relations.memory,
            &lookup_data.single_constraint_with_relation,
        );

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
            single_constraint_with_relation_trace
                .into_iter()
                .chain(memory_interaction_trace)
                .chain(range_check_20_interaction_trace),
            Self {
                single_constraint_with_relation: single_constraint_with_relation_interaction_claim,
                memory: memory_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self) -> SecureField {
        let mut sum = SecureField::zero();
        sum += self.single_constraint_with_relation.claimed_sum;
        sum += self.memory.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.single_constraint_with_relation.mix_into(channel);
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

pub struct Components<const N: usize> {
    pub single_constraint: single_constraint::Component<N>,
    pub multiple_constraints: multiple_constraints::Component<N>,
    pub single_constraint_with_relation: single_constraint_with_relation::Component<N>,
    pub memory: memory::Component,
    pub range_check_20: range_check_20::Component,
}

impl<const N: usize> Components<N> {
    pub fn new(
        location_allocator: &mut TraceLocationAllocator,
        claim: &Claim<N>,
        interaction_claim: &InteractionClaim<N>,
        relations: &Relations,
    ) -> Self {
        Self {
            single_constraint: single_constraint::Component::new(
                location_allocator,
                single_constraint::Eval {
                    claim: claim.single_constraint,
                },
                SecureField::default(),
            ),
            multiple_constraints: multiple_constraints::Component::new(
                location_allocator,
                multiple_constraints::Eval {
                    claim: claim.multiple_constraints,
                },
                SecureField::default(),
            ),
            single_constraint_with_relation: single_constraint_with_relation::Component::new(
                location_allocator,
                single_constraint_with_relation::Eval {
                    claim: claim.single_constraint_with_relation,
                    memory: relations.memory.clone(),
                },
                interaction_claim
                    .single_constraint_with_relation
                    .claimed_sum,
            ),
            memory: memory::Component::new(
                location_allocator,
                memory::Eval {
                    claim: claim.memory,
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
        vec![
            &self.single_constraint,
            &self.multiple_constraints,
            &self.single_constraint_with_relation,
            &self.memory,
            &self.range_check_20,
        ]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![
            &self.single_constraint,
            &self.multiple_constraints,
            &self.single_constraint_with_relation,
            &self.memory,
            &self.range_check_20,
        ]
    }
}
