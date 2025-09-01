//! This component is used to prove the U32StoreEqFpFp opcode.
//! [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) == u32([fp + src1_off], [fp + src1_off + 1])
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - src0_off
//! - src1_off
//! - dst_off
//! - op0_val_lo
//! - op0_val_hi
//! - op0_prev_clock
//! - op1_val_lo
//! - op1_val_hi
//! - op1_prev_clock
//! - diff_inv
//!
//! # Constraints
//!
//! diff = op1_val_lo + op1_val_hi * 2^16 - op0_val_lo - op0_val_hi * 2^16
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * registers update is regular
//!   * `- [pc, fp] + [pc + 1, fp]` in `Registers` relation
//! * read instruction from memory
//!   * `- [pc, inst_prev_clk, opcode_constant, src0_off, src1_off, dst_off] + [pc, clk, opcode_constant, src0_off, src1_off, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read op0
//!   * `- [fp + src0_off, op0_prev_clk, op0_val_lo] + [fp + src0_off, clk, op0_val_lo]`
//!   * `- [fp + src0_off + 1, op0_prev_clk, op0_val_hi] + [fp + src0_off + 1, clk, op0_val_hi]`
//!   * `- [clk - op0_prev_clk - 1]` in `RangeCheck20` relation
//! * read op1
//!   * `- [fp + src1_off, op1_prev_clk, op1_val_lo] + [fp + src1_off, clk, op1_val_lo]`
//!   * `- [fp + src1_off + 1, op1_prev_clk, op1_val_hi] + [fp + src1_off + 1, clk, op1_val_hi]`
//!   * `- [clk - op1_prev_clk - 1]` in `RangeCheck20` relation
//! * diff_inv is the inverse of diff or diff is 0
//!   * `- diff * (diff_inv * diff - 1)`
//! * diff_inv is the inverse of diff or diff_inv is 0
//!   * `- diff_inv * (diff_inv * diff - 1)`
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clk, dst_prev_val] + [fp + dst_off, clk, one - diff * diff_inv]` in `Memory` Relation
//!   * `- [clk - dst_prev_clk - 1]` in `RangeCheck20` relation
//! * limbs of each U32 must be in range [0, 2^16)
//!   * `- [op0_val_lo]` in `RangeCheck16` relation
//!   * `- [op0_val_hi]` in `RangeCheck16` relation
//!   * `- [op1_val_lo]` in `RangeCheck16` relation
//!   * `- [op1_val_hi]` in `RangeCheck16` relation

use cairo_m_common::instruction::U32_STORE_EQ_FP_FP;
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

use crate::adapter::ExecutionBundle;
use crate::components::Relations;
use crate::preprocessed::range_check::RangeCheckProvider;
use crate::utils::enabler::Enabler;
use crate::utils::execution_bundle::PackedExecutionBundle;

const N_TRACE_COLUMNS: usize = 19;
const N_MEMORY_LOOKUPS: usize = 12;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 6;
const N_RANGE_CHECK_16_LOOKUPS: usize = 4;

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
    fn get_range_check_16(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_16.par_iter().flatten()
    }
    fn get_range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_20.par_iter().flatten()
    }
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
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

    /// Writes the trace for the U32StoreEqFpFp opcode.
    ///
    /// # Important
    /// This function filters the inputs and creates a local vector which is cleared after processing.
    /// The local vector's capacity is preserved but its length is set to 0.
    /// This is done to free memory during proof generation as the filtered inputs are no longer needed
    /// after being packed into SIMD-friendly format.
    pub fn write_trace<MC: MerkleChannel>(
        inputs: &mut Vec<ExecutionBundle>,
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
        let two_pow_16 = PackedM31::from(M31::from(1 << 16));
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

                let opcode_constant = PackedM31::from(M31::from(U32_STORE_EQ_FP_FP));
                let src0_off = input.inst_value_1;
                let src1_off = input.inst_value_2;
                let dst_off = input.inst_value_4;

                let op0_val_lo = input.mem1_value;
                let op0_val_hi = input.mem2_value;
                let op0_prev_lo_clock = input.mem1_prev_clock;
                let op0_prev_hi_clock = input.mem2_prev_clock;

                let op1_val_lo = input.mem3_value;
                let op1_val_hi = input.mem4_value;
                let op1_prev_lo_clock = input.mem3_prev_clock;
                let op1_prev_hi_clock = input.mem4_prev_clock;

                let dst_prev_val = input.mem5_prev_value;
                let dst_prev_clock = input.mem5_prev_clock;

                let diff =
                    op1_val_lo + op1_val_hi * two_pow_16 - op0_val_lo - op0_val_hi * two_pow_16;
                let diff_inv = PackedM31::from_array(diff.to_array().map(|x| {
                    if x.0 != 0 {
                        x.inverse()
                    } else {
                        M31::zero()
                    }
                }));

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = src0_off;
                *row[6] = src1_off;
                *row[7] = dst_off;
                *row[8] = op0_val_lo;
                *row[9] = op0_val_hi;
                *row[10] = op0_prev_lo_clock;
                *row[11] = op0_prev_hi_clock;
                *row[12] = op1_val_lo;
                *row[13] = op1_val_hi;
                *row[14] = op1_prev_lo_clock;
                *row[15] = op1_prev_hi_clock;
                *row[16] = dst_prev_val;
                *row[17] = dst_prev_clock;
                *row[18] = diff_inv;

                *lookup_data.registers[0] = [input.pc, input.fp];
                *lookup_data.registers[1] = [input.pc + one, input.fp];

                // Read instruction
                *lookup_data.memory[0] = [
                    input.pc,
                    inst_prev_clock,
                    opcode_constant,
                    src0_off,
                    src1_off,
                    dst_off,
                ];
                *lookup_data.memory[1] = [
                    input.pc,
                    clock,
                    opcode_constant,
                    src0_off,
                    src1_off,
                    dst_off,
                ];

                // Read op0_lo
                *lookup_data.memory[2] = [
                    fp + src0_off,
                    op0_prev_lo_clock,
                    op0_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[3] = [fp + src0_off, clock, op0_val_lo, zero, zero, zero];

                // Read op0_hi
                *lookup_data.memory[4] = [
                    fp + src0_off + one,
                    op0_prev_hi_clock,
                    op0_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[5] = [fp + src0_off + one, clock, op0_val_hi, zero, zero, zero];

                // Read op1_lo
                *lookup_data.memory[6] = [
                    fp + src1_off,
                    op1_prev_lo_clock,
                    op1_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[7] = [fp + src1_off, clock, op1_val_lo, zero, zero, zero];

                // Read op1_hi
                *lookup_data.memory[8] = [
                    fp + src1_off + one,
                    op1_prev_hi_clock,
                    op1_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[9] = [fp + src1_off + one, clock, op1_val_hi, zero, zero, zero];

                // Write dst
                *lookup_data.memory[10] =
                    [fp + dst_off, dst_prev_clock, dst_prev_val, zero, zero, zero];
                *lookup_data.memory[11] =
                    [fp + dst_off, clock, one - diff * diff_inv, zero, zero, zero];

                // Range checks for U32 limbs
                *lookup_data.range_check_16[0] = op0_val_lo;
                *lookup_data.range_check_16[1] = op0_val_hi;
                *lookup_data.range_check_16[2] = op1_val_lo;
                *lookup_data.range_check_16[3] = op1_val_hi;

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - op0_prev_lo_clock - enabler;
                *lookup_data.range_check_20[2] = clock - op0_prev_hi_clock - enabler;
                *lookup_data.range_check_20[3] = clock - op1_prev_lo_clock - enabler;
                *lookup_data.range_check_20[4] = clock - op1_prev_hi_clock - enabler;
                *lookup_data.range_check_20[5] = clock - dst_prev_clock - enabler;
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

        // Registers lookups
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

        // Memory lookups
        for i in 0..N_MEMORY_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.memory[i * 2],
                &interaction_claim_data.lookup_data.memory[i * 2 + 1],
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
        }

        // Range check 16 lookups
        for i in 0..N_RANGE_CHECK_16_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.range_check_16[i * 2],
                &interaction_claim_data.lookup_data.range_check_16[i * 2 + 1],
            )
                .into_par_iter()
                .enumerate()
                .for_each(|(_i, (writer, range_check_16_0, range_check_16_1))| {
                    let num = -PackedQM31::one();
                    let denom_0: PackedQM31 =
                        relations.range_check_16.combine(&[*range_check_16_0]);
                    let denom_1: PackedQM31 =
                        relations.range_check_16.combine(&[*range_check_16_1]);

                    let numerator = num * denom_1 + num * denom_0;
                    let denom = denom_0 * denom_1;

                    writer.write_frac(numerator, denom);
                });
            col.finalize_col();
        }

        // Range check 20 lookups
        for i in 0..N_RANGE_CHECK_20_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.range_check_20[i * 2],
                &interaction_claim_data.lookup_data.range_check_20[i * 2 + 1],
            )
                .into_par_iter()
                .enumerate()
                .for_each(|(_i, (writer, range_check_20_0, range_check_20_1))| {
                    let num = -PackedQM31::one();
                    let denom_0: PackedQM31 =
                        relations.range_check_20.combine(&[*range_check_20_0]);
                    let denom_1: PackedQM31 =
                        relations.range_check_20.combine(&[*range_check_20_1]);

                    let numerator = num * denom_1 + num * denom_0;
                    let denom = denom_0 * denom_1;

                    writer.write_frac(numerator, denom);
                });
            col.finalize_col();
        }

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
        let two_pow_16 = E::F::from(M31::from(1 << 16));
        let opcode_constant = E::F::from(M31::from(U32_STORE_EQ_FP_FP));

        // 19 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let src0_off = eval.next_trace_mask();
        let src1_off = eval.next_trace_mask();
        let dst_off = eval.next_trace_mask();
        let op0_val_lo = eval.next_trace_mask();
        let op0_val_hi = eval.next_trace_mask();
        let op0_prev_lo_clock = eval.next_trace_mask();
        let op0_prev_hi_clock = eval.next_trace_mask();
        let op1_val_lo = eval.next_trace_mask();
        let op1_val_hi = eval.next_trace_mask();
        let op1_prev_lo_clock = eval.next_trace_mask();
        let op1_prev_hi_clock = eval.next_trace_mask();
        let dst_prev_val = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();
        let diff_inv = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        let diff = op1_val_lo.clone() + op1_val_hi.clone() * two_pow_16.clone()
            - op0_val_lo.clone()
            - op0_val_hi.clone() * two_pow_16;

        // diff_inv is the inverse of diff or diff is 0
        eval.add_constraint(diff.clone() * (diff_inv.clone() * diff.clone() - one.clone()));

        // diff_inv is the inverse of diff or diff_inv is 0
        eval.add_constraint(diff_inv.clone() * (diff_inv.clone() * diff.clone() - one.clone()));

        // Registers update
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            E::EF::from(enabler.clone()),
            &[pc.clone() + one.clone(), fp.clone()],
        ));

        // Read instruction from memory
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                inst_prev_clock.clone(),
                opcode_constant.clone(),
                src0_off.clone(),
                src1_off.clone(),
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
                src0_off.clone(),
                src1_off.clone(),
                dst_off.clone(),
            ],
        ));

        // Read op0_lo
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src0_off.clone(),
                op0_prev_lo_clock.clone(),
                op0_val_lo.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src0_off.clone(),
                clock.clone(),
                op0_val_lo.clone(),
            ],
        ));

        // Read op0_hi
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src0_off.clone() + one.clone(),
                op0_prev_hi_clock.clone(),
                op0_val_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src0_off + one.clone(),
                clock.clone(),
                op0_val_hi.clone(),
            ],
        ));

        // Read op1_lo
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src1_off.clone(),
                op1_prev_lo_clock.clone(),
                op1_val_lo.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src1_off.clone(),
                clock.clone(),
                op1_val_lo.clone(),
            ],
        ));

        // Read op1_hi
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src1_off.clone() + one.clone(),
                op1_prev_hi_clock.clone(),
                op1_val_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src1_off + one.clone(),
                clock.clone(),
                op1_val_hi.clone(),
            ],
        ));

        // Write dst
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + dst_off.clone(),
                dst_prev_clock.clone(),
                dst_prev_val,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp + dst_off, clock.clone(), one - diff * diff_inv],
        ));

        // Range check 16
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[op0_val_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[op0_val_hi],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[op1_val_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[op1_val_hi],
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
            &[clock.clone() - op0_prev_lo_clock - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - op0_prev_hi_clock - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - op1_prev_lo_clock - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - op1_prev_hi_clock - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock - dst_prev_clock - enabler],
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
