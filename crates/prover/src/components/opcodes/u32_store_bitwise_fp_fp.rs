//! This component is used to prove the U32StoreBitwiseFpFp opcode.
//! u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) &/|/^ u32([fp + src1_off], [fp + src1_off + 1])
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - opcode_constant
//! - src0_off
//! - src1_off
//! - dst_off
//! - op0_val_0
//! - op0_val_1
//! - op0_val_2
//! - op0_val_3
//! - op0_prev_clock_lo
//! - op0_prev_clock_hi
//! - op1_val_0
//! - op1_val_1
//! - op1_val_2
//! - op1_val_3
//! - op1_prev_clock_lo
//! - op1_prev_clock_hi
//! - dst_prev_val_lo
//! - dst_prev_val_hi
//! - res_0
//! - res_1
//! - res_2
//! - res_3
//! - dst_prev_clock_lo
//! - dst_prev_clock_hi
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * define `bitwise_op = opcode_constant - 36`
//! * registers update is regular
//!   * `- [pc, fp] + [pc + 1, fp]` in `Registers` relation
//! * read instruction
//!   * `- [pc, inst_prev_clk, opcode_constant, src0_off, src1_off, dst_off] + [pc, clk, opcode_constant, src0_off, src1_off, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read op0
//!   * `- [fp + src0_off, op0_prev_clock_lo, op0_val_0 + op0_val_1 * 2 ** 8] + [fp + src0_off, clk, op0_val_0 + op0_val_1 * 2 ** 8]`
//!   * `- [fp + src0_off + 1, op0_prev_clock_hi, op0_val_2 + op0_val_3 * 2 ** 8] + [fp + src0_off + 1, clk, op0_val_2 + op0_val_3 * 2 ** 8]`
//!   * `- [clk - op0_prev_clock_lo - 1]` and `- [clk - op0_prev_clock_hi - 1]` in `RangeCheck20` relation
//! * read op1
//!   * `- [fp + src1_off, op1_prev_clock_lo, op1_val_0 + op1_val_1 * 2 ** 8] + [fp + src1_off, clk, op1_val_0 + op1_val_1 * 2 ** 8]`
//!   * `- [fp + src1_off + 1, op1_prev_clock_hi, op1_val_2 + op1_val_3 * 2 ** 8] + [fp + src1_off + 1, clk, op1_val_2 + op1_val_3 * 2 ** 8]`
//!   * `- [clk - op1_prev_clock_lo - 1]` and `- [clk - op1_prev_clock_hi - 1]` in `RangeCheck20` relation
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clock_lo, dst_prev_val_lo] + [fp + dst_off, clk, res_0 + res_1 * 2 ** 8]` in `Memory` relation
//!   * `- [fp + dst_off + 1, dst_prev_clock_hi, dst_prev_val_hi] + [fp + dst_off + 1, clk, res_2 + res_3 * 2 ** 8]` in `Memory` relation
//!   * `- [clk - dst_prev_clock_lo - 1]` and `- [clk - dst_prev_clock_hi - 1]` in `RangeCheck20` relation
//! * check the validity of the bitwise operation
//!   * `- [bitwise_op, op0_val_0, op1_val_0, res_0]` in Bitwise relation
//!   * `- [bitwise_op, op0_val_1, op1_val_1, res_1]` in Bitwise relation
//!   * `- [bitwise_op, op0_val_2, op1_val_2, res_2]` in Bitwise relation
//!   * `- [bitwise_op, op0_val_3, op1_val_3, res_3]` in Bitwise relation
//! * no need to 8-bit range_check since the bitwise lookup does it

use cairo_m_common::instruction::{U32_STORE_AND_FP_FP, U32_STORE_OR_FP_FP, U32_STORE_XOR_FP_FP};
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
const N_MEMORY_LOOKUPS: usize = 14;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 7;
const N_BITWISE_LOOKUPS: usize = 4;

const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_MEMORY_LOOKUPS + N_REGISTERS_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS + N_BITWISE_LOOKUPS)
        .div_ceil(2);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

impl RangeCheckProvider for InteractionClaimData {
    fn get_range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_20.par_iter().flatten()
    }
}

impl BitwiseProvider for InteractionClaimData {
    fn get_bitwise(&self) -> impl ParallelIterator<Item = &[[PackedM31; 4]]> {
        self.lookup_data.bitwise.par_iter().map(|v| v.as_slice())
    }
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
    pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
    pub bitwise: [Vec<[PackedM31; 4]>; N_BITWISE_LOOKUPS],
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

    /// Writes the trace for the U32StoreBitwiseFpFp opcodes.
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
        // Clear the inputs to free memory early.
        inputs.clear();
        inputs.shrink_to_fit();

        let zero = PackedM31::from(M31::zero());
        let one = PackedM31::from(M31::one());
        let two_pow_8 = PackedM31::from(M31::from(1 << 8));
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

                let opcode_constant = PackedM31::from(input.inst_value_0.to_array().map(|x| {
                    if x.0 == 11 {
                        M31::from(U32_STORE_AND_FP_FP)
                    } else {
                        x
                    }
                }));
                let src0_off = input.inst_value_1;
                let src1_off = input.inst_value_2;
                let dst_off = input.inst_value_3;

                // Read op0 from memory
                let op0_val_lo = get_value(input, data_accesses, 0);
                let op0_val_hi = get_value(input, data_accesses, 1);
                let op0_prev_clock_lo = get_prev_clock(input, data_accesses, 0);
                let op0_prev_clock_hi = get_prev_clock(input, data_accesses, 1);

                // Read op1 from memory
                let op1_val_lo = get_value(input, data_accesses, 2);
                let op1_val_hi = get_value(input, data_accesses, 3);
                let op1_prev_clock_lo = get_prev_clock(input, data_accesses, 2);
                let op1_prev_clock_hi = get_prev_clock(input, data_accesses, 3);

                // Write dst
                let dst_prev_val_lo = get_prev_value(input, data_accesses, 4);
                let dst_prev_val_hi = get_prev_value(input, data_accesses, 5);
                let dst_prev_clock_lo = get_prev_clock(input, data_accesses, 4);
                let dst_prev_clock_hi = get_prev_clock(input, data_accesses, 5);

                // Extract the 8-bit limbs from the 16-bit values
                let decompose_8 = |val: PackedM31| -> (PackedM31, PackedM31) {
                    let lo = PackedM31::from_array(val.to_array().map(|x| M31::from(x.0 & 0xFF)));
                    let hi =
                        PackedM31::from_array(val.to_array().map(|x| M31::from((x.0 >> 8) & 0xFF)));
                    (lo, hi)
                };
                let (op0_val_0, op0_val_1) = decompose_8(op0_val_lo);
                let (op0_val_2, op0_val_3) = decompose_8(op0_val_hi);
                let (op1_val_0, op1_val_1) = decompose_8(op1_val_lo);
                let (op1_val_2, op1_val_3) = decompose_8(op1_val_hi);

                // Compute the result based on the operation
                let bitwise_op = opcode_constant - PackedM31::from(M31::from(U32_STORE_AND_FP_FP));

                let res_0 = compute_bitwise_result(opcode_constant, op0_val_0, op1_val_0);
                let res_1 = compute_bitwise_result(opcode_constant, op0_val_1, op1_val_1);
                let res_2 = compute_bitwise_result(opcode_constant, op0_val_2, op1_val_2);
                let res_3 = compute_bitwise_result(opcode_constant, op0_val_3, op1_val_3);

                let res_lo = res_0 + res_1 * two_pow_8;
                let res_hi = res_2 + res_3 * two_pow_8;

                // Write trace columns
                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = opcode_constant;
                *row[6] = src0_off;
                *row[7] = src1_off;
                *row[8] = dst_off;
                *row[9] = op0_val_0;
                *row[10] = op0_val_1;
                *row[11] = op0_val_2;
                *row[12] = op0_val_3;
                *row[13] = op0_prev_clock_lo;
                *row[14] = op0_prev_clock_hi;
                *row[15] = op1_val_0;
                *row[16] = op1_val_1;
                *row[17] = op1_val_2;
                *row[18] = op1_val_3;
                *row[19] = op1_prev_clock_lo;
                *row[20] = op1_prev_clock_hi;
                *row[21] = dst_prev_val_lo;
                *row[22] = dst_prev_val_hi;
                *row[23] = res_0;
                *row[24] = res_1;
                *row[25] = res_2;
                *row[26] = res_3;
                *row[27] = dst_prev_clock_lo;
                *row[28] = dst_prev_clock_hi;

                // Register lookups
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

                // Read op0
                *lookup_data.memory[2] = [
                    fp + src0_off,
                    op0_prev_clock_lo,
                    op0_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[3] = [fp + src0_off, clock, op0_val_lo, zero, zero, zero];
                *lookup_data.memory[4] = [
                    fp + src0_off + one,
                    op0_prev_clock_hi,
                    op0_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[5] = [fp + src0_off + one, clock, op0_val_hi, zero, zero, zero];

                // Read op1
                *lookup_data.memory[6] = [
                    fp + src1_off,
                    op1_prev_clock_lo,
                    op1_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[7] = [fp + src1_off, clock, op1_val_lo, zero, zero, zero];
                *lookup_data.memory[8] = [
                    fp + src1_off + one,
                    op1_prev_clock_hi,
                    op1_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[9] = [fp + src1_off + one, clock, op1_val_hi, zero, zero, zero];

                // Write dst
                *lookup_data.memory[10] = [
                    fp + dst_off,
                    dst_prev_clock_lo,
                    dst_prev_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[11] = [fp + dst_off, clock, res_lo, zero, zero, zero];
                *lookup_data.memory[12] = [
                    fp + dst_off + one,
                    dst_prev_clock_hi,
                    dst_prev_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[13] = [fp + dst_off + one, clock, res_hi, zero, zero, zero];

                // Bitwise lookups - only store operation_id and inputs, result is verified by lookup
                *lookup_data.bitwise[0] = [bitwise_op, op0_val_0, op1_val_0, res_0];
                *lookup_data.bitwise[1] = [bitwise_op, op0_val_1, op1_val_1, res_1];
                *lookup_data.bitwise[2] = [bitwise_op, op0_val_2, op1_val_2, res_2];
                *lookup_data.bitwise[3] = [bitwise_op, op0_val_3, op1_val_3, res_3];

                // Range checks
                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - op0_prev_clock_lo - enabler;
                *lookup_data.range_check_20[2] = clock - op0_prev_clock_hi - enabler;
                *lookup_data.range_check_20[3] = clock - op1_prev_clock_lo - enabler;
                *lookup_data.range_check_20[4] = clock - op1_prev_clock_hi - enabler;
                *lookup_data.range_check_20[5] = clock - dst_prev_clock_lo - enabler;
                *lookup_data.range_check_20[6] = clock - dst_prev_clock_hi - enabler;
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

fn compute_bitwise_result(opcode: PackedM31, a: PackedM31, b: PackedM31) -> PackedM31 {
    // This is just for computing the trace values
    // The actual verification happens in the bitwise relation
    let and_result = a
        .to_array()
        .iter()
        .zip(b.to_array().iter())
        .map(|(a, b)| M31::from(a.0 & b.0))
        .collect::<Vec<_>>();
    let or_result = a
        .to_array()
        .iter()
        .zip(b.to_array().iter())
        .map(|(a, b)| M31::from(a.0 | b.0))
        .collect::<Vec<_>>();
    let xor_result = a
        .to_array()
        .iter()
        .zip(b.to_array().iter())
        .map(|(a, b)| M31::from(a.0 ^ b.0))
        .collect::<Vec<_>>();

    // Select based on opcode
    let mut result = [M31::zero(); N_LANES];
    for i in 0..N_LANES {
        let op = opcode.to_array()[i].0;
        result[i] = match op {
            U32_STORE_AND_FP_FP => and_result[i], // AND
            U32_STORE_OR_FP_FP => or_result[i],   // OR
            U32_STORE_XOR_FP_FP => xor_result[i], // XOR
            _ => M31::zero(),
        };
    }

    PackedM31::from_array(result)
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

        // Bitwise lookups
        for i in 0..N_BITWISE_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.bitwise[i * 2],
                &interaction_claim_data.lookup_data.bitwise[i * 2 + 1],
            )
                .into_par_iter()
                .enumerate()
                .for_each(|(_i, (writer, bitwise_0, bitwise_1))| {
                    let num = -PackedQM31::one();
                    let denom_0: PackedQM31 = relations.bitwise.combine(bitwise_0);
                    let denom_1: PackedQM31 = relations.bitwise.combine(bitwise_1);

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
                &interaction_claim_data.lookup_data.range_check_20[2 * i],
                &interaction_claim_data.lookup_data.range_check_20[2 * i + 1],
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

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_20[N_RANGE_CHECK_20_LOOKUPS - 1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, range_check_20_last))| {
                let num = -PackedQM31::one();
                let denom: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_last]);
                writer.write_frac(num, denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (Self { claimed_sum }, trace)
    }
}

#[derive(Clone)]
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
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let opcode_constant = eval.next_trace_mask();
        let src0_off = eval.next_trace_mask();
        let src1_off = eval.next_trace_mask();
        let dst_off = eval.next_trace_mask();
        let op0_val_0 = eval.next_trace_mask();
        let op0_val_1 = eval.next_trace_mask();
        let op0_val_2 = eval.next_trace_mask();
        let op0_val_3 = eval.next_trace_mask();
        let op0_prev_clock_lo = eval.next_trace_mask();
        let op0_prev_clock_hi = eval.next_trace_mask();
        let op1_val_0 = eval.next_trace_mask();
        let op1_val_1 = eval.next_trace_mask();
        let op1_val_2 = eval.next_trace_mask();
        let op1_val_3 = eval.next_trace_mask();
        let op1_prev_clock_lo = eval.next_trace_mask();
        let op1_prev_clock_hi = eval.next_trace_mask();
        let dst_prev_val_lo = eval.next_trace_mask();
        let dst_prev_val_hi = eval.next_trace_mask();
        let res_0 = eval.next_trace_mask();
        let res_1 = eval.next_trace_mask();
        let res_2 = eval.next_trace_mask();
        let res_3 = eval.next_trace_mask();
        let dst_prev_clock_lo = eval.next_trace_mask();
        let dst_prev_clock_hi = eval.next_trace_mask();

        let two_pow_8 = E::F::from(BaseField::from(1 << 8));
        let one = E::F::one();

        // Constraint: enabler is boolean
        eval.add_constraint(enabler.clone() * (enabler.clone() - one.clone()));

        let bitwise_op = opcode_constant.clone() - E::F::from(BaseField::from(U32_STORE_AND_FP_FP));

        let op0_val_lo = op0_val_0.clone() + op0_val_1.clone() * two_pow_8.clone();
        let op0_val_hi = op0_val_2.clone() + op0_val_3.clone() * two_pow_8.clone();
        let op1_val_lo = op1_val_0.clone() + op1_val_1.clone() * two_pow_8.clone();
        let op1_val_hi = op1_val_2.clone() + op1_val_3.clone() * two_pow_8.clone();
        let res_lo = res_0.clone() + res_1.clone() * two_pow_8.clone();
        let res_hi = res_2.clone() + res_3.clone() * two_pow_8;

        // Register lookups
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            E::EF::from(-enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            E::EF::from(enabler.clone()),
            &[pc.clone() + one.clone(), fp.clone()],
        ));

        // Read instruction
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
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

        // Read op0
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
            &[
                fp.clone() + src0_off.clone(),
                op0_prev_clock_lo.clone(),
                op0_val_lo.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + src0_off.clone(), clock.clone(), op0_val_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
            &[
                fp.clone() + src0_off.clone() + one.clone(),
                op0_prev_clock_hi.clone(),
                op0_val_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src0_off + one.clone(),
                clock.clone(),
                op0_val_hi,
            ],
        ));

        // Read op1
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
            &[
                fp.clone() + src1_off.clone(),
                op1_prev_clock_lo.clone(),
                op1_val_lo.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + src1_off.clone(), clock.clone(), op1_val_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
            &[
                fp.clone() + src1_off.clone() + one.clone(),
                op1_prev_clock_hi.clone(),
                op1_val_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src1_off + one.clone(),
                clock.clone(),
                op1_val_hi,
            ],
        ));

        // Write dst
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
            &[
                fp.clone() + dst_off.clone(),
                dst_prev_clock_lo.clone(),
                dst_prev_val_lo,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + dst_off.clone(), clock.clone(), res_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(-enabler.clone()),
            &[
                fp.clone() + dst_off.clone() + one.clone(),
                dst_prev_clock_hi.clone(),
                dst_prev_val_hi,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp + dst_off + one, clock.clone(), res_hi],
        ));

        // Bitwise lookups
        eval.add_to_relation(RelationEntry::new(
            &self.relations.bitwise,
            -E::EF::one(),
            &[bitwise_op.clone(), op0_val_0, op1_val_0, res_0],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.bitwise,
            -E::EF::one(),
            &[bitwise_op.clone(), op0_val_1, op1_val_1, res_1],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.bitwise,
            -E::EF::one(),
            &[bitwise_op.clone(), op0_val_2, op1_val_2, res_2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.bitwise,
            -E::EF::one(),
            &[bitwise_op, op0_val_3, op1_val_3, res_3],
        ));

        // Range checks
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
            &[clock.clone() - op1_prev_clock_lo - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - op1_prev_clock_hi - enabler.clone()],
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
