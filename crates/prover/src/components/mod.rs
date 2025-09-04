pub mod clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
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

use crate::adapter::ProverInput;
use crate::preprocessed::bitwise::{bitwise_and, bitwise_or, bitwise_xor};
use crate::preprocessed::range_check::{range_check_16, range_check_20, range_check_8};
use crate::public_data::PublicData;
use crate::relations;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub opcodes: opcodes::Claim,
    pub memory: memory::Claim,
    pub merkle: merkle::Claim,
    pub clock_update: clock_update::Claim,
    pub poseidon2: poseidon2::Claim,
    pub range_check_8: range_check_8::Claim,
    pub range_check_16: range_check_16::Claim,
    pub range_check_20: range_check_20::Claim,
    pub bitwise_and: bitwise_and::Claim,
    pub bitwise_or: bitwise_or::Claim,
    pub bitwise_xor: bitwise_xor::Claim,
}

#[derive(Debug, Clone)]
pub struct Relations {
    pub registers: relations::Registers,
    pub memory: relations::Memory,
    pub merkle: relations::Merkle,
    pub poseidon2: relations::Poseidon2,
    pub range_check_8: relations::RangeCheck8,
    pub range_check_16: relations::RangeCheck16,
    pub range_check_20: relations::RangeCheck20,
    pub bitwise_and: relations::BitwiseAnd,
    pub bitwise_or: relations::BitwiseOr,
    pub bitwise_xor: relations::BitwiseXor,
}

pub struct InteractionClaimData {
    pub opcodes: opcodes::InteractionClaimData,
    pub memory: memory::InteractionClaimData,
    pub merkle: merkle::InteractionClaimData,
    pub clock_update: clock_update::InteractionClaimData,
    pub poseidon2: poseidon2::InteractionClaimData,
    pub range_check_8: range_check_8::InteractionClaimData,
    pub range_check_16: range_check_16::InteractionClaimData,
    pub range_check_20: range_check_20::InteractionClaimData,
    pub bitwise_and: bitwise_and::InteractionClaimData,
    pub bitwise_or: bitwise_or::InteractionClaimData,
    pub bitwise_xor: bitwise_xor::InteractionClaimData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub opcodes: opcodes::InteractionClaim,
    pub memory: memory::InteractionClaim,
    pub merkle: merkle::InteractionClaim,
    pub clock_update: clock_update::InteractionClaim,
    pub poseidon2: poseidon2::InteractionClaim,
    pub range_check_8: range_check_8::InteractionClaim,
    pub range_check_16: range_check_16::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
    pub bitwise_and: bitwise_and::InteractionClaim,
    pub bitwise_or: bitwise_or::InteractionClaim,
    pub bitwise_xor: bitwise_xor::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.opcodes.log_sizes(),
            self.memory.log_sizes(),
            self.merkle.log_sizes(),
            self.clock_update.log_sizes(),
            self.poseidon2.log_sizes(),
            self.range_check_8.log_sizes(),
            self.range_check_16.log_sizes(),
            self.range_check_20.log_sizes(),
            self.bitwise_and.log_sizes(),
            self.bitwise_or.log_sizes(),
            self.bitwise_xor.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.memory.mix_into(channel);
        self.merkle.mix_into(channel);
        self.clock_update.mix_into(channel);
        self.poseidon2.mix_into(channel);
        self.range_check_8.mix_into(channel);
        self.range_check_16.mix_into(channel);
        self.range_check_20.mix_into(channel);
        self.bitwise_and.mix_into(channel);
        self.bitwise_or.mix_into(channel);
        self.bitwise_xor.mix_into(channel);
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
            merkle::Claim::write_trace::<MC>(&input.merkle_trees);

        // Write poseidon2 trace
        let (poseidon2_claim, poseidon2_trace, poseidon2_interaction_claim_data) =
            poseidon2::Claim::write_trace(&input.poseidon2_inputs);

        // Write clock update trace
        let (clock_update_claim, clock_update_trace, clock_update_interaction_claim_data) =
            clock_update::Claim::write_trace(&input.memory.clock_update_data);

        // Write range_check components
        let range_check_8_data = opcodes_interaction_claim_data.range_check_8();
        let (range_check_8_claim, range_check_8_trace, range_check_8_interaction_claim_data) =
            range_check_8::Claim::write_trace(range_check_8_data);

        let range_check_16_data = opcodes_interaction_claim_data.range_check_16();
        let (range_check_16_claim, range_check_16_trace, range_check_16_interaction_claim_data) =
            range_check_16::Claim::write_trace(range_check_16_data);

        let range_check_20_data = opcodes_interaction_claim_data.range_check_20();
        let (range_check_20_claim, range_check_20_trace, range_check_20_interaction_claim_data) =
            range_check_20::Claim::write_trace(range_check_20_data);

        // Write bitwise components (empty for now, will be populated when opcodes use them)
        let (bitwise_and_claim, bitwise_and_trace, bitwise_and_interaction_claim_data) =
            bitwise_and::Claim::write_trace(rayon::iter::empty());
        let (bitwise_or_claim, bitwise_or_trace, bitwise_or_interaction_claim_data) =
            bitwise_or::Claim::write_trace(rayon::iter::empty());
        let (bitwise_xor_claim, bitwise_xor_trace, bitwise_xor_interaction_claim_data) =
            bitwise_xor::Claim::write_trace(rayon::iter::empty());

        // Gather all lookup data
        let interaction_claim_data = InteractionClaimData {
            opcodes: opcodes_interaction_claim_data,
            memory: memory_interaction_claim_data,
            merkle: merkle_interaction_claim_data,
            clock_update: clock_update_interaction_claim_data,
            poseidon2: poseidon2_interaction_claim_data,
            range_check_8: range_check_8_interaction_claim_data,
            range_check_16: range_check_16_interaction_claim_data,
            range_check_20: range_check_20_interaction_claim_data,
            bitwise_and: bitwise_and_interaction_claim_data,
            bitwise_or: bitwise_or_interaction_claim_data,
            bitwise_xor: bitwise_xor_interaction_claim_data,
        };

        // Combine all traces
        let trace = opcodes_trace
            .into_iter()
            .chain(memory_trace.to_evals())
            .chain(merkle_trace.to_evals())
            .chain(clock_update_trace.to_evals())
            .chain(poseidon2_trace.to_evals())
            .chain(range_check_8_trace)
            .chain(range_check_16_trace)
            .chain(range_check_20_trace)
            .chain(bitwise_and_trace)
            .chain(bitwise_or_trace)
            .chain(bitwise_xor_trace);

        (
            Self {
                opcodes: opcodes_claim,
                memory: memory_claim,
                merkle: merkle_claim,
                clock_update: clock_update_claim,
                poseidon2: poseidon2_claim,
                range_check_8: range_check_8_claim,
                range_check_16: range_check_16_claim,
                range_check_20: range_check_20_claim,
                bitwise_and: bitwise_and_claim,
                bitwise_or: bitwise_or_claim,
                bitwise_xor: bitwise_xor_claim,
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
        let (poseidon2_interaction_claim, poseidon2_interaction_trace) =
            poseidon2::InteractionClaim::write_interaction_trace(
                relations,
                &interaction_claim_data.poseidon2,
            );

        let (range_check_8_interaction_claim, range_check_8_interaction_trace) =
            range_check_8::InteractionClaim::write_interaction_trace(
                &relations.range_check_8,
                &interaction_claim_data.range_check_8,
            );

        let (range_check_16_interaction_claim, range_check_16_interaction_trace) =
            range_check_16::InteractionClaim::write_interaction_trace(
                &relations.range_check_16,
                &interaction_claim_data.range_check_16,
            );

        let (range_check_20_interaction_claim, range_check_20_interaction_trace) =
            range_check_20::InteractionClaim::write_interaction_trace(
                &relations.range_check_20,
                &interaction_claim_data.range_check_20,
            );

        let (bitwise_and_interaction_claim, bitwise_and_interaction_trace) =
            bitwise_and::InteractionClaim::write_interaction_trace(
                &relations.bitwise_and,
                &interaction_claim_data.bitwise_and,
            );

        let (bitwise_or_interaction_claim, bitwise_or_interaction_trace) =
            bitwise_or::InteractionClaim::write_interaction_trace(
                &relations.bitwise_or,
                &interaction_claim_data.bitwise_or,
            );

        let (bitwise_xor_interaction_claim, bitwise_xor_interaction_trace) =
            bitwise_xor::InteractionClaim::write_interaction_trace(
                &relations.bitwise_xor,
                &interaction_claim_data.bitwise_xor,
            );

        (
            opcodes_interaction_trace
                .into_iter()
                .chain(memory_interaction_trace)
                .chain(merkle_interaction_trace)
                .chain(clock_update_interaction_trace)
                .chain(poseidon2_interaction_trace)
                .chain(range_check_8_interaction_trace)
                .chain(range_check_16_interaction_trace)
                .chain(range_check_20_interaction_trace)
                .chain(bitwise_and_interaction_trace)
                .chain(bitwise_or_interaction_trace)
                .chain(bitwise_xor_interaction_trace),
            Self {
                opcodes: opcodes_interaction_claim,
                memory: memory_interaction_claim,
                merkle: merkle_interaction_claim,
                clock_update: clock_update_interaction_claim,
                poseidon2: poseidon2_interaction_claim,
                range_check_8: range_check_8_interaction_claim,
                range_check_16: range_check_16_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
                bitwise_and: bitwise_and_interaction_claim,
                bitwise_or: bitwise_or_interaction_claim,
                bitwise_xor: bitwise_xor_interaction_claim,
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
        sum += self.poseidon2.claimed_sum;
        sum += self.range_check_8.claimed_sum;
        sum += self.range_check_16.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum += self.bitwise_and.claimed_sum;
        sum += self.bitwise_or.claimed_sum;
        sum += self.bitwise_xor.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.memory.mix_into(channel);
        self.merkle.mix_into(channel);
        self.clock_update.mix_into(channel);
        self.poseidon2.mix_into(channel);
        self.range_check_8.mix_into(channel);
        self.range_check_16.mix_into(channel);
        self.range_check_20.mix_into(channel);
        self.bitwise_and.mix_into(channel);
        self.bitwise_or.mix_into(channel);
        self.bitwise_xor.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            registers: relations::Registers::draw(channel),
            memory: relations::Memory::draw(channel),
            merkle: relations::Merkle::draw(channel),
            poseidon2: relations::Poseidon2::draw(channel),
            range_check_8: relations::RangeCheck8::draw(channel),
            range_check_16: relations::RangeCheck16::draw(channel),
            range_check_20: relations::RangeCheck20::draw(channel),
            bitwise_and: relations::BitwiseAnd::draw(channel),
            bitwise_or: relations::BitwiseOr::draw(channel),
            bitwise_xor: relations::BitwiseXor::draw(channel),
        }
    }
}

pub struct Components {
    pub opcodes: opcodes::Component,
    pub memory: memory::Component,
    pub merkle: merkle::Component,
    pub clock_update: clock_update::Component,
    pub poseidon2: poseidon2::Component,
    pub range_check_8: range_check_8::Component,
    pub range_check_16: range_check_16::Component,
    pub range_check_20: range_check_20::Component,
    pub bitwise_and: bitwise_and::Component,
    pub bitwise_or: bitwise_or::Component,
    pub bitwise_xor: bitwise_xor::Component,
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
            poseidon2: poseidon2::Component::new(
                location_allocator,
                poseidon2::Eval {
                    claim: claim.poseidon2.clone(),
                    relations: relations.clone(),
                },
                interaction_claim.poseidon2.claimed_sum,
            ),
            range_check_8: range_check_8::Component::new(
                location_allocator,
                range_check_8::Eval {
                    claim: claim.range_check_8,
                    relation: relations.range_check_8.clone(),
                },
                interaction_claim.range_check_8.claimed_sum,
            ),
            range_check_16: range_check_16::Component::new(
                location_allocator,
                range_check_16::Eval {
                    claim: claim.range_check_16,
                    relation: relations.range_check_16.clone(),
                },
                interaction_claim.range_check_16.claimed_sum,
            ),
            range_check_20: range_check_20::Component::new(
                location_allocator,
                range_check_20::Eval {
                    claim: claim.range_check_20,
                    relation: relations.range_check_20.clone(),
                },
                interaction_claim.range_check_20.claimed_sum,
            ),
            bitwise_and: bitwise_and::Component::new(
                location_allocator,
                bitwise_and::Eval {
                    claim: claim.bitwise_and,
                    relation: relations.bitwise_and.clone(),
                    claimed_sum: interaction_claim.bitwise_and.claimed_sum,
                },
                interaction_claim.bitwise_and.claimed_sum,
            ),
            bitwise_or: bitwise_or::Component::new(
                location_allocator,
                bitwise_or::Eval {
                    claim: claim.bitwise_or,
                    relation: relations.bitwise_or.clone(),
                    claimed_sum: interaction_claim.bitwise_or.claimed_sum,
                },
                interaction_claim.bitwise_or.claimed_sum,
            ),
            bitwise_xor: bitwise_xor::Component::new(
                location_allocator,
                bitwise_xor::Eval {
                    claim: claim.bitwise_xor,
                    relation: relations.bitwise_xor.clone(),
                    claimed_sum: interaction_claim.bitwise_xor.claimed_sum,
                },
                interaction_claim.bitwise_xor.claimed_sum,
            ),
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        let mut provers = self.opcodes.provers();
        provers.push(&self.memory);
        provers.push(&self.merkle);
        provers.push(&self.clock_update);
        provers.push(&self.poseidon2);
        provers.push(&self.range_check_8);
        provers.push(&self.range_check_16);
        provers.push(&self.range_check_20);
        provers.push(&self.bitwise_and);
        provers.push(&self.bitwise_or);
        provers.push(&self.bitwise_xor);
        provers
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        let mut verifiers = self.opcodes.verifiers();
        verifiers.push(&self.memory);
        verifiers.push(&self.merkle);
        verifiers.push(&self.clock_update);
        verifiers.push(&self.poseidon2);
        verifiers.push(&self.range_check_8);
        verifiers.push(&self.range_check_16);
        verifiers.push(&self.range_check_20);
        verifiers.push(&self.bitwise_and);
        verifiers.push(&self.bitwise_or);
        verifiers.push(&self.bitwise_xor);
        verifiers
    }
}
