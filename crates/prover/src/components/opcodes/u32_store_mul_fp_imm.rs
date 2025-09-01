//! This component is used to prove the U32StoreMulFpImm opcode.
//! u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) * u32(imm_lo, imm_hi)
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - src_off
//! - imm_lo
//! - imm_hi
//! - dst_off
//! - op0_val_lo
//! - op0_val_hi
//! - op0_prev_clock
//! - dst_prev_val_lo
//! - dst_prev_val_hi
//! - dst_prev_clock
//! - res_lo
//! - res_hi
//! - p0_hi (p0_lo is actually res_lo)
//! - p1_lo
//! - p1_hi
//! - p2_lo
//! - p2_hi
//! - overflow_limb
//!
//! AIR explanation:
//!      res = op0 * imm                                                       (mod 2**32)
//! <=>  res = (op0_lo * imm_lo) + (op0_hi * imm_lo + op0_lo * imm_hi) * 2**16 (mod 2**32)
//! <=>  res = p0 + (p1 + p2) * 2**16                                          (mod 2**32)
//! <=>  res = p0_lo + (p0_hi + p1_lo + p2_lo) * 2**16                         (mod 2**32)
//! <=>  res = p0_lo + (p0_hi + p1_lo + p2_lo - overflow_limb) * 2**16
//! Last expression isn't (mod 2**32) so both limbs in the last expression are in range [0, 2^16): they can be identified as res_lo and res_hi.
//! Therefore AIR must check that:
//!  - p{i}_lo and p{i}_hi are the correct decomposition of the correct products
//!  - and that res_lo and res_hi are correctly built from these products limbs.
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * overflow_limb is either 0 or 1 or 2
//!   * `- overflow_limb * (1 - overflow_limb) * (2 - overflow_limb)`
//! * registers update is regular (+2 because of the two-worded instruction)
//!   * `- [pc, fp] + [pc + 2, fp]` in `Registers` relation
//! * read 2 instruction words from memory
//!   * `- [pc, inst_prev_clk, opcode_constant, src_off, imm_lo, imm_hi] + [pc, clk, opcode_constant, src_off, imm_lo, imm_hi]` in `Memory` relation
//!   * `- [pc + 1, inst_prev_clk, dst_off] + [pc + 1, clk, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read op0
//!   * `- [fp + src_off, op0_prev_clk, op0_val_lo] + [fp + src_off, clk, op0_val_lo]`
//!   * `- [fp + src_off + 1, op0_prev_clk, op0_val_hi] + [fp + src_off + 1, clk, op0_val_hi]`
//!   * `- [clk - op0_prev_clk - 1]` in `RangeCheck20` relation
//! * p{i} are correctly decomposed (range check 16 of limbs is done afterwards)
//!   * `op0_val_lo * imm_lo - (p0_hi << 16 + res_lo)`
//!   * `op0_val_hi * imm_lo - (p1_hi << 16 + p1_lo)`
//!   * `op0_val_lo * imm_hi - (p2_hi << 16 + p2_lo)`
//!   * `(p0_lo + p1_hi + p2_hi - overflow_limb) - res`
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clk, dst_prev_val_lo] + [fp + dst_off, clk, res_lo]` in `Memory` Relation
//!   * `- [fp + dst_off + 1, dst_prev_clk, dst_prev_val_hi] + [fp + dst_off + 1, clk, res_hi]` in `Memory` Relation
//!   * `- [clk - dst_prev_clk - 1]` in `RangeCheck20` relation
//! * limbs of each U32 must be in range [0, 2^16)
//!   * `- [op0_val_lo]` in `RangeCheck16` relation
//!   * `- [op0_val_hi]` in `RangeCheck16` relation
//!   * `- [imm_lo]` in `RangeCheck16` relation
//!   * `- [imm_hi]` in `RangeCheck16` relation
//!   * `- [p0_hi]` in `RangeCheck16` relation
//!   * `- [p1_lo]` in `RangeCheck16` relation
//!   * `- [p1_hi]` in `RangeCheck16` relation
//!   * `- [p2_lo]` in `RangeCheck16` relation
//!   * `- [p2_hi]` in `RangeCheck16` relation
//!   * `- [res_lo]` in `RangeCheck16` relation
//!   * `- [res_hi]` in `RangeCheck16` relation

use cairo_m_common::instruction::U32_STORE_MUL_FP_IMM;
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

const N_TRACE_COLUMNS: usize = 23;
const N_MEMORY_LOOKUPS: usize = 12;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 3;
const N_RANGE_CHECK_16_LOOKUPS: usize = 11;

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

    /// Writes the trace for the StoreMulFpImm opcode.
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

                let opcode_constant = PackedM31::from(M31::from(U32_STORE_MUL_FP_IMM));
                let src_off = input.inst_value_1;
                let imm_lo = input.inst_value_2;
                let imm_hi = input.inst_value_3;
                let dst_off = input.inst_value_4;

                let op0_val_lo = input.mem1_value_limb0;
                let op0_val_hi = input.mem1_value_limb1;
                let op0_prev_clock = input.mem1_prev_clock;

                let dst_prev_val_lo = input.mem2_prev_value_limb0;
                let dst_prev_val_hi = input.mem2_prev_value_limb1;
                let dst_prev_clock = input.mem2_prev_clock;

                // Compute products: p0 = op0_lo * imm_lo, p1 = op0_hi * imm_lo, p2 = op0_lo * imm_hi
                // Each product is a 32-bit value that needs to be decomposed into two 16-bit limbs

                // Helper to decompose a value into 16-bit limbs
                let decompose_16 = |val: PackedM31| -> (PackedM31, PackedM31) {
                    let lo = PackedM31::from_array(val.to_array().map(|x| M31::from(x.0 & 0xFFFF)));
                    let hi = PackedM31::from_array(val.to_array().map(|x| M31::from(x.0 >> 16)));
                    (lo, hi)
                };

                // Compute and decompose products
                let p0 = op0_val_lo * imm_lo;
                let (p0_lo, p0_hi) = decompose_16(p0);

                let p1 = op0_val_hi * imm_lo;
                let (p1_lo, p1_hi) = decompose_16(p1);

                let p2 = op0_val_lo * imm_hi;
                let (p2_lo, p2_hi) = decompose_16(p2);

                // Compute res_lo and res_hi
                let res_lo = p0_lo;
                let overflow_limb = PackedM31::from_array(
                    (p0_hi + p1_lo + p2_lo)
                        .to_array()
                        .map(|x| M31::from(x.0 >> 16)),
                );
                let res_hi = p0_hi + p1_lo + p2_lo - overflow_limb * two_pow_16;

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = src_off;
                *row[6] = imm_lo;
                *row[7] = imm_hi;
                *row[8] = dst_off;
                *row[9] = op0_val_lo;
                *row[10] = op0_val_hi;
                *row[11] = op0_prev_clock;
                *row[12] = dst_prev_val_lo;
                *row[13] = dst_prev_val_hi;
                *row[14] = dst_prev_clock;
                *row[15] = res_lo;
                *row[16] = res_hi;
                *row[17] = p0_hi;
                *row[18] = p1_lo;
                *row[19] = p1_hi;
                *row[20] = p2_lo;
                *row[21] = p2_hi;
                *row[22] = overflow_limb;

                *lookup_data.registers[0] = [input.pc, input.fp];
                *lookup_data.registers[1] = [input.pc + one + one, input.fp];

                // Read first QM31 word for instruction
                *lookup_data.memory[0] = [
                    input.pc,
                    inst_prev_clock,
                    opcode_constant,
                    src_off,
                    imm_lo,
                    imm_hi,
                ];
                *lookup_data.memory[1] =
                    [input.pc, clock, opcode_constant, src_off, imm_lo, imm_hi];

                // Read second QM31 word for instruction
                *lookup_data.memory[2] =
                    [input.pc + one, inst_prev_clock, dst_off, zero, zero, zero];
                *lookup_data.memory[3] = [input.pc + one, clock, dst_off, zero, zero, zero];

                // Read op0_lo
                *lookup_data.memory[4] =
                    [fp + src_off, op0_prev_clock, op0_val_lo, zero, zero, zero];
                *lookup_data.memory[5] = [fp + src_off, clock, op0_val_lo, zero, zero, zero];

                // Read op0_hi
                *lookup_data.memory[6] = [
                    fp + src_off + one,
                    op0_prev_clock,
                    op0_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[7] = [fp + src_off + one, clock, op0_val_hi, zero, zero, zero];

                // Write dst_lo
                *lookup_data.memory[8] = [
                    fp + dst_off,
                    dst_prev_clock,
                    dst_prev_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[9] = [fp + dst_off, clock, res_lo, zero, zero, zero];

                // Write dst_hi
                *lookup_data.memory[10] = [
                    fp + dst_off + one,
                    dst_prev_clock,
                    dst_prev_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[11] = [fp + dst_off + one, clock, res_hi, zero, zero, zero];

                // Limbs of each U32 must be in range [0, 2^16)
                *lookup_data.range_check_16[0] = op0_val_lo;
                *lookup_data.range_check_16[1] = op0_val_hi;
                *lookup_data.range_check_16[2] = imm_lo;
                *lookup_data.range_check_16[3] = imm_hi;
                *lookup_data.range_check_16[4] = p0_hi;
                *lookup_data.range_check_16[5] = p1_lo;
                *lookup_data.range_check_16[6] = p1_hi;
                *lookup_data.range_check_16[7] = p2_lo;
                *lookup_data.range_check_16[8] = p2_hi;
                *lookup_data.range_check_16[9] = res_lo;
                *lookup_data.range_check_16[10] = res_hi;

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - op0_prev_clock - enabler;
                *lookup_data.range_check_20[2] = clock - dst_prev_clock - enabler;
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
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_16[10],
            &interaction_claim_data.lookup_data.range_check_20[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_16_0, range_check_20_0))| {
                let num = -PackedQM31::one();
                let denom_0: PackedQM31 = relations.range_check_16.combine(&[*range_check_16_0]);
                let denom_1: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_0]);

                let numerator = num * denom_1 + num * denom_0;
                let denom = denom_0 * denom_1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_20[1],
            &interaction_claim_data.lookup_data.range_check_20[2],
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
        let two = E::F::from(M31::from(2));
        let two_pow_16 = E::F::from(M31::from(1 << 16));
        let opcode_constant = E::F::from(M31::from(U32_STORE_MUL_FP_IMM));

        // 23 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let src_off = eval.next_trace_mask();
        let imm_lo = eval.next_trace_mask();
        let imm_hi = eval.next_trace_mask();
        let dst_off = eval.next_trace_mask();
        let op0_val_lo = eval.next_trace_mask();
        let op0_val_hi = eval.next_trace_mask();
        let op0_prev_clock = eval.next_trace_mask();
        let dst_prev_val_lo = eval.next_trace_mask();
        let dst_prev_val_hi = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();
        let res_lo = eval.next_trace_mask();
        let res_hi = eval.next_trace_mask();
        let p0_hi = eval.next_trace_mask();
        let p1_lo = eval.next_trace_mask();
        let p1_hi = eval.next_trace_mask();
        let p2_lo = eval.next_trace_mask();
        let p2_hi = eval.next_trace_mask();
        let overflow_limb = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // overflow_limb is 0, 1, or 2
        eval.add_constraint(
            overflow_limb.clone()
                * (one.clone() - overflow_limb.clone())
                * (two - overflow_limb.clone()),
        );

        // Product decomposition constraints
        // p0 = op0_val_lo * imm_lo = p0_hi * 2^16 + p0_lo (where p0_lo is res_lo)
        eval.add_constraint(
            enabler.clone()
                * (op0_val_lo.clone() * imm_lo.clone()
                    - (p0_hi.clone() * two_pow_16.clone() + res_lo.clone())),
        );

        // p1 = op0_val_hi * imm_lo = p1_hi * 2^16 + p1_lo
        eval.add_constraint(
            enabler.clone()
                * (op0_val_hi.clone() * imm_lo.clone()
                    - (p1_hi.clone() * two_pow_16.clone() + p1_lo.clone())),
        );

        // p2 = op0_val_lo * imm_hi = p2_hi * 2^16 + p2_lo
        eval.add_constraint(
            enabler.clone()
                * (op0_val_lo.clone() * imm_hi.clone()
                    - (p2_hi.clone() * two_pow_16.clone() + p2_lo.clone())),
        );

        // Result constraint: res_hi = p0_hi + p1_lo + p2_lo - overflow_limb * 2^16
        eval.add_constraint(
            enabler.clone()
                * (res_hi.clone()
                    - (p0_hi.clone() + p1_lo.clone() + p2_lo.clone() - overflow_limb * two_pow_16)),
        );

        // Registers update
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            E::EF::from(enabler.clone()),
            &[pc.clone() + one.clone() + one.clone(), fp.clone()],
        ));

        // Read 1st instruction word from memory
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                inst_prev_clock.clone(),
                opcode_constant.clone(),
                src_off.clone(),
                imm_lo.clone(),
                imm_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                clock.clone(),
                opcode_constant,
                src_off.clone(),
                imm_lo.clone(),
                imm_hi.clone(),
            ],
        ));

        // Read 2nd instruction word from memory
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone() + one.clone(),
                inst_prev_clock.clone(),
                dst_off.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[pc + one.clone(), clock.clone(), dst_off.clone()],
        ));

        // Read op0_lo
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone(),
                op0_prev_clock.clone(),
                op0_val_lo.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone(),
                clock.clone(),
                op0_val_lo.clone(),
            ],
        ));

        // Read op0_hi
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone() + one.clone(),
                op0_prev_clock.clone(),
                op0_val_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off + one.clone(),
                clock.clone(),
                op0_val_hi.clone(),
            ],
        ));

        // Write dst_lo
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + dst_off.clone(),
                dst_prev_clock.clone(),
                dst_prev_val_lo,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + dst_off.clone(), clock.clone(), res_lo.clone()],
        ));

        // Write dst_hi
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + dst_off.clone() + one.clone(),
                dst_prev_clock.clone(),
                dst_prev_val_hi,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp + dst_off + one, clock.clone(), res_hi.clone()],
        ));

        // Range check 16 for all limbs
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
            &[imm_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[imm_hi],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[p0_hi],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[p1_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[p1_hi],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[p2_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[p2_hi],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[res_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[res_hi],
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
            &[clock.clone() - op0_prev_clock - enabler.clone()],
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
