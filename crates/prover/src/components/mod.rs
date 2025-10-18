pub mod clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
pub mod poseidon2;
pub mod sha256;
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

use crate::adapter::ProverInput;
use crate::preprocessed::bitwise;
use crate::preprocessed::ch_maj::{ch, maj};
use crate::preprocessed::range_check::{range_check_16, range_check_20, range_check_8};
use crate::preprocessed::sigma::{
    big_sigma0_0, big_sigma0_1, big_sigma1_0, big_sigma1_1, small_sigma0_0, small_sigma0_1,
    small_sigma1_0, small_sigma1_1,
};
use crate::preprocessed::xor::{
    xor_big_sigma0_0, xor_big_sigma0_1, xor_big_sigma1, xor_small_sigma0, xor_small_sigma1,
};
use crate::public_data::PublicData;
use crate::relations;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claim {
    pub opcodes: opcodes::Claim,
    pub memory: memory::Claim,
    pub merkle: merkle::Claim,
    pub clock_update: clock_update::Claim,
    pub poseidon2: poseidon2::Claim,
    pub sha256: sha256::Claim,
    pub ch: ch::Claim,
    pub maj: maj::Claim,
    pub range_check_8: range_check_8::Claim,
    pub range_check_16: range_check_16::Claim,
    pub range_check_20: range_check_20::Claim,
    pub bitwise: bitwise::Claim,
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
}

#[derive(Debug, Clone)]
pub struct Relations {
    pub registers: relations::Registers,
    pub memory: relations::Memory,
    pub merkle: relations::Merkle,
    pub poseidon2: relations::Poseidon2,
    pub small_sigma0_0: relations::SmallSigma0_0,
    pub small_sigma0_1: relations::SmallSigma0_1,
    pub small_sigma1_0: relations::SmallSigma1_0,
    pub small_sigma1_1: relations::SmallSigma1_1,
    pub big_sigma0_0: relations::BigSigma0_0,
    pub big_sigma0_1: relations::BigSigma0_1,
    pub big_sigma1_0: relations::BigSigma1_0,
    pub big_sigma1_1: relations::BigSigma1_1,
    pub xor_small_sigma0: relations::XorSmallSigma0,
    pub xor_small_sigma1: relations::XorSmallSigma1,
    pub xor_big_sigma0_0: relations::XorBigSigma0_0,
    pub xor_big_sigma0_1: relations::XorBigSigma0_1,
    pub xor_big_sigma1: relations::XorBigSigma1,
    pub ch: relations::Ch,
    pub maj: relations::Maj,
    pub range_check_8: relations::RangeCheck8,
    pub range_check_16: relations::RangeCheck16,
    pub range_check_20: relations::RangeCheck20,
    pub bitwise: relations::Bitwise,
}

pub struct InteractionClaimData {
    pub opcodes: opcodes::InteractionClaimData,
    pub memory: memory::InteractionClaimData,
    pub merkle: merkle::InteractionClaimData,
    pub clock_update: clock_update::InteractionClaimData,
    pub poseidon2: poseidon2::InteractionClaimData,
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
    pub range_check_8: range_check_8::InteractionClaimData,
    pub range_check_16: range_check_16::InteractionClaimData,
    pub range_check_20: range_check_20::InteractionClaimData,
    pub bitwise: bitwise::InteractionClaimData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InteractionClaim {
    pub opcodes: opcodes::InteractionClaim,
    pub memory: memory::InteractionClaim,
    pub merkle: merkle::InteractionClaim,
    pub clock_update: clock_update::InteractionClaim,
    pub poseidon2: poseidon2::InteractionClaim,
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
    pub range_check_8: range_check_8::InteractionClaim,
    pub range_check_16: range_check_16::InteractionClaim,
    pub range_check_20: range_check_20::InteractionClaim,
    pub bitwise: bitwise::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.opcodes.log_sizes(),
            self.memory.log_sizes(),
            self.merkle.log_sizes(),
            self.clock_update.log_sizes(),
            self.poseidon2.log_sizes(),
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
            self.range_check_8.log_sizes(),
            self.range_check_16.log_sizes(),
            self.range_check_20.log_sizes(),
            self.bitwise.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.memory.mix_into(channel);
        self.merkle.mix_into(channel);
        self.clock_update.mix_into(channel);
        self.poseidon2.mix_into(channel);
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
        self.range_check_8.mix_into(channel);
        self.range_check_16.mix_into(channel);
        self.range_check_20.mix_into(channel);
        self.bitwise.mix_into(channel);
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

        // Write sha256 trace
        let (sha256_claim, sha256_trace, sha256_interaction_claim_data) =
            sha256::Claim::write_trace(&input.sha256_inputs);

        // Write ch trace
        let (ch_claim, ch_trace, ch_interaction_claim_data) = ch::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .ch
                .par_iter()
                .map(|v| v.as_slice()),
        );
        let (maj_claim, maj_trace, maj_interaction_claim_data) = maj::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .maj
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (small_sigma0_0_claim, small_sigma0_0_trace, small_sigma0_0_interaction_claim_data) =
            small_sigma0_0::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .small_sigma0_0
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (small_sigma0_1_claim, small_sigma0_1_trace, small_sigma0_1_interaction_claim_data) =
            small_sigma0_1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .small_sigma0_1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (small_sigma1_0_claim, small_sigma1_0_trace, small_sigma1_0_interaction_claim_data) =
            small_sigma1_0::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .small_sigma1_0
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (small_sigma1_1_claim, small_sigma1_1_trace, small_sigma1_1_interaction_claim_data) =
            small_sigma1_1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .small_sigma1_1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (big_sigma0_0_claim, big_sigma0_0_trace, big_sigma0_0_interaction_claim_data) =
            big_sigma0_0::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .big_sigma0_0
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (big_sigma0_1_claim, big_sigma0_1_trace, big_sigma0_1_interaction_claim_data) =
            big_sigma0_1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .big_sigma0_1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (big_sigma1_0_claim, big_sigma1_0_trace, big_sigma1_0_interaction_claim_data) =
            big_sigma1_0::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .big_sigma1_0
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        let (big_sigma1_1_claim, big_sigma1_1_trace, big_sigma1_1_interaction_claim_data) =
            big_sigma1_1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .big_sigma1_1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

        // Write xor components
        let (
            xor_small_sigma0_claim,
            xor_small_sigma0_trace,
            xor_small_sigma0_interaction_claim_data,
        ) = xor_small_sigma0::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .xor_small_sigma0
                .par_iter()
                .map(|v| v.as_slice()),
        );

        let (
            xor_small_sigma1_claim,
            xor_small_sigma1_trace,
            xor_small_sigma1_interaction_claim_data,
        ) = xor_small_sigma1::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .xor_small_sigma1
                .par_iter()
                .map(|v| v.as_slice()),
        );
        let (
            xor_big_sigma0_0_claim,
            xor_big_sigma0_0_trace,
            xor_big_sigma0_0_interaction_claim_data,
        ) = xor_big_sigma0_0::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .xor_big_sigma0_0
                .par_iter()
                .map(|v| v.as_slice()),
        );
        let (
            xor_big_sigma0_1_claim,
            xor_big_sigma0_1_trace,
            xor_big_sigma0_1_interaction_claim_data,
        ) = xor_big_sigma0_1::Claim::write_trace(
            sha256_interaction_claim_data
                .lookup_data
                .xor_big_sigma0_1
                .par_iter()
                .map(|v| v.as_slice()),
        );
        let (xor_big_sigma1_claim, xor_big_sigma1_trace, xor_big_sigma1_interaction_claim_data) =
            xor_big_sigma1::Claim::write_trace(
                sha256_interaction_claim_data
                    .lookup_data
                    .xor_big_sigma1
                    .par_iter()
                    .map(|v| v.as_slice()),
            );

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

        // Write bitwise components
        let bitwise_data = opcodes_interaction_claim_data.bitwise();
        let (bitwise_claim, bitwise_trace, bitwise_interaction_claim_data) =
            bitwise::Claim::write_trace(bitwise_data);

        // Gather all lookup data
        let interaction_claim_data = InteractionClaimData {
            opcodes: opcodes_interaction_claim_data,
            memory: memory_interaction_claim_data,
            merkle: merkle_interaction_claim_data,
            clock_update: clock_update_interaction_claim_data,
            poseidon2: poseidon2_interaction_claim_data,
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
            range_check_8: range_check_8_interaction_claim_data,
            range_check_16: range_check_16_interaction_claim_data,
            range_check_20: range_check_20_interaction_claim_data,
            bitwise: bitwise_interaction_claim_data,
        };

        // Combine all traces
        let trace = opcodes_trace
            .into_iter()
            .chain(memory_trace.to_evals())
            .chain(merkle_trace.to_evals())
            .chain(clock_update_trace.to_evals())
            .chain(poseidon2_trace.to_evals())
            .chain(sha256_trace.to_evals())
            .chain(ch_trace)
            .chain(maj_trace)
            .chain(small_sigma0_0_trace)
            .chain(small_sigma0_1_trace)
            .chain(small_sigma1_0_trace)
            .chain(small_sigma1_1_trace)
            .chain(big_sigma0_0_trace)
            .chain(big_sigma0_1_trace)
            .chain(big_sigma1_0_trace)
            .chain(big_sigma1_1_trace)
            .chain(xor_small_sigma0_trace)
            .chain(xor_small_sigma1_trace)
            .chain(xor_big_sigma0_0_trace)
            .chain(xor_big_sigma0_1_trace)
            .chain(xor_big_sigma1_trace)
            .chain(range_check_8_trace)
            .chain(range_check_16_trace)
            .chain(range_check_20_trace)
            .chain(bitwise_trace);

        (
            Self {
                opcodes: opcodes_claim,
                memory: memory_claim,
                merkle: merkle_claim,
                clock_update: clock_update_claim,
                poseidon2: poseidon2_claim,
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
                range_check_8: range_check_8_claim,
                range_check_16: range_check_16_claim,
                range_check_20: range_check_20_claim,
                bitwise: bitwise_claim,
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

        let (bitwise_interaction_claim, bitwise_interaction_trace) =
            bitwise::InteractionClaim::write_interaction_trace(
                &relations.bitwise,
                &interaction_claim_data.bitwise,
            );

        (
            opcodes_interaction_trace
                .into_iter()
                .chain(memory_interaction_trace)
                .chain(merkle_interaction_trace)
                .chain(clock_update_interaction_trace)
                .chain(poseidon2_interaction_trace)
                .chain(sha256_interaction_trace)
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
                .chain(range_check_8_interaction_trace)
                .chain(range_check_16_interaction_trace)
                .chain(range_check_20_interaction_trace)
                .chain(bitwise_interaction_trace),
            Self {
                opcodes: opcodes_interaction_claim,
                memory: memory_interaction_claim,
                merkle: merkle_interaction_claim,
                clock_update: clock_update_interaction_claim,
                poseidon2: poseidon2_interaction_claim,
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
                range_check_8: range_check_8_interaction_claim,
                range_check_16: range_check_16_interaction_claim,
                range_check_20: range_check_20_interaction_claim,
                bitwise: bitwise_interaction_claim,
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
        sum += self.range_check_8.claimed_sum;
        sum += self.range_check_16.claimed_sum;
        sum += self.range_check_20.claimed_sum;
        sum += self.bitwise.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.memory.mix_into(channel);
        self.merkle.mix_into(channel);
        self.clock_update.mix_into(channel);
        self.poseidon2.mix_into(channel);
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
        self.range_check_8.mix_into(channel);
        self.range_check_16.mix_into(channel);
        self.range_check_20.mix_into(channel);
        self.bitwise.mix_into(channel);
    }
}

impl Relations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            registers: relations::Registers::draw(channel),
            memory: relations::Memory::draw(channel),
            merkle: relations::Merkle::draw(channel),
            poseidon2: relations::Poseidon2::draw(channel),
            xor_small_sigma0: relations::XorSmallSigma0::draw(channel),
            xor_small_sigma1: relations::XorSmallSigma1::draw(channel),
            xor_big_sigma0_0: relations::XorBigSigma0_0::draw(channel),
            xor_big_sigma0_1: relations::XorBigSigma0_1::draw(channel),
            xor_big_sigma1: relations::XorBigSigma1::draw(channel),
            ch: relations::Ch::draw(channel),
            maj: relations::Maj::draw(channel),
            small_sigma0_0: relations::SmallSigma0_0::draw(channel),
            small_sigma0_1: relations::SmallSigma0_1::draw(channel),
            small_sigma1_0: relations::SmallSigma1_0::draw(channel),
            small_sigma1_1: relations::SmallSigma1_1::draw(channel),
            big_sigma0_0: relations::BigSigma0_0::draw(channel),
            big_sigma0_1: relations::BigSigma0_1::draw(channel),
            big_sigma1_0: relations::BigSigma1_0::draw(channel),
            big_sigma1_1: relations::BigSigma1_1::draw(channel),
            range_check_8: relations::RangeCheck8::draw(channel),
            range_check_16: relations::RangeCheck16::draw(channel),
            range_check_20: relations::RangeCheck20::draw(channel),
            bitwise: relations::Bitwise::draw(channel),
        }
    }
}

pub struct Components {
    pub opcodes: opcodes::Component,
    pub memory: memory::Component,
    pub merkle: merkle::Component,
    pub clock_update: clock_update::Component,
    pub poseidon2: poseidon2::Component,
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
    pub range_check_8: range_check_8::Component,
    pub range_check_16: range_check_16::Component,
    pub range_check_20: range_check_20::Component,
    pub bitwise: bitwise::Component,
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
            bitwise: bitwise::Component::new(
                location_allocator,
                bitwise::Eval {
                    claim: claim.bitwise,
                    relation: relations.bitwise.clone(),
                    claimed_sum: interaction_claim.bitwise.claimed_sum,
                },
                interaction_claim.bitwise.claimed_sum,
            ),
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        let mut provers = self.opcodes.provers();
        provers.push(&self.memory);
        provers.push(&self.merkle);
        provers.push(&self.clock_update);
        provers.push(&self.poseidon2);
        provers.push(&self.sha256);
        provers.push(&self.ch);
        provers.push(&self.maj);
        provers.push(&self.small_sigma0_0);
        provers.push(&self.small_sigma0_1);
        provers.push(&self.small_sigma1_0);
        provers.push(&self.small_sigma1_1);
        provers.push(&self.big_sigma0_0);
        provers.push(&self.big_sigma0_1);
        provers.push(&self.big_sigma1_0);
        provers.push(&self.big_sigma1_1);
        provers.push(&self.xor_small_sigma0);
        provers.push(&self.xor_small_sigma1);
        provers.push(&self.xor_big_sigma0_0);
        provers.push(&self.xor_big_sigma0_1);
        provers.push(&self.xor_big_sigma1);
        provers.push(&self.range_check_8);
        provers.push(&self.range_check_16);
        provers.push(&self.range_check_20);
        provers.push(&self.bitwise);
        provers
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        let mut verifiers = self.opcodes.verifiers();
        verifiers.push(&self.memory);
        verifiers.push(&self.merkle);
        verifiers.push(&self.clock_update);
        verifiers.push(&self.poseidon2);
        verifiers.push(&self.sha256);
        verifiers.push(&self.ch);
        verifiers.push(&self.maj);
        verifiers.push(&self.small_sigma0_0);
        verifiers.push(&self.small_sigma0_1);
        verifiers.push(&self.small_sigma1_0);
        verifiers.push(&self.small_sigma1_1);
        verifiers.push(&self.big_sigma0_0);
        verifiers.push(&self.big_sigma0_1);
        verifiers.push(&self.big_sigma1_0);
        verifiers.push(&self.big_sigma1_1);
        verifiers.push(&self.xor_small_sigma0);
        verifiers.push(&self.xor_small_sigma1);
        verifiers.push(&self.xor_big_sigma0_0);
        verifiers.push(&self.xor_big_sigma0_1);
        verifiers.push(&self.xor_big_sigma1);
        verifiers.push(&self.range_check_8);
        verifiers.push(&self.range_check_16);
        verifiers.push(&self.range_check_20);
        verifiers.push(&self.bitwise);
        verifiers
    }
}
