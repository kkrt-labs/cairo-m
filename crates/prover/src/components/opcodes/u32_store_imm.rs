//! This component is used to prove the U32StoreImm opcode.
//! u32([fp + dst_off], [fp + dst_off + 1]) = u32(imm_lo, imm_hi)
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - imm_lo
//! - imm_hi
//! - dst_off
//! - dst_prev_val_lo
//! - dst_prev_val_hi
//! - dst_prev_clock_lo
//! - dst_prev_clock_hi
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * registers update is regular
//!   * `- [pc, fp, clock] + [pc + 1, fp, clock + 1]` in `Registers` relation
//! * read instruction from memory
//!   * `- [pc, inst_prev_clk, opcode_constant, imm_lo, imm_hi, dst_off] + [pc, clk, opcode_constant, imm_lo, imm_hi, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clk, dst_prev_val_lo] + [fp + dst_off, clk, imm_lo]` in `Memory` Relation
//!   * `- [fp + dst_off + 1, dst_prev_clk, dst_prev_val_hi] + [fp + dst_off + 1, clk, imm_hi]` in `Memory` Relation
//!   * `- [clk - dst_prev_clk - 1]` in `RangeCheck20` relation
//! * limbs of each U32 must be in range [0, 2^16)
//!   * `- [imm_lo]` in `RangeCheck16` relation
//!   * `- [imm_hi]` in `RangeCheck16` relation

use cairo_m_common::instruction::U32_STORE_IMM;
use num_traits::{One, Zero};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use rayon::slice::ParallelSlice;
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SecureField, SECURE_EXTENSION_DEGREE};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::adapter::memory::DataAccess;
use crate::adapter::ExecutionBundle;
use crate::components::Relations;
use crate::preprocessed::bitwise::BitwiseProvider;
use crate::preprocessed::range_check::RangeCheckProvider;
use crate::utils::data_accesses::{get_prev_clock, get_prev_value};
use crate::utils::enabler::Enabler;
use crate::utils::execution_bundle::PackedExecutionBundle;

const N_TRACE_COLUMNS: usize = 12;
const N_MEMORY_LOOKUPS: usize = 6;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 3;
const N_RANGE_CHECK_16_LOOKUPS: usize = 2;

const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_MEMORY_LOOKUPS
        + N_REGISTERS_LOOKUPS
        + N_RANGE_CHECK_20_LOOKUPS
        + N_RANGE_CHECK_16_LOOKUPS)
        .div_ceil(2);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

impl RangeCheckProvider for InteractionClaimData {
    fn get_range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_20.par_iter().flatten()
    }

    fn get_range_check_16(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_16.par_iter().flatten()
    }
}

impl BitwiseProvider for InteractionClaimData {}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub registers: [Vec<[PackedM31; 3]>; N_REGISTERS_LOOKUPS],
    pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
    pub range_check_16: [Vec<PackedM31>; N_RANGE_CHECK_16_LOOKUPS],
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_trace = vec![self.log_size; N_LOOKUPS_COLUMNS];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }

    /// Writes the trace for the U32StoreImm opcode.
    ///
    /// # Important
    /// This function filters the inputs and creates a local vector which is cleared after processing.
    /// The local vector's capacity is preserved but its length is set to 0.
    /// This is done to free memory during proof generation as the filtered inputs are no longer needed
    /// after being packed into SIMD-friendly format.
    pub fn write_trace<MC: MerkleChannel>(
        inputs: &mut Vec<ExecutionBundle>,
        data_accesses: &[DataAccess],
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let non_padded_length = inputs.len();
        let log_size = std::cmp::max(LOG_N_LANES, inputs.len().next_power_of_two().ilog2());

        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
                LookupData::uninitialized(log_size - LOG_N_LANES),
            )
        };
        inputs.resize(1 << log_size, ExecutionBundle::default());
        let packed_inputs: Vec<PackedExecutionBundle> = inputs
            .par_chunks_exact(N_LANES)
            .map(|chunk| {
                let array: [ExecutionBundle; N_LANES] = chunk.try_into().unwrap();
                Pack::pack(array)
            })
            .collect();
        // Clear the inputs to free memory early. The data has been packed into SIMD format
        // and the original inputs are no longer needed. This reduces memory pressure during
        // proof generation. Note: this preserves the vector's capacity for potential reuse.
        inputs.clear();
        inputs.shrink_to_fit();

        let zero = PackedM31::from(M31::zero());
        let one = PackedM31::from(M31::one());
        let enabler_col = Enabler::new(non_padded_length);
        (
            trace.par_iter_mut(),
            packed_inputs.par_iter(),
            lookup_data.par_iter_mut(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(row_index, (mut row, input, lookup_data))| {
                let enabler = enabler_col.packed_at(row_index);
                let pc = input.pc;
                let fp = input.fp;
                let clock = input.clock;
                let inst_prev_clock = input.inst_prev_clock;

                let opcode_constant = PackedM31::from(M31::from(U32_STORE_IMM));
                let imm_lo = input.inst_value_1;
                let imm_hi = input.inst_value_2;
                let dst_off = input.inst_value_3;

                let dst_prev_clock_lo = get_prev_clock(input, data_accesses, 0);
                let dst_prev_val_lo = get_prev_value(input, data_accesses, 0);
                let dst_prev_clock_hi = get_prev_clock(input, data_accesses, 1);
                let dst_prev_val_hi = get_prev_value(input, data_accesses, 1);

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = imm_lo;
                *row[6] = imm_hi;
                *row[7] = dst_off;
                *row[8] = dst_prev_val_lo;
                *row[9] = dst_prev_val_hi;
                *row[10] = dst_prev_clock_lo;
                *row[11] = dst_prev_clock_hi;

                *lookup_data.registers[0] = [input.pc, input.fp, input.clock];
                *lookup_data.registers[1] = [input.pc + one, input.fp, input.clock + one];

                // Read instruction
                *lookup_data.memory[0] = [
                    input.pc,
                    inst_prev_clock,
                    opcode_constant,
                    imm_lo,
                    imm_hi,
                    dst_off,
                ];
                *lookup_data.memory[1] =
                    [input.pc, clock, opcode_constant, imm_lo, imm_hi, dst_off];

                // Write dst_lo
                *lookup_data.memory[2] = [
                    fp + dst_off,
                    dst_prev_clock_lo,
                    dst_prev_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[3] = [fp + dst_off, clock, imm_lo, zero, zero, zero];

                // Write dst_hi
                *lookup_data.memory[4] = [
                    fp + dst_off + one,
                    dst_prev_clock_hi,
                    dst_prev_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[5] = [fp + dst_off + one, clock, imm_hi, zero, zero, zero];

                // Limbs of each U32 must be in range [0, 2^16)
                *lookup_data.range_check_16[0] = imm_lo;
                *lookup_data.range_check_16[1] = imm_hi;

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - dst_prev_clock_lo - enabler;
                *lookup_data.range_check_20[2] = clock - dst_prev_clock_hi - enabler;
            });

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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
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
            &interaction_claim_data.lookup_data.registers[0],
            &interaction_claim_data.lookup_data.registers[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, registers_prev, registers_new))| {
                let num_prev = -PackedQM31::from(enabler_col.packed_at(i));
                let num_new = PackedQM31::from(enabler_col.packed_at(i));
                let denom_prev: PackedQM31 = relations.registers.combine(registers_prev);
                let denom_new: PackedQM31 = relations.registers.combine(registers_new);

                let numerator = num_prev * denom_new + num_new * denom_prev;
                let denom = denom_prev * denom_new;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[0],
            &interaction_claim_data.lookup_data.memory[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, memory_prev, memory_new))| {
                let num_prev = -PackedQM31::from(enabler_col.packed_at(i));
                let num_new = PackedQM31::from(enabler_col.packed_at(i));
                let denom_prev: PackedQM31 = relations.memory.combine(memory_prev);
                let denom_new: PackedQM31 = relations.memory.combine(memory_new);

                let numerator = num_prev * denom_new + num_new * denom_prev;
                let denom = denom_prev * denom_new;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[2],
            &interaction_claim_data.lookup_data.memory[3],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, memory_prev, memory_new))| {
                let num_prev = -PackedQM31::from(enabler_col.packed_at(i));
                let num_new = PackedQM31::from(enabler_col.packed_at(i));
                let denom_prev: PackedQM31 = relations.memory.combine(memory_prev);
                let denom_new: PackedQM31 = relations.memory.combine(memory_new);

                let numerator = num_prev * denom_new + num_new * denom_prev;
                let denom = denom_prev * denom_new;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[4],
            &interaction_claim_data.lookup_data.memory[5],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, memory_prev, memory_new))| {
                let num_prev = -PackedQM31::from(enabler_col.packed_at(i));
                let num_new = PackedQM31::from(enabler_col.packed_at(i));
                let denom_prev: PackedQM31 = relations.memory.combine(memory_prev);
                let denom_new: PackedQM31 = relations.memory.combine(memory_new);

                let numerator = num_prev * denom_new + num_new * denom_prev;
                let denom = denom_prev * denom_new;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_16[0],
            &interaction_claim_data.lookup_data.range_check_16[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_16_0, range_check_16_1))| {
                let num = -PackedQM31::one();
                let denom_0: PackedQM31 = relations.range_check_16.combine(&[*range_check_16_0]);
                let denom_1: PackedQM31 = relations.range_check_16.combine(&[*range_check_16_1]);

                let numerator = num * denom_1 + num * denom_0;
                let denom = denom_0 * denom_1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_20[0],
            &interaction_claim_data.lookup_data.range_check_20[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_20_0, range_check_20_1))| {
                let num = -PackedQM31::one();
                let denom_0: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_0]);
                let denom_1: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_1]);

                let numerator = num * denom_1 + num * denom_0;
                let denom = denom_0 * denom_1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_20[2],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_20_2))| {
                let num = -PackedQM31::one();
                let denom: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_2]);
                let numerator = num * denom;
                let denom = denom * denom;
                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (Self { claimed_sum }, trace)
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
        let one = E::F::from(M31::one());
        let opcode_constant = E::F::from(M31::from(U32_STORE_IMM));

        // 12 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let imm_lo = eval.next_trace_mask();
        let imm_hi = eval.next_trace_mask();
        let dst_off = eval.next_trace_mask();
        let dst_prev_val_lo = eval.next_trace_mask();
        let dst_prev_val_hi = eval.next_trace_mask();
        let dst_prev_clock_lo = eval.next_trace_mask();
        let dst_prev_clock_hi = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Registers update
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone(), clock.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            E::EF::from(enabler.clone()),
            &[
                pc.clone() + one.clone(),
                fp.clone(),
                clock.clone() + one.clone(),
            ],
        ));

        // Read instruction
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                inst_prev_clock.clone(),
                opcode_constant.clone(),
                imm_lo.clone(),
                imm_hi.clone(),
                dst_off.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                pc,
                clock.clone(),
                opcode_constant,
                imm_lo.clone(),
                imm_hi.clone(),
                dst_off.clone(),
            ],
        ));

        // Write dst_lo
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + dst_off.clone(),
                dst_prev_clock_lo.clone(),
                dst_prev_val_lo,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + dst_off.clone(), clock.clone(), imm_lo.clone()],
        ));

        // Write dst_hi
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + dst_off.clone() + one.clone(),
                dst_prev_clock_hi.clone(),
                dst_prev_val_hi,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp + dst_off + one, clock.clone(), imm_hi.clone()],
        ));

        // Range check 16
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[imm_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[imm_hi],
        ));

        // Range check 20
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - inst_prev_clock - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - dst_prev_clock_lo - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock - dst_prev_clock_hi - enabler],
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
