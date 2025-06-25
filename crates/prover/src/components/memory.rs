use num_traits::Zero;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::adapter::memory::Memory;
use crate::relations;

const N_M31_IN_MEMORY_ENTRY: usize = 7; // Address, value, clock, multiplicity

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Claim {
    pub log_size: u32,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct InteractionClaimData {
    pub memory: Vec<[PackedM31; N_M31_IN_MEMORY_ENTRY]>,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N_M31_IN_MEMORY_ENTRY];
        let interaction_log_sizes = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<MC: MerkleChannel>(
        inputs: &Memory,
    ) -> (
        Self,
        ComponentTrace<N_M31_IN_MEMORY_ENTRY>,
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let initial_memory_len = inputs.initial_memory.len();
        let log_size = std::cmp::max(
            (initial_memory_len + inputs.final_memory.len()).next_power_of_two(),
            N_LANES,
        )
        .ilog2();

        // Pack memory entries from the prover input
        let packed_inputs: Vec<[PackedM31; N_M31_IN_MEMORY_ENTRY]> = inputs
            .initial_memory
            .iter()
            .chain(inputs.final_memory.iter())
            .enumerate()
            .map(|(i, (address, (value, clock)))| {
                let value_array = value.to_m31_array();
                let mult = if i < initial_memory_len {
                    M31::from(1)
                } else {
                    M31::from(-1)
                };
                [
                    *address,
                    *clock,
                    value_array[0],
                    value_array[1],
                    value_array[2],
                    value_array[3],
                    mult,
                ]
            })
            .chain(std::iter::repeat([M31::zero(); N_M31_IN_MEMORY_ENTRY]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        // Generate lookup data and fill the trace
        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<N_M31_IN_MEMORY_ENTRY>::uninitialized(log_size),
                InteractionClaimData::uninitialized(log_size - LOG_N_LANES),
            )
        };
        (
            trace.par_iter_mut(),
            packed_inputs.into_par_iter(),
            lookup_data.memory.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(|(mut row, input, lookup_memory)| {
                *row[0] = input[0];
                *row[1] = input[1];
                *row[2] = input[2];
                *row[3] = input[3];
                *row[4] = input[4];
                *row[5] = input[5];
                *row[6] = input[6];
                *lookup_memory = input;
            });

        // Return the trace and lookup data
        (Self { log_size }, trace, lookup_data)
    }
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        memory: &relations::Memory,
        lookup_data: &InteractionClaimData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = lookup_data.memory.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.memory)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = memory.combine(&value[..6]);
                let mult: PackedQM31 = PackedQM31::from(value[6]);
                writer.write_frac(mult, denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, Self { claimed_sum })
    }
}

pub struct Eval {
    pub claim: Claim,
    pub memory: relations::Memory,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let memory_entries: [E::F; N_M31_IN_MEMORY_ENTRY - 1] =
            std::array::from_fn(|_| eval.next_trace_mask());
        let multiplicities: E::F = eval.next_trace_mask();

        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(multiplicities),
            &memory_entries,
        ));

        eval.finalize_logup();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
