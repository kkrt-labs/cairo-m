use std::simd::Simd;

use itertools::{chain, Itertools};
use num_traits::One;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Column};
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::adapter::memory::MemoryBoundaries;
use crate::relations;

const N_M31_IN_MEMORY_ENTRY: usize = 6;

#[derive(Clone, Default)]
pub struct Claim {
    pub inputs: MemoryBoundaries,
    pub log_size: u32,
}

#[derive(Copy, Clone)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

pub struct LookupData {
    pub initial_memory: Vec<[PackedM31; N_M31_IN_MEMORY_ENTRY]>,
    pub final_memory: Vec<[PackedM31; N_M31_IN_MEMORY_ENTRY]>,
}

impl Claim {
    pub fn new(memory_boundaries: MemoryBoundaries) -> Self {
        let column_length = std::cmp::max(
            std::cmp::max(
                memory_boundaries.initial_memory.len(),
                memory_boundaries.final_memory.len(),
            )
            .next_power_of_two(),
            N_LANES,
        );
        let log_size = column_length.ilog2();

        Self {
            log_size,
            inputs: memory_boundaries,
        }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; 2 * N_M31_IN_MEMORY_ENTRY];
        let interaction_log_sizes = vec![self.log_size; SECURE_EXTENSION_DEGREE * 2];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<MC: MerkleChannel>(
        &mut self,
    ) -> (
        Vec<CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Pack memory entries from the prover input
        let initial_memory: Vec<[PackedM31; N_M31_IN_MEMORY_ENTRY]> = self
            .inputs
            .initial_memory
            .iter()
            .map(|(address, value, clock)| {
                [*address, value[0], value[1], value[2], value[3], *clock]
            })
            .chain(std::iter::repeat([0u32; N_M31_IN_MEMORY_ENTRY]))
            .take(1 << self.log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| unsafe {
                    PackedM31::from_simd_unchecked(Simd::from_array(std::array::from_fn(|y| {
                        chunk[y][x]
                    })))
                })
            })
            .collect();
        let final_memory: Vec<[PackedM31; N_M31_IN_MEMORY_ENTRY]> = self
            .inputs
            .final_memory
            .iter()
            .map(|(address, value, clock)| {
                [*address, value[0], value[1], value[2], value[3], *clock]
            })
            .chain(std::iter::repeat([0u32; N_M31_IN_MEMORY_ENTRY]))
            .take(1 << self.log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| unsafe {
                    PackedM31::from_simd_unchecked(Simd::from_array(std::array::from_fn(|y| {
                        chunk[y][x]
                    })))
                })
            })
            .collect();

        // Generate lookup data and fill the trace
        let mut loookup_data_initial_memory = Vec::new();
        let mut loookup_data_final_memory = Vec::new();

        let mut trace_initial_memory =
            std::iter::repeat_with(|| BaseColumn::zeros(1 << self.log_size))
                .take(N_M31_IN_MEMORY_ENTRY)
                .collect_vec();
        for (i, values) in initial_memory.iter().enumerate() {
            for (j, value) in values.iter().enumerate() {
                trace_initial_memory[j].data[i] = *value;
            }
            loookup_data_initial_memory.push(*values);
        }

        let mut trace_final_memory =
            std::iter::repeat_with(|| BaseColumn::zeros(1 << self.log_size))
                .take(N_M31_IN_MEMORY_ENTRY)
                .collect_vec();
        for (i, values) in final_memory.iter().enumerate() {
            for (j, value) in values.iter().enumerate() {
                trace_final_memory[j].data[i] = *value;
            }
            loookup_data_final_memory.push(*values);
        }

        // Return the trace and lookup data
        (
            chain!(
                trace_initial_memory.into_iter().map(|eval| {
                    CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                        CanonicCoset::new(self.log_size).circle_domain(),
                        eval,
                    )
                }),
                trace_final_memory.into_iter().map(|eval| {
                    CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                        CanonicCoset::new(self.log_size).circle_domain(),
                        eval,
                    )
                })
            )
            .collect_vec(),
            LookupData {
                initial_memory: loookup_data_initial_memory,
                final_memory: loookup_data_final_memory,
            },
        )
    }
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        memory: &relations::Memory,
        lookup_data: &LookupData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = std::cmp::max(
            lookup_data.initial_memory.len().ilog2(),
            lookup_data.final_memory.len().ilog2(),
        ) + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.initial_memory)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = memory.combine(value);
                writer.write_frac(-PackedQM31::one(), denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.final_memory)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = memory.combine(value);
                writer.write_frac(PackedQM31::one(), denom);
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
        let initial_memory_entries: [E::F; N_M31_IN_MEMORY_ENTRY] =
            std::array::from_fn(|_| eval.next_trace_mask());
        let final_memory_entries: [E::F; N_M31_IN_MEMORY_ENTRY] =
            std::array::from_fn(|_| eval.next_trace_mask());

        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::one(),
            &initial_memory_entries,
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::one(),
            &final_memory_entries,
        ));

        eval.finalize_logup();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
