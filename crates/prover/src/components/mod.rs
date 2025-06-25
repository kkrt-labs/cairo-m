pub mod memory;
pub mod store_imm;

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
    pub store_imm: store_imm::Claim,
    pub memory: memory::Claim,
    pub range_check_20: range_check_20::Claim,
}

pub struct ClaimData {
    pub store_imm: store_imm::ClaimData,
    pub memory: memory::ClaimData,
    pub range_check_20: range_check_20::ClaimData,
}

pub struct Relations {
    pub registers: relations::Registers,
    pub memory: relations::Memory,
    pub range_check_20: relations::RangeCheck_20,
}

#[derive(Serialize, Deserialize)]
pub struct InteractionClaim {
    pub store_imm: store_imm::InteractionClaim,
    pub memory: memory::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.store_imm.log_sizes(),
            self.memory.log_sizes(),
            self.range_check_20.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.store_imm.mix_into(channel);
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        input: &mut ProverInput,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        ClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Write opcode components
        let (store_imm_claim, store_imm_trace, store_imm_claim_data) =
            store_imm::Claim::write_trace(
                input
                    .instructions
                    .states_by_opcodes
                    .entry(Opcode::StoreImm)
                    .or_default(),
            );

        // Write memory component from the prover input
        let (memory_claim, memory_trace, memory_claim_data) =
            memory::Claim::write_trace(input.memory_boundaries.clone());

        // Write range_check components
        // TODO: use memory and other components lookup data to generate multiplicity column
        let range_check_20_lookup_data = store_imm_claim_data
            .lookup_data
            .range_check_20
            .par_iter()
            .flatten();
        let (range_check_20_claim, range_check_20_trace, range_check_20_claim_data) =
            range_check_20::Claim::write_trace(range_check_20_lookup_data);

        // Combine all traces
        let trace = store_imm_trace
            .to_evals()
            .into_iter()
            .chain(memory_trace.to_evals())
            .chain(range_check_20_trace);

        (
            Self {
                store_imm: store_imm_claim,
                memory: memory_claim,
                range_check_20: range_check_20_claim,
            },
            trace,
            ClaimData {
                store_imm: store_imm_claim_data,
                memory: memory_claim_data,
                range_check_20: range_check_20_claim_data,
            },
        )
    }
}

impl InteractionClaim {
    pub fn write_interaction_trace(
        relations: &Relations,
        claim_data: &ClaimData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        Self,
    ) {
        let (store_imm_interaction_trace, store_imm_interaction_claim) =
            store_imm::InteractionClaim::write_interaction_trace(
                &relations.memory,
                &relations.registers,
                &relations.range_check_20,
                &claim_data.store_imm,
            );

        let (memory_interaction_trace, memory_interaction_claim) =
            memory::InteractionClaim::write_interaction_trace(
                &relations.memory,
                &claim_data.memory,
            );

        let (range_check_20_interaction_trace, range_check_20_interaction_claim) =
            range_check_20::InteractionClaim::write_interaction_trace(
                &relations.range_check_20,
                &claim_data.range_check_20,
            );

        (
            store_imm_interaction_trace
                .into_iter()
                .chain(memory_interaction_trace)
                .chain(range_check_20_interaction_trace),
            Self {
                store_imm: store_imm_interaction_claim,
                memory: memory_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self, relations: &Relations, public_data: PublicData) -> SecureField {
        let mut sum = SecureField::zero();
        sum += public_data.initial_logup_sum(relations);
        sum += self.store_imm.claimed_sum;
        sum += self.memory.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.store_imm.mix_into(channel);
        self.memory.mix_into(channel);
        self.range_check_20.mix_into(channel);
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
    pub store_imm: store_imm::Component,
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
            store_imm: store_imm::Component::new(
                location_allocator,
                store_imm::Eval {
                    claim: claim.store_imm.clone(),
                    registers: relations.registers.clone(),
                    memory: relations.memory.clone(),
                    range_check_20: relations.range_check_20.clone(),
                },
                interaction_claim.store_imm.claimed_sum,
            ),
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
        vec![&self.store_imm, &self.memory, &self.range_check_20]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![&self.store_imm, &self.memory, &self.range_check_20]
    }
}
