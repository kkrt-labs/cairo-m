//! This component is used to prove the U32StoreDivFpImm opcode.
//! u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) / u32(imm_lo, imm_hi)
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - src_off
//! - imm_0 (d_0)
//! - imm_1 (d_1)
//! - imm_2 (d_2)
//! - imm_3 (d_3)
//! - dst_off
//! - op0_val_lo (n_lo)
//! - op0_val_hi (n_hi)
//! - op0_prev_clock_lo
//! - op0_prev_clock_hi
//! - dst_prev_val_lo
//! - dst_prev_val_hi
//! - dst_prev_clock_lo
//! - dst_prev_clock_hi
//! - q_0
//! - q_1
//! - q_2
//! - q_3
//! - mul_carry_0
//! - mul_carry_1
//! - mul_carry_2
//! - mul_carry_3
//! - mul_carry_4
//! - mul_carry_5
//! - mul_carry_6
//! - prod_0
//! - prod_1
//! - prod_2
//! - prod_3
//! - prod_4
//! - prod_5
//! - prod_6
//! - prod_7
//! - add_carry_0
//! - add_carry_1
//! - add_carry_2
//! - add_carry_3
//! - sub_borrow_0
//! - sub_borrow_1
//! - r_lo
//! - r_hi
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * add_carry_{i<3} and sub_borrow_{j<2} are bools
//!   * `- add_carry_0 * (1 - add_carry_0)`
//!   * `- add_carry_1 * (1 - add_carry_1)`
//!   * `- add_carry_2 * (1 - add_carry_2)`
//!   * `- sub_borrow_0 * (1 - sub_borrow_0)`
//! * registers update is regular (+2 because of the two-worded instruction)
//!   * `- [pc, fp] + [pc + 2, fp]` in `Registers` relation
//! * read 2 instruction words from memory
//!   * `- [pc, inst_prev_clk, src_off, imm_lo, imm_hi] + [pc, clk, src_off, imm_lo, imm_hi]` in `Memory` relation
//!   * `- [pc + 1, inst_prev_clk, dst_off] + [pc + 1, clk, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read op0
//!   * `- [fp + src_off, op0_prev_clock_lo_clk, op0_val_lo] + [fp + src_off, clk, op0_val_lo]`
//!   * `- [fp + src_off + 1, op0_prev_clock_hi_clk, op0_val_hi] + [fp + src_off + 1, clk, op0_val_hi]`
//!   * `- [clk - op0_prev_clock_lo_clk - 1]` and `- [clk - op0_prev_clock_hi_clk - 1]` in `RangeCheck20` relation
//! * prove that prod = q * d (u32 * u32 -> u64)
//!   * `prod_0 - (q_0 * d_0 - mul_carry_0 * 2 ** 8)`
//!   * `prod_1 - (q_0 * d_1 + q_1 * d_0 + mul_carry_0 - mul_carry_1 * 2 ** 8)`
//!   * `prod_2 - (q_0 * d_2 + q_2 * d_0 + q_1 * d_1 + mul_carry_1 - mul_carry_2 * 2 ** 8)`
//!   * `prod_3 - (q_0 * d_3 + q_3 * d_0 + q_1 * d_2 + q_2 * d_1 + mul_carry_2 - mul_carry_3 * 2 ** 8)`
//!   * `prod_4 - (q_1 * d_3 + q_3 * d_1 + q_2 * d_2 + mul_carry_3 - mul_carry_4 * 2 ** 8)`
//!   * `prod_5 - (q_2 * d_3 + q_3 * d_2 + mul_carry_4 - mul_carry_5 * 2 ** 8)`
//!   * `prod_6 - (q_3 * d_3 + mul_carry_5 - mul_carry_6 * 2 ** 8)`
//!   * `prod_7 - mul_carry_6`
//! * carry limbs must be in the correct range
//!   * `- [254 - mul_carry_0]` in `RangeCheck16` relation
//!   * `- [509 - mul_carry_1]` in `RangeCheck16` relation
//!   * `- [764 - mul_carry_2]` in `RangeCheck16` relation
//!   * `- [1019 - mul_carry_3]` in `RangeCheck16` relation
//!   * `- [1274 - mul_carry_4]` in `RangeCheck16` relation
//!   * `- [512 - mul_carry_5]` in `RangeCheck16` relation
//!   * `- [256 - mul_carry_6]` in `RangeCheck16` relation
//! * prove that n = prod + r
//!   * `n_lo - (prod_0 + prod_1 * 2 ** 8 + r_lo - add_carry_0 * 2 ** 16)`
//!   * `n_hi - (prod_2 + prod_3 * 2 ** 8 + r_hi + add_carry_0 - add_carry_1 * 2 ** 16)`
//!   * `(prod_4 + prod_5 * 2 ** 8 + add_carry_1 - add_carry_2 * 2 ** 16)`
//!   * `(prod_6 + prod_7 * 2 ** 8 + add_carry_2 - add_carry_3 * 2 ** 16)`
//!   * `add_carry_3`
//! * prove that r < d by showing that the u32 operation d - r - 1 doesn't underflow
//!   * `- [d_0 + d_1 * 2 ** 8 + sub_borrow_0 * 2 ** 16 - r_lo - 1]` in `RangeCheck16` relation
//!   * `- [d_2 + d_3 * 2 ** 8 + sub_borrow_1 * 2 ** 16 - r_hi]` in `RangeCheck16` relation
//!   * `sub_borrow_1`
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clock_lo_clk, dst_prev_val_lo] + [fp + dst_off, clk, q_0 + q_1 * 2 ** 8]` in `Memory` Relation
//!   * `- [fp + dst_off + 1, dst_prev_clock_hi_clk, dst_prev_val_hi] + [fp + dst_off + 1, clk, q_2 + q_3 * 2 ** 8]` in `Memory` Relation
//!   * `- [clk - dst_prev_clock_lo_clk - 1]` and `- [clk - dst_prev_clock_hi_clk - 1]` in `RangeCheck20` relation
//! * limbs of each U32 must be either in range [0, 2^16) or in range [0, 2^8)
//!   * `- [d_0]` in `RangeCheck8` relation
//!   * `- [d_1]` in `RangeCheck8` relation
//!   * `- [d_2]` in `RangeCheck8` relation
//!   * `- [d_3]` in `RangeCheck8` relation
//!   * `- [q_0]` in `RangeCheck8` relation
//!   * `- [q_1]` in `RangeCheck8` relation
//!   * `- [q_2]` in `RangeCheck8` relation
//!   * `- [q_3]` in `RangeCheck8` relation
//!   * `- [prod_0]` in `RangeCheck8` relation
//!   * `- [prod_1]` in `RangeCheck8` relation
//!   * `- [prod_2]` in `RangeCheck8` relation
//!   * `- [prod_3]` in `RangeCheck8` relation
//!   * `- [prod_4]` in `RangeCheck8` relation
//!   * `- [prod_5]` in `RangeCheck8` relation
//!   * `- [prod_6]` in `RangeCheck8` relation
//!   * `- [prod_7]` in `RangeCheck8` relation
//!   * `- [n_lo]` in `RangeCheck16` relation
//!   * `- [n_hi]` in `RangeCheck16` relation
//!   * `- [r_lo]` in `RangeCheck16` relation
//!   * `- [r_hi]` in `RangeCheck16` relation

use crate::utils::data_accesses::{get_prev_clock, get_prev_value, get_value};
use cairo_m_common::instruction::U32_STORE_DIV_REM_FP_IMM;
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

use crate::adapter::{memory::DataAccess, ExecutionBundle};
use crate::components::Relations;
use crate::preprocessed::bitwise::BitwiseProvider;
use crate::preprocessed::range_check::RangeCheckProvider;
use crate::utils::enabler::Enabler;
use crate::utils::execution_bundle::PackedExecutionBundle;

const N_TRACE_COLUMNS: usize = 46;
const N_MEMORY_LOOKUPS: usize = 12;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_8_LOOKUPS: usize = 16;
const N_RANGE_CHECK_16_LOOKUPS: usize = 13;
const N_RANGE_CHECK_20_LOOKUPS: usize = 5;

// 1 * (255 * 255) = 254 * 2^8 + 1
// 2 * (255 * 255) + 254 = 509 * 2^8
// 3 * (255 * 255) + 509 = 764 * 2^8
// 4 * (255 * 255) + 764 = 1019 * 2^8
// 3 * (255 * 255) + 1019 = 765 * 2^8 + 254
// 2 * (255 * 255) + 765 = 510 * 2^8 + 255
// 1 * (255 * 255) + 510 = 255 * 2^8 + 255
const MAX_CARRY_0: u32 = 254;
const MAX_CARRY_1: u32 = 509;
const MAX_CARRY_2: u32 = 764;
const MAX_CARRY_3: u32 = 1019;
const MAX_CARRY_4: u32 = 765;
const MAX_CARRY_5: u32 = 510;
const MAX_CARRY_6: u32 = 255;

const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_MEMORY_LOOKUPS
        + N_REGISTERS_LOOKUPS
        + N_RANGE_CHECK_20_LOOKUPS
        + N_RANGE_CHECK_16_LOOKUPS
        + N_RANGE_CHECK_8_LOOKUPS)
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
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
    pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
    pub range_check_16: [Vec<PackedM31>; N_RANGE_CHECK_16_LOOKUPS],
    pub range_check_8: [Vec<PackedM31>; N_RANGE_CHECK_8_LOOKUPS],
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

    /// Writes the trace for the StoreDivFpImm opcode.
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

                let opcode_constant = PackedM31::from(M31::from(U32_STORE_DIV_REM_FP_IMM));
                let src_off = input.inst_value_1;
                let imm_lo = input.inst_value_2;
                let imm_hi = input.inst_value_3;
                let dst_off = input.inst_value_4;

                // Operand 0 (u32) comes from two separate memory reads
                let n_lo = get_value(input, data_accesses, 0); // numerator low
                let n_hi = get_value(input, data_accesses, 1); // numerator high
                let op0_prev_clock_lo = get_prev_clock(input, data_accesses, 0);
                let op0_prev_clock_hi = get_prev_clock(input, data_accesses, 1);

                // Destination (u32) previous values and clocks for each limb
                let dst_prev_val_lo = get_prev_value(input, data_accesses, 2);
                let dst_prev_val_hi = get_prev_value(input, data_accesses, 3);
                let dst_prev_clock_lo = get_prev_clock(input, data_accesses, 2);
                let dst_prev_clock_hi = get_prev_clock(input, data_accesses, 3);

                // Decompose immediate (divisor) into 8-bit limbs: d = d_0 + d_1*2^8 + d_2*2^16 + d_3*2^24
                let two_pow_8 = PackedM31::from(M31::from(1 << 8));
                let decompose_8 = |val: PackedM31| -> (PackedM31, PackedM31) {
                    let lo = PackedM31::from_array(val.to_array().map(|x| M31::from(x.0 & 0xFF)));
                    let hi =
                        PackedM31::from_array(val.to_array().map(|x| M31::from((x.0 >> 8) & 0xFF)));
                    (lo, hi)
                };

                let (d_0, d_1) = decompose_8(imm_lo);
                let (d_2, d_3) = decompose_8(imm_hi);

                // Compute quotient and remainder
                let n_lo_array = n_lo.to_array();
                let n_hi_array = n_hi.to_array();
                let imm_lo_array = imm_lo.to_array();
                let imm_hi_array = imm_hi.to_array();

                // Convert to u32 iterators and perform euclidean division
                let n_u32_iter = n_lo_array
                    .iter()
                    .zip(n_hi_array.iter())
                    .map(|(lo, hi)| (lo.0 | (hi.0 << 16)));
                let d_u32_iter = imm_lo_array
                    .iter()
                    .zip(imm_hi_array.iter())
                    .map(|(lo, hi)| (lo.0 | (hi.0 << 16)));
                let q_r_u32_iter =
                    n_u32_iter.zip(d_u32_iter).map(
                        |(n, d)| {
                            if d == 0 {
                                (0, 0)
                            } else {
                                (n / d, n % d)
                            }
                        },
                    );

                // Convert quotient and remainder back to PackedM31
                let q_lo = PackedM31::from_array(
                    q_r_u32_iter
                        .clone()
                        .map(|(q, _)| M31::from(q & 0xFFFF))
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );
                let q_hi = PackedM31::from_array(
                    q_r_u32_iter
                        .clone()
                        .map(|(q, _)| M31::from(q >> 16))
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );
                let r_lo = PackedM31::from_array(
                    q_r_u32_iter
                        .clone()
                        .map(|(_, r)| M31::from(r & 0xFFFF))
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );
                let r_hi = PackedM31::from_array(
                    q_r_u32_iter
                        .map(|(_, r)| M31::from(r >> 16))
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );

                let (q_0, q_1) = decompose_8(q_lo);
                let (q_2, q_3) = decompose_8(q_hi);

                // Compute multiplication products for verification: q * d
                // Using 8-bit limbs to prevent overflow
                let prod_0 = q_0 * d_0;
                let prod_1_raw = q_0 * d_1 + q_1 * d_0;
                let prod_2_raw = q_0 * d_2 + q_2 * d_0 + q_1 * d_1;
                let prod_3_raw = q_0 * d_3 + q_3 * d_0 + q_1 * d_2 + q_2 * d_1;
                let prod_4_raw = q_1 * d_3 + q_3 * d_1 + q_2 * d_2;
                let prod_5_raw = q_2 * d_3 + q_3 * d_2;
                let prod_6_raw = q_3 * d_3;

                // Compute carries for multiplication
                let mul_carry_0 =
                    PackedM31::from_array(prod_0.to_array().map(|x| M31::from(x.0 >> 8)));
                let prod_1_with_carry = prod_1_raw + mul_carry_0;
                let mul_carry_1 = PackedM31::from_array(
                    prod_1_with_carry.to_array().map(|x| M31::from(x.0 >> 8)),
                );
                let prod_2_with_carry = prod_2_raw + mul_carry_1;
                let mul_carry_2 = PackedM31::from_array(
                    prod_2_with_carry.to_array().map(|x| M31::from(x.0 >> 8)),
                );
                let prod_3_with_carry = prod_3_raw + mul_carry_2;
                let mul_carry_3 = PackedM31::from_array(
                    prod_3_with_carry.to_array().map(|x| M31::from(x.0 >> 8)),
                );
                let prod_4_with_carry = prod_4_raw + mul_carry_3;
                let mul_carry_4 = PackedM31::from_array(
                    prod_4_with_carry.to_array().map(|x| M31::from(x.0 >> 8)),
                );
                let prod_5_with_carry = prod_5_raw + mul_carry_4;
                let mul_carry_5 = PackedM31::from_array(
                    prod_5_with_carry.to_array().map(|x| M31::from(x.0 >> 8)),
                );
                let prod_6_with_carry = prod_6_raw + mul_carry_5;
                let mul_carry_6 = PackedM31::from_array(
                    prod_6_with_carry.to_array().map(|x| M31::from(x.0 >> 8)),
                );

                // Final product limbs (8-bit)
                let prod_0 = prod_0 - mul_carry_0 * two_pow_8;
                let prod_1 = prod_1_with_carry - mul_carry_1 * two_pow_8;
                let prod_2 = prod_2_with_carry - mul_carry_2 * two_pow_8;
                let prod_3 = prod_3_with_carry - mul_carry_3 * two_pow_8;
                let prod_4 = prod_4_with_carry - mul_carry_4 * two_pow_8;
                let prod_5 = prod_5_with_carry - mul_carry_5 * two_pow_8;
                let prod_6 = prod_6_with_carry - mul_carry_6 * two_pow_8;
                let prod_7 = mul_carry_6;

                // Compute addition carries for n = prod + r
                let add_0_raw = prod_0 + prod_1 * two_pow_8 + r_lo;
                let add_carry_0 = PackedM31::from_array(add_0_raw.to_array().map(|x| {
                    if x.0 > 0xFFFF {
                        M31::one()
                    } else {
                        M31::zero()
                    }
                }));
                let add_1_raw = prod_2 + prod_3 * two_pow_8 + r_hi + add_carry_0;
                let add_carry_1 = PackedM31::from_array(add_1_raw.to_array().map(|x| {
                    if x.0 > 0xFFFF {
                        M31::one()
                    } else {
                        M31::zero()
                    }
                }));
                let add_2_raw = prod_4 + prod_5 * two_pow_8 + add_carry_1;
                let add_carry_2 = PackedM31::from_array(add_2_raw.to_array().map(|x| {
                    if x.0 > 0xFFFF {
                        M31::one()
                    } else {
                        M31::zero()
                    }
                }));
                let add_3_raw = prod_6 + prod_7 * two_pow_8 + add_carry_2;
                let add_carry_3 = PackedM31::from_array(add_3_raw.to_array().map(|x| {
                    if x.0 > 0xFFFF {
                        M31::one()
                    } else {
                        M31::zero()
                    }
                }));

                // Compute subtraction borrows for r < d check
                let sub_borrow_0 = PackedM31::from_array(
                    d_0.to_array()
                        .iter()
                        .zip(d_1.to_array().iter())
                        .zip(r_lo.to_array().iter())
                        .map(|((d0, d1), r)| {
                            let d_val = d0.0 | (d1.0 << 8);
                            if d_val < r.0 + 1 {
                                M31::one()
                            } else {
                                M31::zero()
                            }
                        })
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );
                let sub_borrow_1 = PackedM31::from_array(
                    d_2.to_array()
                        .iter()
                        .zip(d_3.to_array().iter())
                        .zip(r_hi.to_array().iter())
                        .zip(sub_borrow_0.to_array().iter())
                        .map(|(((d2, d3), r), b)| {
                            let d_val = d2.0 | (d3.0 << 8);
                            if d_val < r.0 + b.0 {
                                M31::one()
                            } else {
                                M31::zero()
                            }
                        })
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );

                // Result is q_0 + q_1 * 2^8 for lo, q_2 + q_3 * 2^8 for hi
                let res_lo = q_0 + q_1 * two_pow_8;
                let res_hi = q_2 + q_3 * two_pow_8;

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = src_off;
                *row[6] = d_0; // imm_0
                *row[7] = d_1; // imm_1
                *row[8] = d_2; // imm_2
                *row[9] = d_3; // imm_3
                *row[10] = dst_off;
                *row[11] = n_lo; // op0_val_lo
                *row[12] = n_hi; // op0_val_hi
                *row[13] = op0_prev_clock_lo;
                *row[14] = op0_prev_clock_hi;
                *row[15] = dst_prev_val_lo;
                *row[16] = dst_prev_val_hi;
                *row[17] = dst_prev_clock_lo;
                *row[18] = dst_prev_clock_hi;
                *row[19] = q_0;
                *row[20] = q_1;
                *row[21] = q_2;
                *row[22] = q_3;
                *row[23] = mul_carry_0;
                *row[24] = mul_carry_1;
                *row[25] = mul_carry_2;
                *row[26] = mul_carry_3;
                *row[27] = mul_carry_4;
                *row[28] = mul_carry_5;
                *row[29] = mul_carry_6;
                *row[30] = prod_0;
                *row[31] = prod_1;
                *row[32] = prod_2;
                *row[33] = prod_3;
                *row[34] = prod_4;
                *row[35] = prod_5;
                *row[36] = prod_6;
                *row[37] = prod_7;
                *row[38] = add_carry_0;
                *row[39] = add_carry_1;
                *row[40] = add_carry_2;
                *row[41] = add_carry_3;
                *row[42] = sub_borrow_0;
                *row[43] = sub_borrow_1;
                *row[44] = r_lo;
                *row[45] = r_hi;

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
                *lookup_data.memory[4] = [fp + src_off, op0_prev_clock_lo, n_lo, zero, zero, zero];
                *lookup_data.memory[5] = [fp + src_off, clock, n_lo, zero, zero, zero];

                // Read op0_hi
                *lookup_data.memory[6] = [
                    fp + src_off + one,
                    op0_prev_clock_hi,
                    n_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[7] = [fp + src_off + one, clock, n_hi, zero, zero, zero];

                // Write dst_lo
                *lookup_data.memory[8] = [
                    fp + dst_off,
                    dst_prev_clock_lo,
                    dst_prev_val_lo,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[9] = [fp + dst_off, clock, res_lo, zero, zero, zero];

                // Write dst_hi
                *lookup_data.memory[10] = [
                    fp + dst_off + one,
                    dst_prev_clock_hi,
                    dst_prev_val_hi,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[11] = [fp + dst_off + one, clock, res_hi, zero, zero, zero];

                // Range checks as specified in the AIR
                // 8-bit range checks
                *lookup_data.range_check_8[0] = d_0;
                *lookup_data.range_check_8[1] = d_1;
                *lookup_data.range_check_8[2] = d_2;
                *lookup_data.range_check_8[3] = d_3;
                *lookup_data.range_check_8[4] = q_0;
                *lookup_data.range_check_8[5] = q_1;
                *lookup_data.range_check_8[6] = q_2;
                *lookup_data.range_check_8[7] = q_3;
                *lookup_data.range_check_8[8] = prod_0;
                *lookup_data.range_check_8[9] = prod_1;
                *lookup_data.range_check_8[10] = prod_2;
                *lookup_data.range_check_8[11] = prod_3;
                *lookup_data.range_check_8[12] = prod_4;
                *lookup_data.range_check_8[13] = prod_5;
                *lookup_data.range_check_8[14] = prod_6;
                *lookup_data.range_check_8[15] = prod_7;

                // 16-bit range checks
                *lookup_data.range_check_16[0] = n_lo;
                *lookup_data.range_check_16[1] = n_hi;
                *lookup_data.range_check_16[2] = r_lo;
                *lookup_data.range_check_16[3] = r_hi;

                // Carry limbs must be in the correct range
                let max_carry_0 = PackedM31::from(M31::from(MAX_CARRY_0));
                let max_carry_1 = PackedM31::from(M31::from(MAX_CARRY_1));
                let max_carry_2 = PackedM31::from(M31::from(MAX_CARRY_2));
                let max_carry_3 = PackedM31::from(M31::from(MAX_CARRY_3));
                let max_carry_4 = PackedM31::from(M31::from(MAX_CARRY_4));
                let max_carry_5 = PackedM31::from(M31::from(MAX_CARRY_5));
                let max_carry_6 = PackedM31::from(M31::from(MAX_CARRY_6));
                *lookup_data.range_check_16[4] = max_carry_0 - mul_carry_0;
                *lookup_data.range_check_16[5] = max_carry_1 - mul_carry_1;
                *lookup_data.range_check_16[6] = max_carry_2 - mul_carry_2;
                *lookup_data.range_check_16[7] = max_carry_3 - mul_carry_3;
                *lookup_data.range_check_16[8] = max_carry_4 - mul_carry_4;
                *lookup_data.range_check_16[9] = max_carry_5 - mul_carry_5;
                *lookup_data.range_check_16[10] = max_carry_6 - mul_carry_6;

                // Subtraction checks for r < d
                let sub_check_lo = d_0 + d_1 * two_pow_8 + sub_borrow_0 * two_pow_16 - r_lo - one;
                let sub_check_hi =
                    d_2 + d_3 * two_pow_8 + sub_borrow_1 * two_pow_16 - r_hi - sub_borrow_0;
                *lookup_data.range_check_16[11] = sub_check_lo;
                *lookup_data.range_check_16[12] = sub_check_hi;

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
                &interaction_claim_data.lookup_data.memory[2 * i],
                &interaction_claim_data.lookup_data.memory[2 * i + 1],
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

        // Range checks 8
        for i in 0..N_RANGE_CHECK_8_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.range_check_8[2 * i],
                &interaction_claim_data.lookup_data.range_check_8[2 * i + 1],
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

        // Range checks 16
        for i in 0..N_RANGE_CHECK_16_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.range_check_16[2 * i],
                &interaction_claim_data.lookup_data.range_check_16[2 * i + 1],
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

        // Range checks 20
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_16[12],
            &interaction_claim_data.lookup_data.range_check_20[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, val1, val2))| {
                let num = -PackedQM31::one();
                let denom_0: PackedQM31 = relations.range_check_16.combine(&[*val1]);
                let denom_1: PackedQM31 = relations.range_check_20.combine(&[*val2]);

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
            .for_each(|(_i, (writer, val1, val2))| {
                let num = -PackedQM31::one();
                let denom_0: PackedQM31 = relations.range_check_20.combine(&[*val1]);
                let denom_1: PackedQM31 = relations.range_check_20.combine(&[*val2]);

                let numerator = num * denom_1 + num * denom_0;
                let denom = denom_0 * denom_1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.range_check_20[3],
            &interaction_claim_data.lookup_data.range_check_20[4],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(_i, (writer, val1, val2))| {
                let num = -PackedQM31::one();
                let denom_0: PackedQM31 = relations.range_check_20.combine(&[*val1]);
                let denom_1: PackedQM31 = relations.range_check_20.combine(&[*val2]);

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
        let two_pow_8 = E::F::from(M31::from(1 << 8));
        let two_pow_16 = E::F::from(M31::from(1 << 16));
        let opcode_constant = E::F::from(M31::from(U32_STORE_DIV_REM_FP_IMM));

        // 46 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let src_off = eval.next_trace_mask();
        let d_0 = eval.next_trace_mask(); // imm_0 (divisor limb 0)
        let d_1 = eval.next_trace_mask(); // imm_1 (divisor limb 1)
        let d_2 = eval.next_trace_mask(); // imm_2 (divisor limb 2)
        let d_3 = eval.next_trace_mask(); // imm_3 (divisor limb 3)
        let dst_off = eval.next_trace_mask();
        let n_lo = eval.next_trace_mask(); // op0_val_lo (numerator low)
        let n_hi = eval.next_trace_mask(); // op0_val_hi (numerator high)
        let op0_prev_clock_lo = eval.next_trace_mask();
        let op0_prev_clock_hi = eval.next_trace_mask();
        let dst_prev_val_lo = eval.next_trace_mask();
        let dst_prev_val_hi = eval.next_trace_mask();
        let dst_prev_clock_lo = eval.next_trace_mask();
        let dst_prev_clock_hi = eval.next_trace_mask();
        let q_0 = eval.next_trace_mask(); // quotient limb 0
        let q_1 = eval.next_trace_mask(); // quotient limb 1
        let q_2 = eval.next_trace_mask(); // quotient limb 2
        let q_3 = eval.next_trace_mask(); // quotient limb 3
        let mul_carry_0 = eval.next_trace_mask();
        let mul_carry_1 = eval.next_trace_mask();
        let mul_carry_2 = eval.next_trace_mask();
        let mul_carry_3 = eval.next_trace_mask();
        let mul_carry_4 = eval.next_trace_mask();
        let mul_carry_5 = eval.next_trace_mask();
        let mul_carry_6 = eval.next_trace_mask();
        let prod_0 = eval.next_trace_mask();
        let prod_1 = eval.next_trace_mask();
        let prod_2 = eval.next_trace_mask();
        let prod_3 = eval.next_trace_mask();
        let prod_4 = eval.next_trace_mask();
        let prod_5 = eval.next_trace_mask();
        let prod_6 = eval.next_trace_mask();
        let prod_7 = eval.next_trace_mask();
        let add_carry_0 = eval.next_trace_mask();
        let add_carry_1 = eval.next_trace_mask();
        let add_carry_2 = eval.next_trace_mask();
        let add_carry_3 = eval.next_trace_mask();
        let sub_borrow_0 = eval.next_trace_mask();
        let sub_borrow_1 = eval.next_trace_mask();
        let r_lo = eval.next_trace_mask(); // remainder low
        let r_hi = eval.next_trace_mask(); // remainder high

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Add carries and sub borrows are 0 or 1 (according to AIR spec lines 57-60)
        eval.add_constraint(
            enabler.clone() * add_carry_0.clone() * (one.clone() - add_carry_0.clone()),
        );
        eval.add_constraint(
            enabler.clone() * add_carry_1.clone() * (one.clone() - add_carry_1.clone()),
        );
        eval.add_constraint(
            enabler.clone() * add_carry_2.clone() * (one.clone() - add_carry_2.clone()),
        );
        eval.add_constraint(
            enabler.clone() * sub_borrow_0.clone() * (one.clone() - sub_borrow_0.clone()),
        );

        // Reconstitute the immediate (divisor) value from 8-bit limbs
        let imm_lo = d_0.clone() + d_1.clone() * two_pow_8.clone();
        let imm_hi = d_2.clone() + d_3.clone() * two_pow_8.clone();

        // Product verification constraints: prove that prod = q * d
        eval.add_constraint(
            enabler.clone()
                * (q_0.clone() * d_0.clone()
                    - mul_carry_0.clone() * two_pow_8.clone()
                    - prod_0.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (q_0.clone() * d_1.clone() + q_1.clone() * d_0.clone() + mul_carry_0.clone()
                    - mul_carry_1.clone() * two_pow_8.clone()
                    - prod_1.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (q_0.clone() * d_2.clone()
                    + q_2.clone() * d_0.clone()
                    + q_1.clone() * d_1.clone()
                    + mul_carry_1.clone()
                    - mul_carry_2.clone() * two_pow_8.clone()
                    - prod_2.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (q_0.clone() * d_3.clone()
                    + q_3.clone() * d_0.clone()
                    + q_1.clone() * d_2.clone()
                    + q_2.clone() * d_1.clone()
                    + mul_carry_2.clone()
                    - mul_carry_3.clone() * two_pow_8.clone()
                    - prod_3.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (q_1.clone() * d_3.clone()
                    + q_3.clone() * d_1.clone()
                    + q_2.clone() * d_2.clone()
                    + mul_carry_3.clone()
                    - mul_carry_4.clone() * two_pow_8.clone()
                    - prod_4.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (q_2.clone() * d_3.clone() + q_3.clone() * d_2.clone() + mul_carry_4.clone()
                    - mul_carry_5.clone() * two_pow_8.clone()
                    - prod_5.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (q_3.clone() * d_3.clone() + mul_carry_5.clone()
                    - mul_carry_6.clone() * two_pow_8.clone()
                    - prod_6.clone()),
        );

        // mul_carry_6 = prod_7
        eval.add_constraint(enabler.clone() * (mul_carry_6.clone() - prod_7.clone()));

        // Prove that n = prod + r
        eval.add_constraint(
            enabler.clone()
                * (n_lo.clone()
                    - (prod_0.clone() + prod_1.clone() * two_pow_8.clone() + r_lo.clone()
                        - add_carry_0.clone() * two_pow_16.clone())),
        );

        eval.add_constraint(
            enabler.clone()
                * (n_hi.clone()
                    - (prod_2.clone()
                        + prod_3.clone() * two_pow_8.clone()
                        + r_hi.clone()
                        + add_carry_0
                        - add_carry_1.clone() * two_pow_16.clone())),
        );

        eval.add_constraint(
            enabler.clone()
                * (prod_4.clone() + prod_5.clone() * two_pow_8.clone() + add_carry_1
                    - add_carry_2.clone() * two_pow_16.clone()),
        );

        eval.add_constraint(
            enabler.clone()
                * (prod_6.clone() + prod_7.clone() * two_pow_8.clone() + add_carry_2
                    - add_carry_3.clone() * two_pow_16.clone()),
        );

        eval.add_constraint(enabler.clone() * add_carry_3);

        // Prove that r < d by checking d - r - 1 doesn't underflow
        // sub_borrow_1 must be 0 for the subtraction to not underflow and sub_check_lo and hi must be u16
        let sub_check_lo = d_0.clone()
            + d_1.clone() * two_pow_8.clone()
            + sub_borrow_0.clone() * two_pow_16.clone()
            - r_lo.clone()
            - one.clone();
        let sub_check_hi =
            d_2.clone() + d_3.clone() * two_pow_8.clone() + sub_borrow_1.clone() * two_pow_16
                - r_hi.clone()
                - sub_borrow_0;
        eval.add_constraint(enabler.clone() * sub_borrow_1);

        // Result is the quotient
        let res_lo = q_0.clone() + q_1.clone() * two_pow_8.clone();
        let res_hi = q_2.clone() + q_3.clone() * two_pow_8;

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
                imm_lo,
                imm_hi,
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

        // Read n_lo (numerator low)
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone(),
                op0_prev_clock_lo.clone(),
                n_lo.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + src_off.clone(), clock.clone(), n_lo.clone()],
        ));

        // Read n_hi (numerator high)
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone() + one.clone(),
                op0_prev_clock_hi.clone(),
                n_hi.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off + one.clone(),
                clock.clone(),
                n_hi.clone(),
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
            &[fp.clone() + dst_off.clone(), clock.clone(), res_lo],
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
            &[fp + dst_off + one, clock.clone(), res_hi],
        ));

        // Range checks for 8-bit values (d_i, q_i, prod_i)
        for val in &[d_0, d_1, d_2, d_3, q_0, q_1, q_2, q_3] {
            eval.add_to_relation(RelationEntry::new(
                &self.relations.range_check_8,
                -E::EF::one(),
                &[val.clone()],
            ));
        }

        // Range checks for product limbs (8-bit)
        for prod in &[
            prod_0, prod_1, prod_2, prod_3, prod_4, prod_5, prod_6, prod_7,
        ] {
            eval.add_to_relation(RelationEntry::new(
                &self.relations.range_check_8,
                -E::EF::one(),
                &[prod.clone()],
            ));
        }

        // Range check 16 for all 16-bit limbs
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[n_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[n_hi],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[r_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[r_hi],
        ));

        // Range checks for mul_carry values using RangeCheck16
        let max_carry_0 = E::F::from(M31::from(MAX_CARRY_0));
        let max_carry_1 = E::F::from(M31::from(MAX_CARRY_1));
        let max_carry_2 = E::F::from(M31::from(MAX_CARRY_2));
        let max_carry_3 = E::F::from(M31::from(MAX_CARRY_3));
        let max_carry_4 = E::F::from(M31::from(MAX_CARRY_4));
        let max_carry_5 = E::F::from(M31::from(MAX_CARRY_5));
        let max_carry_6 = E::F::from(M31::from(MAX_CARRY_6));

        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_0 - mul_carry_0],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_1 - mul_carry_1],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_2 - mul_carry_2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_3 - mul_carry_3],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_4 - mul_carry_4],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_5 - mul_carry_5],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[max_carry_6 - mul_carry_6],
        ));

        // Range checks for subtraction verification: d - r - 1
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[sub_check_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[sub_check_hi],
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
