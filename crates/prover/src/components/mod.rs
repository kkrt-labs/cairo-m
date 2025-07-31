pub mod chips;
pub mod clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_prover::core::air::{Component as ComponentVerifier, ComponentProver};
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::backend::simd::SimdBackend;
pub use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::adapter::{MockHasher, ProverInput};
use crate::preprocessed::range_check::range_check_20;
use crate::public_data::PublicData;
use crate::relations;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub opcodes: opcodes::Claim,
    pub memory: memory::Claim,
    pub merkle: merkle::Claim,
    pub clock_update: clock_update::Claim,
    pub range_check_20: range_check_20::Claim,
}

#[derive(Debug, Clone)]
pub struct Relations {
    pub registers: relations::Registers,
    pub memory: relations::Memory,
    pub merkle: relations::Merkle,
    pub range_check_20: relations::RangeCheck20,
}

pub struct InteractionClaimData {
    pub opcodes: opcodes::InteractionClaimData,
    pub memory: memory::InteractionClaimData,
    pub merkle: merkle::InteractionClaimData,
    pub clock_update: clock_update::InteractionClaimData,
    pub range_check_20: range_check_20::InteractionClaimData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub opcodes: opcodes::InteractionClaim,
    pub memory: memory::InteractionClaim,
    pub merkle: merkle::InteractionClaim,
    pub clock_update: clock_update::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.opcodes.log_sizes(),
            self.memory.log_sizes(),
            self.merkle.log_sizes(),
            self.clock_update.log_sizes(),
            self.range_check_20.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.memory.mix_into(channel);
        self.merkle.mix_into(channel);
        self.clock_update.mix_into(channel);
        self.range_check_20.mix_into(channel);
    }

    pub fn write_trace<MC: MerkleChannel>(
        input: &mut ProverInput,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Write opcode components
        let (opcodes_claim, opcodes_trace, opcodes_interaction_claim_data) =
            opcodes::Claim::write_trace(&mut input.instructions);

        // Write memory trace
        let (memory_claim, memory_trace, memory_interaction_claim_data) =
            memory::Claim::write_trace(&input.memory, &input.merkle_trees);

        // Write merkle trace
        let (merkle_claim, merkle_trace, merkle_interaction_claim_data) =
            merkle::Claim::write_trace::<MC, MockHasher>(&input.merkle_trees);

        // Write clock update trace
        let (clock_update_claim, clock_update_trace, clock_update_interaction_claim_data) =
            clock_update::Claim::write_trace(&input.memory.clock_update_data);

        // Write range_check components
        let range_check_data = opcodes_interaction_claim_data.range_check_20();
        let (range_check_20_claim, range_check_20_trace, range_check_20_interaction_claim_data) =
            range_check_20::Claim::write_trace(range_check_data);

        // Gather all lookup data
        let interaction_claim_data = InteractionClaimData {
            opcodes: opcodes_interaction_claim_data,
            memory: memory_interaction_claim_data,
            merkle: merkle_interaction_claim_data,
            clock_update: clock_update_interaction_claim_data,
            range_check_20: range_check_20_interaction_claim_data,
        };

        // Combine all traces
        let trace = opcodes_trace
            .into_iter()
            .chain(memory_trace.to_evals())
            .chain(merkle_trace.to_evals())
            .chain(clock_update_trace.to_evals())
            .chain(range_check_20_trace);

        (
            Self {
                opcodes: opcodes_claim,
                memory: memory_claim,
                merkle: merkle_claim,
                clock_update: clock_update_claim,
                range_check_20: range_check_20_claim,
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
        let (opcodes_interaction_claim, opcodes_interaction_trace) =
            opcodes::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.opcodes,
            );

        let (memory_interaction_claim, memory_interaction_trace) =
            memory::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.memory,
            );

        let (merkle_interaction_claim, merkle_interaction_trace) =
            merkle::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.merkle,
            );

        let (clock_update_interaction_claim, clock_update_interaction_trace) =
            clock_update::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.clock_update,
            );

        let (range_check_20_interaction_claim, range_check_20_interaction_trace) =
            range_check_20::InteractionClaim::write_interaction_trace(
                &relations.range_check_20,
                &interaction_claim_data.range_check_20,
            );

        (
            opcodes_interaction_trace
                .into_iter()
                .chain(memory_interaction_trace)
                .chain(merkle_interaction_trace)
                .chain(clock_update_interaction_trace)
                .chain(range_check_20_interaction_trace),
            Self {
                opcodes: opcodes_interaction_claim,
                memory: memory_interaction_claim,
                merkle: merkle_interaction_claim,
                clock_update: clock_update_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
            },
        )
    }

    pub fn claimed_sum(&self, relations: &Relations, public_data: PublicData) -> SecureField {
        let mut sum = SecureField::zero();
        sum += public_data.initial_logup_sum(relations);
        sum += self.opcodes.claimed_sum();
        sum += self.memory.claimed_sum;
        sum += self.merkle.claimed_sum;
        sum += self.clock_update.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.memory.mix_into(channel);
        self.merkle.mix_into(channel);
        self.clock_update.mix_into(channel);
        self.range_check_20.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            registers: relations::Registers::draw(channel),
            memory: relations::Memory::draw(channel),
            merkle: relations::Merkle::draw(channel),
            range_check_20: relations::RangeCheck20::draw(channel),
        }
    }
}

pub struct Components {
    pub opcodes: opcodes::Component,
    pub memory: memory::Component,
    pub merkle: merkle::Component,
    pub clock_update: clock_update::Component,
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
            opcodes: opcodes::Component::new(
                location_allocator,
                &claim.opcodes,
                &interaction_claim.opcodes,
                relations,
            ),
            memory: memory::Component::new(
                location_allocator,
                memory::Eval {
                    claim: claim.memory.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.memory.claimed_sum,
            ),
            merkle: merkle::Component::new(
                location_allocator,
                merkle::Eval {
                    claim: claim.merkle.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.merkle.claimed_sum,
            ),
            clock_update: clock_update::Component::new(
                location_allocator,
                clock_update::Eval {
                    claim: claim.clock_update.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.clock_update.claimed_sum,
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
        let mut provers = self.opcodes.provers();
        provers.push(&self.memory);
        provers.push(&self.merkle);
        provers.push(&self.clock_update);
        provers.push(&self.range_check_20);
        provers
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        let mut verifiers = self.opcodes.verifiers();
        verifiers.push(&self.memory);
        verifiers.push(&self.merkle);
        verifiers.push(&self.clock_update);
        verifiers.push(&self.range_check_20);
        verifiers
    }
}
