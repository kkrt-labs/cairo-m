//! Component to add intermediate values for large clock diffs.
//!
//! # Columns
//!
//! - enabler
//! - addr
//! - prev_clk
//! - QM31 value
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * update the clock
//!   * `- [addr, prev_clk, value]` in `Memory` relation
//!   * `+ [addr, prev_clk + RC_20, value]` in `Memory` relation

use num_traits::{One, Zero};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::simd::m31::{LOG_N_LANES, N_LANES, PackedM31};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{QM31, SECURE_EXTENSION_DEGREE, SecureField};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::adapter::memory::RC20_LIMIT;
use crate::components::Relations;
use crate::utils::enabler::Enabler;

const N_TRACE_COLUMNS: usize = 7;
const N_MEMORY_LOOKUPS: usize = 2;
const N_INTERACTION_COLUMNS: usize = SECURE_EXTENSION_DEGREE * N_MEMORY_LOOKUPS.div_ceil(2);

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_log_sizes = vec![self.log_size; N_INTERACTION_COLUMNS];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<MC: MerkleChannel>(
        clock_update_data: &[(M31, M31, QM31)],
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let non_padded_length = clock_update_data.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack entries from the prover input
        let packed_inputs: Vec<[PackedM31; N_TRACE_COLUMNS - 1]> = clock_update_data
            .iter()
            .map(|&(addr, prev_clk, value)| {
                let value_array = value.to_m31_array();
                [
                    addr,
                    prev_clk,
                    value_array[0],
                    value_array[1],
                    value_array[2],
                    value_array[3],
                ]
            })
            .chain(std::iter::repeat([M31::zero(); N_TRACE_COLUMNS - 1]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let enabler_col = Enabler::new(non_padded_length);
        // Generate lookup data and fill the trace
        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
                LookupData::uninitialized(log_size - LOG_N_LANES),
            )
        };
        (
            trace.par_iter_mut(),
            packed_inputs.into_par_iter(),
            lookup_data.par_iter_mut(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(row_index, (mut row, input, lookup_data))| {
                let enabler = enabler_col.packed_at(row_index);
                let address = input[0];
                let prev_clk = input[1];
                let value0 = input[2];
                let value1 = input[3];
                let value2 = input[4];
                let value3 = input[5];

                *row[0] = enabler;
                *row[1] = address;
                *row[2] = prev_clk;
                *row[3] = value0;
                *row[4] = value1;
                *row[5] = value2;
                *row[6] = value3;

                *lookup_data.memory[0] = [address, prev_clk, value0, value1, value2, value3];
                *lookup_data.memory[1] = [
                    address,
                    prev_clk + PackedM31::broadcast(M31::from(RC20_LIMIT)),
                    value0,
                    value1,
                    value2,
                    value3,
                ];
            });

        // Return the trace and lookup data
        (
            Self { log_size },
            trace,
            InteractionClaimData {
                lookup_data,
                non_padded_length,
            },
        )
    }
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        relations: &Relations,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    ) {
        let log_size = interaction_claim_data.lookup_data.memory[0].len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[0],
            &interaction_claim_data.lookup_data.memory[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let num0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom0: PackedQM31 = relations.memory.combine(value0);
                let num1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.memory.combine(value1);

                let numerator = num0 * denom1 + num1 * denom0;
                let denom = denom0 * denom1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
    }
}

pub struct Eval {
    pub claim: Claim,
    pub relations: Relations,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let enabler = eval.next_trace_mask();
        let address = eval.next_trace_mask();
        let prev_clk = eval.next_trace_mask();
        let value0 = eval.next_trace_mask();
        let value1 = eval.next_trace_mask();
        let value2 = eval.next_trace_mask();
        let value3 = eval.next_trace_mask();

        let one = E::F::one();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one - enabler.clone()));

        // Update the clock
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                address.clone(),
                prev_clk.clone(),
                value0.clone(),
                value1.clone(),
                value2.clone(),
                value3.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler),
            &[
                address,
                prev_clk + E::F::from(M31::from(RC20_LIMIT)),
                value0,
                value1,
                value2,
                value3,
            ],
        ));

        eval.finalize_logup_in_pairs();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
