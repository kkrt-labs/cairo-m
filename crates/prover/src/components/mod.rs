pub mod memory;
pub mod store_deref_fp;

use cairo_m_common::Opcode;
use num_traits::Zero;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
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
use crate::preprocessed::range_check::range_check_20;
use crate::public_data::PublicData;
use crate::relations;

#[derive(Serialize, Deserialize)]
pub struct Claim {
    pub memory: memory::Claim,
    pub range_check_20: range_check_20::Claim,
    pub store_deref_fp: store_deref_fp::Claim,
}

pub struct Relations {
    pub registers: relations::Registers,
    pub memory: relations::Memory,
    pub range_check_20: relations::RangeCheck_20,
}

pub struct LookupData {
    pub memory: memory::LookupData,
    pub range_check_20: range_check_20::LookupData,
    pub store_deref_fp: store_deref_fp::InteractionClaimData,
}

#[derive(Serialize, Deserialize)]
pub struct InteractionClaim {
    pub memory: memory::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
    pub store_deref_fp: store_deref_fp::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.memory.log_sizes(),
            self.range_check_20.log_sizes(),
            self.store_deref_fp.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
        self.store_deref_fp.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        input: &mut ProverInput,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // TODO: Write opcode components

        // Write memory component from the prover input
        let (memory_claim, memory_trace, memory_lookup_data) =
            memory::Claim::write_trace(input.memory_boundaries.clone());

        let (store_deref_fp_claim, store_deref_fp_trace, store_deref_fp_interaction_claim_data) =
            store_deref_fp::Claim::write_trace(
                input
                    .instructions
                    .states_by_opcodes
                    .entry(Opcode::StoreDerefFp)
                    .or_default(),
            );

        // Write range_check components
        let range_check_20_lookup_data = store_deref_fp_interaction_claim_data
            .lookup_data
            .range_check_20
            .par_iter()
            .flatten();
        let (range_check_20_claim, range_check_20_trace, range_check_20_lookup_data) =
            range_check_20::Claim::write_trace(range_check_20_lookup_data);

        // Gather all lookup data
        let lookup_data = LookupData {
            memory: memory_lookup_data,
            range_check_20: range_check_20_lookup_data,
            store_deref_fp: store_deref_fp_interaction_claim_data,
        };

        // Combine all traces
        let trace = memory_trace
            .to_evals()
            .into_iter()
            .chain(range_check_20_trace)
            .chain(store_deref_fp_trace.to_evals());

        (
            Self {
                memory: memory_claim,
                range_check_20: range_check_20_claim,
                store_deref_fp: store_deref_fp_claim,
            },
            trace,
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

        let (store_deref_fp_interaction_trace, store_deref_fp_interaction_claim) =
            store_deref_fp::InteractionClaim::write_interaction_trace(
                &relations.memory,
                &relations.registers,
                &relations.range_check_20,
                &lookup_data.store_deref_fp,
            );

        (
            memory_interaction_trace
                .into_iter()
                .chain(range_check_20_interaction_trace)
                .chain(store_deref_fp_interaction_trace),
            Self {
                memory: memory_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
                store_deref_fp: store_deref_fp_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self, relations: &Relations, public_data: PublicData) -> SecureField {
        let mut sum = SecureField::zero();
        sum += public_data.initial_logup_sum(relations);
        sum += self.memory.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum += self.store_deref_fp.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
        self.store_deref_fp.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            registers: relations::Registers::draw(channel),
            memory: relations::Memory::draw(channel),
            range_check_20: relations::RangeCheck_20::draw(channel),
        }
    }
}

pub struct Components {
    pub memory: memory::Component,
    pub range_check_20: range_check_20::Component,
    pub store_deref_fp: store_deref_fp::Component,
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
            store_deref_fp: store_deref_fp::Component::new(
                location_allocator,
                store_deref_fp::Eval {
                    claim: claim.store_deref_fp.clone(),
                    memory: relations.memory.clone(),
                    registers: relations.registers.clone(),
                    range_check_20: relations.range_check_20.clone(),
                },
                interaction_claim.store_deref_fp.claimed_sum,
            ),
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        vec![&self.memory, &self.range_check_20, &self.store_deref_fp]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![&self.memory, &self.range_check_20, &self.store_deref_fp]
    }
}
