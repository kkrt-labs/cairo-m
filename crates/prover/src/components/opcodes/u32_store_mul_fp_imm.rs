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
//! - imm_0
//! - imm_1
//! - imm_2
//! - imm_3
//! - dst_off
//! - op0_0
//! - op0_1
//! - op0_2
//! - op0_3
//! - op0_prev_clock_lo
//! - op0_prev_clock_hi
//! - dst_prev_val_lo
//! - dst_prev_val_hi
//! - dst_prev_clock_lo
//! - dst_prev_clock_hi
//! - res_0
//! - res_1
//! - res_2
//! - res_3
//! - carry_0
//! - carry_1
//! - carry_2
//! - carry_3
//!
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * registers update is regular (+2 because of the two-worded instruction)
//!   * `- [pc, fp, clock] + [pc + 2, fp, clock + 1]` in `Registers` relation
//! * read 2 instruction words from memory
//!   * `- [pc, inst_prev_clk, opcode_constant, src_off, imm_0 + imm_1 * 2 ** 8, imm_2 + imm_3 * 2 ** 8]
//!      + [pc, clk          , opcode_constant, src_off, imm_0 + imm_1 * 2 ** 8, imm_2 + imm_3 * 2 ** 8]` in `Memory` relation
//!   * `- [pc + 1, inst_prev_clk, dst_off] + [pc + 1, clk, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read op0
//!   * `- [fp + src_off, op0_prev_clock_lo_clk, op0_0 + op0_1 * 2 ** 8] + [fp + src_off, clk, op0_0 + op0_1 * 2 ** 8]`
//!   * `- [fp + src_off + 1, op0_prev_clock_hi_clk, op0_2 + op0_3 * 2 ** 8] + [fp + src_off + 1, clk, op0_2 + op0_3 * 2 ** 8]`
//!   * `- [clk - op0_prev_clock_lo_clk - 1]` and `- [clk - op0_prev_clock_hi_clk - 1]` in `RangeCheck20` relation
//! * check res limbs correctness
//!   * `- res_0 - (op0_0 * imm_0 - carry_0 * 2 ** 8)`
//!   * `- res_1 - (op0_0 * imm_1 + op0_1 * imm_0 + carry_0 - carry_1 * 2 ** 8)`
//!   * `- res_2 - (op0_0 * imm_2 + op0_1 * imm_1 + op0_2 * imm_0 + carry_1 - carry_2 * 2 ** 8)`
//!   * `- res_3 - (op0_0 * imm_3 + op0_1 * imm_2 + op0_2 * imm_1 + op0_3 * imm_0 + carry_2 - carry_3 * 2 ** 8)`
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clock_lo_clk, dst_prev_val_lo] + [fp + dst_off, clk, res_0 + res_1 * 2 ** 8]` in `Memory` relation
//!   * `- [fp + dst_off + 1, dst_prev_clock_hi_clk, dst_prev_val_hi] + [fp + dst_off + 1, clk, res_2 + res_3 * 2 ** 8]` in `Memory` relation
//!   * `- [clk - dst_prev_clock_lo_clk - 1]` and `- [clk - dst_prev_clock_hi_clk - 1]` in `RangeCheck20` relation
//! * limbs of each U32 must be in range [0, 2^8)
//!   * `- [op0_0]` in `RangeCheck8` relation
//!   * `- [op0_1]` in `RangeCheck8` relation
//!   * `- [op0_2]` in `RangeCheck8` relation
//!   * `- [op0_3]` in `RangeCheck8` relation
//!   * `- [imm_0]` in `RangeCheck8` relation
//!   * `- [imm_1]` in `RangeCheck8` relation
//!   * `- [imm_2]` in `RangeCheck8` relation
//!   * `- [imm_3]` in `RangeCheck8` relation
//!   * `- [res_0]` in `RangeCheck8` relation
//!   * `- [res_1]` in `RangeCheck8` relation
//!   * `- [res_2]` in `RangeCheck8` relation
//!   * `- [res_3]` in `RangeCheck8` relation
//! * carry limbs must be in the correct range
//!   * `- [254 - carry_0]` in `RangeCheck16` relation
//!   * `- [509 - carry_1]` in `RangeCheck16` relation
//!   * `- [764 - carry_2]` in `RangeCheck16` relation
//!   * `- [1019 - carry_3]` in `RangeCheck16` relation

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

use crate::adapter::memory::DataAccess;
use crate::adapter::ExecutionBundle;
use crate::components::Relations;
use crate::preprocessed::bitwise::BitwiseProvider;
use crate::preprocessed::range_check::RangeCheckProvider;
use crate::utils::data_accesses::{get_prev_clock, get_prev_value, get_value};
use crate::utils::enabler::Enabler;
use crate::utils::execution_bundle::PackedExecutionBundle;

const N_TRACE_COLUMNS: usize = 29;
const N_MEMORY_LOOKUPS: usize = 12;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_8_LOOKUPS: usize = 12;
const N_RANGE_CHECK_16_LOOKUPS: usize = 4;
const N_RANGE_CHECK_20_LOOKUPS: usize = 5;

// 1 * (255 * 255) = 254 * 2^8 + 1
// 2 * (255 * 255) + 254 = 509 * 2^8
// 3 * (255 * 255) + 509 = 764 * 2^8
// 4 * (255 * 255) + 764 = 1019 * 2^8
const MAX_CARRY_0: u32 = 254;
const MAX_CARRY_1: u32 = 509;
const MAX_CARRY_2: u32 = 764;
const MAX_CARRY_3: u32 = 1019;

const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_MEMORY_LOOKUPS
        + N_REGISTERS_LOOKUPS
        + N_RANGE_CHECK_20_LOOKUPS
        + N_RANGE_CHECK_8_LOOKUPS
        + N_RANGE_CHECK_16_LOOKUPS)
        .div_ceil(2);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

impl RangeCheckProvider for InteractionClaimData {
    fn get_range_check_8(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_8.par_iter().flatten()
    }
    fn get_range_check_16(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_16.par_iter().flatten()
    }
    fn get_range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_20.par_iter().flatten()
    }
}

impl BitwiseProvider for InteractionClaimData {}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub registers: [Vec<[PackedM31; 3]>; N_REGISTERS_LOOKUPS],
    pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
    pub range_check_8: [Vec<PackedM31>; N_RANGE_CHECK_8_LOOKUPS],
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

                let opcode_constant = PackedM31::from(M31::from(U32_STORE_MUL_FP_IMM));
                let src_off = input.inst_value_1;
                let imm_lo = input.inst_value_2;
                let imm_hi = input.inst_value_3;
                let dst_off = input.inst_value_4;

                // Operand 0 (u32) comes from two separate memory reads
                let op0_prev_clock_lo = get_prev_clock(input, data_accesses, 0);
                let op0_val_lo = get_value(input, data_accesses, 0);
                let op0_prev_clock_hi = get_prev_clock(input, data_accesses, 1);
                let op0_val_hi = get_value(input, data_accesses, 1);

                // Destination (u32) previous values and clocks for each limb
                let dst_prev_clock_lo = get_prev_clock(input, data_accesses, 2);
                let dst_prev_val_lo = get_prev_value(input, data_accesses, 2);
                let dst_prev_clock_hi = get_prev_clock(input, data_accesses, 3);
                let dst_prev_val_hi = get_prev_value(input, data_accesses, 3);

                // Helper to decompose M31 value into 8-bit limbs
                let decompose_8 = |val: PackedM31| -> (PackedM31, PackedM31) {
                    let lo = PackedM31::from_array(val.to_array().map(|x| M31::from(x.0 & 0xFF)));
                    let hi = PackedM31::from_array(val.to_array().map(|x| M31::from(x.0 >> 8)));
                    (lo, hi)
                };

                // Decompose operands and immediates into 8-bit limbs
                let (op0_0, op0_1) = decompose_8(op0_val_lo);
                let (op0_2, op0_3) = decompose_8(op0_val_hi);
                let (imm_0, imm_1) = decompose_8(imm_lo);
                let (imm_2, imm_3) = decompose_8(imm_hi);

                // Compute carry values and result limbs according to the AIR
                let two_pow_8 = PackedM31::from(M31::from(1 << 8));

                // res_0 = op0_0 * imm_0 - carry_0 * 2^8
                let prod_0 = op0_0 * imm_0;
                let carry_0 = PackedM31::from_array(prod_0.to_array().map(|x| M31::from(x.0 >> 8)));
                let res_0 = prod_0 - carry_0 * two_pow_8;

                // res_1 = op0_0 * imm_1 + op0_1 * imm_0 + carry_0 - carry_1 * 2^8
                let sum_1 = op0_0 * imm_1 + op0_1 * imm_0 + carry_0;
                let carry_1 = PackedM31::from_array(sum_1.to_array().map(|x| M31::from(x.0 >> 8)));
                let res_1 = sum_1 - carry_1 * two_pow_8;

                // res_2 = op0_0 * imm_2 + op0_1 * imm_1 + op0_2 * imm_0 + carry_1 - carry_2 * 2^8
                let sum_2 = op0_0 * imm_2 + op0_1 * imm_1 + op0_2 * imm_0 + carry_1;
                let carry_2 = PackedM31::from_array(sum_2.to_array().map(|x| M31::from(x.0 >> 8)));
                let res_2 = sum_2 - carry_2 * two_pow_8;

                // res_3 = op0_0 * imm_3 + op0_1 * imm_2 + op0_2 * imm_1 + op0_3 * imm_0 + carry_2 - carry_3 * 2^8
                let sum_3 = op0_0 * imm_3 + op0_1 * imm_2 + op0_2 * imm_1 + op0_3 * imm_0 + carry_2;
                let carry_3 = PackedM31::from_array(sum_3.to_array().map(|x| M31::from(x.0 >> 8)));
                let res_3 = sum_3 - carry_3 * two_pow_8;

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = src_off;
                *row[6] = imm_0;
                *row[7] = imm_1;
                *row[8] = imm_2;
                *row[9] = imm_3;
                *row[10] = dst_off;
                *row[11] = op0_0;
                *row[12] = op0_1;
                *row[13] = op0_2;
                *row[14] = op0_3;
                *row[15] = op0_prev_clock_lo;
                *row[16] = op0_prev_clock_hi;
                *row[17] = dst_prev_val_lo;
                *row[18] = dst_prev_val_hi;
                *row[19] = dst_prev_clock_lo;
                *row[20] = dst_prev_clock_hi;
                *row[21] = res_0;
                *row[22] = res_1;
                *row[23] = res_2;
                *row[24] = res_3;
                *row[25] = carry_0;
                *row[26] = carry_1;
                *row[27] = carry_2;
                *row[28] = carry_3;

                *lookup_data.registers[0] = [input.pc, input.fp, input.clock];
                *lookup_data.registers[1] = [input.pc + one + one, input.fp, input.clock + one];

                // Read first QM31 word for instruction
                let two_pow_8 = PackedM31::from(M31::from(1 << 8));
                *lookup_data.memory[0] = [
                    input.pc,
                    inst_prev_clock,
                    opcode_constant,
                    src_off,
                    imm_0 + imm_1 * two_pow_8,
                    imm_2 + imm_3 * two_pow_8,
                ];
                *lookup_data.memory[1] = [
                    input.pc,
                    clock,
                    opcode_constant,
                    src_off,
                    imm_0 + imm_1 * two_pow_8,
                    imm_2 + imm_3 * two_pow_8,
                ];

                // Read second QM31 word for instruction
                *lookup_data.memory[2] =
                    [input.pc + one, inst_prev_clock, dst_off, zero, zero, zero];
                *lookup_data.memory[3] = [input.pc + one, clock, dst_off, zero, zero, zero];

                // Read op0_lo
                *lookup_data.memory[4] = [
                    fp + src_off,
                    op0_prev_clock_lo,
                    op0_0 + op0_1 * two_pow_8,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[5] = [
                    fp + src_off,
                    clock,
                    op0_0 + op0_1 * two_pow_8,
                    zero,
                    zero,
                    zero,
                ];

                // Read op0_hi
                *lookup_data.memory[6] = [
                    fp + src_off + one,
                    op0_prev_clock_hi,
                    op0_2 + op0_3 * two_pow_8,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[7] = [
                    fp + src_off + one,
                    clock,
                    op0_2 + op0_3 * two_pow_8,
                    zero,
                    zero,
                    zero,
                ];

                // Write dst_lo
                *lookup_data.memory[8] = [
                    fp + dst_off,
                    dst_prev_clock_lo,
                    dst_prev_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[9] = [
                    fp + dst_off,
                    clock,
                    res_0 + res_1 * two_pow_8,
                    zero,
                    zero,
                    zero,
                ];

                // Write dst_hi
                *lookup_data.memory[10] = [
                    fp + dst_off + one,
                    dst_prev_clock_hi,
                    dst_prev_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[11] = [
                    fp + dst_off + one,
                    clock,
                    res_2 + res_3 * two_pow_8,
                    zero,
                    zero,
                    zero,
                ];

                // Limbs of each U32 must be in range [0, 2^8)
                *lookup_data.range_check_8[0] = op0_0;
                *lookup_data.range_check_8[1] = op0_1;
                *lookup_data.range_check_8[2] = op0_2;
                *lookup_data.range_check_8[3] = op0_3;
                *lookup_data.range_check_8[4] = imm_0;
                *lookup_data.range_check_8[5] = imm_1;
                *lookup_data.range_check_8[6] = imm_2;
                *lookup_data.range_check_8[7] = imm_3;
                *lookup_data.range_check_8[8] = res_0;
                *lookup_data.range_check_8[9] = res_1;
                *lookup_data.range_check_8[10] = res_2;
                *lookup_data.range_check_8[11] = res_3;

                // Carry limbs must be in the correct range
                let max_carry_0 = PackedM31::from(M31::from(MAX_CARRY_0));
                let max_carry_1 = PackedM31::from(M31::from(MAX_CARRY_1));
                let max_carry_2 = PackedM31::from(M31::from(MAX_CARRY_2));
                let max_carry_3 = PackedM31::from(M31::from(MAX_CARRY_3));
                *lookup_data.range_check_16[0] = max_carry_0 - carry_0;
                *lookup_data.range_check_16[1] = max_carry_1 - carry_1;
                *lookup_data.range_check_16[2] = max_carry_2 - carry_2;
                *lookup_data.range_check_16[3] = max_carry_3 - carry_3;

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - op0_prev_clock_lo - enabler;
                *lookup_data.range_check_20[2] = clock - op0_prev_clock_hi - enabler;
                *lookup_data.range_check_20[3] = clock - dst_prev_clock_lo - enabler;
                *lookup_data.range_check_20[4] = clock - dst_prev_clock_hi - enabler;
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

        // Range check 8 lookups
        for i in 0..N_RANGE_CHECK_8_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.range_check_8[i * 2],
                &interaction_claim_data.lookup_data.range_check_8[i * 2 + 1],
            )
                .into_par_iter()
                .enumerate()
                .for_each(|(_i, (writer, range_check_8_0, range_check_8_1))| {
                    let num = -PackedQM31::one();
                    let denom_0: PackedQM31 = relations.range_check_8.combine(&[*range_check_8_0]);
                    let denom_1: PackedQM31 = relations.range_check_8.combine(&[*range_check_8_1]);

                    let numerator = num * denom_1 + num * denom_0;
                    let denom = denom_0 * denom_1;

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
            &interaction_claim_data.lookup_data.range_check_20[3],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_20_2, range_check_20_3))| {
                let num = -PackedQM31::one();
                let denom_2: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_2]);
                let denom_3: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_3]);

                let numerator = num * denom_3 + num * denom_2;
                let denom = denom_2 * denom_3;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        // Last RC20 with itself since we have an odd number (5 total)
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_20[4],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_20_4))| {
                let num = -PackedQM31::one();
                let denom: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_4]);

                writer.write_frac(num, denom);
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
        let two_pow_8 = E::F::from(M31::from(1 << 8));
        let opcode_constant = E::F::from(M31::from(U32_STORE_MUL_FP_IMM));

        // 29 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let src_off = eval.next_trace_mask();
        let imm_0 = eval.next_trace_mask();
        let imm_1 = eval.next_trace_mask();
        let imm_2 = eval.next_trace_mask();
        let imm_3 = eval.next_trace_mask();
        let dst_off = eval.next_trace_mask();
        let op0_0 = eval.next_trace_mask();
        let op0_1 = eval.next_trace_mask();
        let op0_2 = eval.next_trace_mask();
        let op0_3 = eval.next_trace_mask();
        let op0_prev_clock_lo = eval.next_trace_mask();
        let op0_prev_clock_hi = eval.next_trace_mask();
        let dst_prev_val_lo = eval.next_trace_mask();
        let dst_prev_val_hi = eval.next_trace_mask();
        let dst_prev_clock_lo = eval.next_trace_mask();
        let dst_prev_clock_hi = eval.next_trace_mask();
        let res_0 = eval.next_trace_mask();
        let res_1 = eval.next_trace_mask();
        let res_2 = eval.next_trace_mask();
        let res_3 = eval.next_trace_mask();
        let carry_0 = eval.next_trace_mask();
        let carry_1 = eval.next_trace_mask();
        let carry_2 = eval.next_trace_mask();
        let carry_3 = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Check res limbs correctness
        // res_0 = op0_0 * imm_0 - carry_0 * 2^8
        eval.add_constraint(
            enabler.clone()
                * (res_0.clone()
                    - (op0_0.clone() * imm_0.clone() - carry_0.clone() * two_pow_8.clone())),
        );

        // res_1 = op0_0 * imm_1 + op0_1 * imm_0 + carry_0 - carry_1 * 2^8
        eval.add_constraint(
            enabler.clone()
                * (res_1.clone()
                    - (op0_0.clone() * imm_1.clone()
                        + op0_1.clone() * imm_0.clone()
                        + carry_0.clone()
                        - carry_1.clone() * two_pow_8.clone())),
        );

        // res_2 = op0_0 * imm_2 + op0_1 * imm_1 + op0_2 * imm_0 + carry_1 - carry_2 * 2^8
        eval.add_constraint(
            enabler.clone()
                * (res_2.clone()
                    - (op0_0.clone() * imm_2.clone()
                        + op0_1.clone() * imm_1.clone()
                        + op0_2.clone() * imm_0.clone()
                        + carry_1.clone()
                        - carry_2.clone() * two_pow_8.clone())),
        );

        // res_3 = op0_0 * imm_3 + op0_1 * imm_2 + op0_2 * imm_1 + op0_3 * imm_0 + carry_2 - carry_3 * 2^8
        eval.add_constraint(
            enabler.clone()
                * (res_3.clone()
                    - (op0_0.clone() * imm_3.clone()
                        + op0_1.clone() * imm_2.clone()
                        + op0_2.clone() * imm_1.clone()
                        + op0_3.clone() * imm_0.clone()
                        + carry_2.clone()
                        - carry_3.clone() * two_pow_8.clone())),
        );

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
                pc.clone() + one.clone() + one.clone(),
                fp.clone(),
                clock.clone() + one.clone(),
            ],
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
                imm_0.clone() + imm_1.clone() * two_pow_8.clone(),
                imm_2.clone() + imm_3.clone() * two_pow_8.clone(),
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
                imm_0.clone() + imm_1.clone() * two_pow_8.clone(),
                imm_2.clone() + imm_3.clone() * two_pow_8.clone(),
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
                op0_prev_clock_lo.clone(),
                op0_0.clone() + op0_1.clone() * two_pow_8.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone(),
                clock.clone(),
                op0_0.clone() + op0_1.clone() * two_pow_8.clone(),
            ],
        ));

        // Read op0_hi
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone() + one.clone(),
                op0_prev_clock_hi.clone(),
                op0_2.clone() + op0_3.clone() * two_pow_8.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off + one.clone(),
                clock.clone(),
                op0_2.clone() + op0_3.clone() * two_pow_8.clone(),
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
            &[
                fp.clone() + dst_off.clone(),
                clock.clone(),
                res_0.clone() + res_1.clone() * two_pow_8.clone(),
            ],
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
            &[
                fp + dst_off + one,
                clock.clone(),
                res_2.clone() + res_3.clone() * two_pow_8,
            ],
        ));

        // Range check 8 for all limbs
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[op0_0],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[op0_1],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[op0_2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[op0_3],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[imm_0],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[imm_1],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[imm_2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[imm_3],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[res_0],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[res_1],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[res_2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_8,
            -E::EF::one(),
            &[res_3],
        ));

        // Range check 16 for carry limbs
        let max_carry_0 = E::F::from(M31::from(MAX_CARRY_0));
        let max_carry_1 = E::F::from(M31::from(MAX_CARRY_1));
        let max_carry_2 = E::F::from(M31::from(MAX_CARRY_2));
        let max_carry_3 = E::F::from(M31::from(MAX_CARRY_3));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_0 - carry_0],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_1 - carry_1],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_2 - carry_2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_3 - carry_3],
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
            &[clock.clone() - op0_prev_clock_lo - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - op0_prev_clock_hi - enabler.clone()],
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
