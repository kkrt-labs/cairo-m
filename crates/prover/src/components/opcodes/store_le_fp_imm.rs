//! This component is used to prove the StoreLeFpImm opcode.
//! [fp + dst_off] = [fp + src_off] <= imm
//!
//! Math from https://github.com/starkware-libs/cairo-lang/blob/v0.14.0.1/src/starkware/cairo/common/math.cairo#L161-L228
//!
//! The argument is basically:
//! - let a and b be two field elements such that
//!   ... P - 1 | 0 ------- a ---- b --------- P - 1 | 0 ------- a ---- b --------- P - 1 | 0 ...
//! - guess two numbers `arc_short` and `arc_long` that are respectively "almost smaller" than P / 3 and P / 2 by construction
//! - consequently their sum is smaller than P
//! - prove that `{arc_short, arc_long}` is one of these three tuples:
//!   - `{a, b - a}`
//!   - `{a, P - 1 - b}`
//!   - `{b - a, P - 1 - b}`
//! - the set equality is proved by asserting that sum and prod of the two members are equal
//!
//! The "almost smaller" actually means that they are smaller than P // n + 2*2**16:
//!
//! - P = n q_n + r_n; q_n = q_n_high * 2**16 + q_n_low;
//! - hint (0 <= a_0 < 2**16, 0 <= a_1 < 2**16) and build:
//! ```text
//!  a_0 + (q_n_high + 1) * a_1
//!   = a_0 + a_1 + q_n_high * a_1
//!   < q_n + 2**17 == P // n + 2**17
//! ```
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - src_off
//! - imm
//! - dst_off
//! - src_val
//! - src_prev_clock
//! - dst_prev_val
//! - dst_prev_clock
//! - a
//! - b
//! - keep_0_1
//! - keep_0_2
//! - keep_1_2
//! - arc_short_lo
//! - arc_short_hi
//! - arc_long_lo
//! - arc_long_hi
//! - is_le
//!
//! # Constraints
//!
//! PRIME_OVER_3_HIGH = ceil(PRIME / 3 / 2 ** 16)
//! PRIME_OVER_2_HIGH = ceil(PRIME / 2 / 2 ** 16)
//! arc_short = arc_short_lo + arc_short_hi * PRIME_OVER_3_HIGH
//! arc_long = arc_long_lo + arc_long_hi * PRIME_OVER_2_HIGH
//! arc_sum = arc_short + arc_long
//! arc_prod = arc_short * arc_long
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * keep_{i} is a bool
//!   * `keep_0_1 * (1 - keep_0_1)`
//!   * `keep_0_2 * (1 - keep_0_2)`
//!   * `keep_1_2 * (1 - keep_1_2)`
//! * only one of keep_0_1, keep_0_2, keep_1_2 is equal to 1
//!   * `enabler * (keep_0_1 + keep_0_2 + keep_1_2 - 1)`
//! * is_le is a bool
//!   * `is_le * (1 - is_le)`
//! * enforce that 2 of the 3 arcs are arc_short_lo and arc_short_hi
//!   * `keep_0_1 * (arc_sum - (a + b - a))`
//!   * `keep_0_1 * (arc_prod - a * (b - a))`
//!   * `keep_0_2 * (arc_sum - (a + P - 1 - b))`
//!   * `keep_0_2 * (arc_prod - a * (P - 1 - b))`
//!   * `keep_1_2 * (arc_sum - (b - a + P - 1 - b))`
//!   * `keep_1_2 * (arc_prod - (b - a) * (P - 1 - b))`
//! * rebuild imm and src_val from a and b
//!   * `a - is_le * src_val - (1 - is_le) * imm`
//!   * `b - is_le * imm - (1 - is_le) * src_val`
//! * registers update is regular
//!   * `- [pc, fp, clock] + [pc + 1, fp, clock + 1]` in `Registers` relation
//! * read instruction from memory
//!   * `- [pc, inst_prev_clk, opcode_constant, src_off, imm, dst_off] + [pc, clk, opcode_constant, src_off, imm, dst_off]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read src
//!   * `- [fp + src_off, src_prev_clk, src_val] + [fp + src_off, clk, src_val]` in `Memory` relation
//!   * `- [clk - src_prev_clk - 1]` in `RangeCheck20` relation
//! * write dst in [fp + dst_off]
//!   * `- [fp + dst_off, dst_prev_clk, dst_prev_val] + [fp + dst_off, clk, is_le]` in `Memory` Relation
//!   * `- [clk - dst_prev_clk - 1]` in `RangeCheck20` relation
//! * range check arc limbs
//!   * `- [arc_short_lo]` in `RangeCheck16` relation
//!   * `- [arc_short_hi]` in `RangeCheck16` relation
//!   * `- [arc_long_lo]` in `RangeCheck16` relation
//!   * `- [arc_long_hi]` in `RangeCheck16` relation

use cairo_m_common::instruction::STORE_LE_FP_IMM;
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
use stwo_prover::core::fields::m31::{BaseField, M31, P};
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

const PRIME_OVER_3_HIGH: u32 = ((P / 3) >> 16) + 1;
const PRIME_OVER_2_HIGH: u32 = ((P / 2) >> 16) + 1;

const N_TRACE_COLUMNS: usize = 22;
const N_MEMORY_LOOKUPS: usize = 6;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 3;
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

// Implement RangeCheckProvider to expose range_check_20 and range_check_16 data
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

    /// Writes the trace for the StoreLeFpImm opcode.
    ///
    /// # Important
    /// This function consumes the contents of `inputs` by clearing it after processing.
    /// This is done to free memory during proof generation as the inputs are no longer needed
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
                let opcode_constant = PackedM31::from(M31::from(STORE_LE_FP_IMM));
                let src_off = input.inst_value_1;
                let imm = input.inst_value_2;
                let dst_off = input.inst_value_3;

                // Get source value from memory
                let src_val = get_value(input, data_accesses, 0);
                let src_prev_clock = get_prev_clock(input, data_accesses, 0);

                // Get destination previous value
                let dst_prev_val = get_prev_value(input, data_accesses, 1);
                let dst_prev_clock = get_prev_clock(input, data_accesses, 1);

                // Simple comparison logic for packed values
                let mut is_le: [M31; N_LANES] = [M31::zero(); N_LANES];

                let a = PackedM31::from_array(
                    src_val
                        .to_array()
                        .iter()
                        .zip(imm.to_array().iter())
                        .enumerate()
                        .map(|(i, (x, y))| {
                            if x.0 <= y.0 {
                                is_le[i] = M31::one();
                                *x
                            } else {
                                *y
                            }
                        })
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );
                let b = PackedM31::from_array(
                    src_val
                        .to_array()
                        .iter()
                        .zip(imm.to_array().iter())
                        .map(|(x, y)| if x.0 <= y.0 { *y } else { *x })
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                );
                let is_le = PackedM31::from_array(is_le);

                // Compute arcs for range checking the comparison
                // We need to prove that a <= b using the arc method
                // This partitions the circle into 3 arcs and ensures 2 of them fit in specific ranges

                // Compute the two shortest arc lengths and their indices

                let mut arc_short_lo_val: [M31; N_LANES] = [M31::zero(); N_LANES];
                let mut arc_short_hi_val: [M31; N_LANES] = [M31::zero(); N_LANES];
                let mut arc_long_lo_val: [M31; N_LANES] = [M31::zero(); N_LANES];
                let mut arc_long_hi_val: [M31; N_LANES] = [M31::zero(); N_LANES];
                let mut keep_0_1: [M31; N_LANES] = [M31::zero(); N_LANES];
                let mut keep_0_2: [M31; N_LANES] = [M31::zero(); N_LANES];
                let mut keep_1_2: [M31; N_LANES] = [M31::zero(); N_LANES];

                for (i, lane) in (0..N_LANES).enumerate() {
                    let a_val = a.to_array()[lane].0;
                    let b_val = b.to_array()[lane].0;

                    // Three arc lengths: a, b-a, P-1-b
                    let mut lengths_and_indices = [
                        (a_val, 0u8),
                        (b_val.saturating_sub(a_val), 1u8),
                        (P - 1 - b_val, 2u8),
                    ];

                    // Sort by length
                    lengths_and_indices.sort_by_key(|x| x.0);

                    // The longest arc is excluded
                    let exclude = lengths_and_indices[2].1;

                    arc_short_lo_val[i] = M31::from(lengths_and_indices[0].0 % PRIME_OVER_3_HIGH);
                    arc_short_hi_val[i] = M31::from(lengths_and_indices[0].0 / PRIME_OVER_3_HIGH);
                    arc_long_lo_val[i] = M31::from(lengths_and_indices[1].0 % PRIME_OVER_2_HIGH);
                    arc_long_hi_val[i] = M31::from(lengths_and_indices[1].0 / PRIME_OVER_2_HIGH);
                    if enabler.to_array()[i] == M31::one() {
                        keep_0_1[i] = M31::from(u32::from(exclude == 2));
                        keep_0_2[i] = M31::from(u32::from(exclude == 1));
                        keep_1_2[i] = M31::from(u32::from(exclude == 0));
                    }
                }

                // Pack the results
                let arc_short_lo = PackedM31::from_array(arc_short_lo_val);
                let arc_short_hi = PackedM31::from_array(arc_short_hi_val);
                let arc_long_lo = PackedM31::from_array(arc_long_lo_val);
                let arc_long_hi = PackedM31::from_array(arc_long_hi_val);
                let keep_0_1 = PackedM31::from_array(keep_0_1);
                let keep_0_2 = PackedM31::from_array(keep_0_2);
                let keep_1_2 = PackedM31::from_array(keep_1_2);

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = src_off;
                *row[6] = imm;
                *row[7] = dst_off;
                *row[8] = src_val;
                *row[9] = src_prev_clock;
                *row[10] = dst_prev_val;
                *row[11] = dst_prev_clock;
                *row[12] = a;
                *row[13] = b;
                *row[14] = keep_0_1;
                *row[15] = keep_0_2;
                *row[16] = keep_1_2;
                *row[17] = arc_short_lo;
                *row[18] = arc_short_hi;
                *row[19] = arc_long_lo;
                *row[20] = arc_long_hi;
                *row[21] = is_le;

                *lookup_data.registers[0] = [input.pc, input.fp, input.clock];
                *lookup_data.registers[1] = [input.pc + one, input.fp, input.clock + one];

                // Read instruction
                *lookup_data.memory[0] = [
                    input.pc,
                    inst_prev_clock,
                    opcode_constant,
                    src_off,
                    imm,
                    dst_off,
                ];
                *lookup_data.memory[1] = [input.pc, clock, opcode_constant, src_off, imm, dst_off];

                // Read source value
                *lookup_data.memory[2] = [fp + src_off, src_prev_clock, src_val, zero, zero, zero];
                *lookup_data.memory[3] = [fp + src_off, clock, src_val, zero, zero, zero];

                // Write destination
                *lookup_data.memory[4] =
                    [fp + dst_off, dst_prev_clock, dst_prev_val, zero, zero, zero];
                *lookup_data.memory[5] = [fp + dst_off, clock, is_le, zero, zero, zero];

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - src_prev_clock - enabler;
                *lookup_data.range_check_20[2] = clock - dst_prev_clock - enabler;

                // Range check arc limbs
                *lookup_data.range_check_16[0] = arc_short_lo;
                *lookup_data.range_check_16[1] = arc_short_hi;
                *lookup_data.range_check_16[2] = arc_long_lo;
                *lookup_data.range_check_16[3] = arc_long_hi;
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
                    let denom = denom_1 * denom_0;

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
        col.par_iter_mut()
            .zip(&interaction_claim_data.lookup_data.range_check_20[2])
            .for_each(|(writer, range_check_20_2)| {
                let num = -PackedQM31::one();
                let denom: PackedQM31 = relations.range_check_20.combine(&[*range_check_20_2]);
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
        let opcode_constant = E::F::from(M31::from(STORE_LE_FP_IMM));

        // Constants for arc computation
        let prime_over_3_high = E::F::from(M31::from(PRIME_OVER_3_HIGH));
        let prime_over_2_high = E::F::from(M31::from(PRIME_OVER_2_HIGH));

        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let src_off = eval.next_trace_mask();
        let imm = eval.next_trace_mask();
        let dst_off = eval.next_trace_mask();
        let src_val = eval.next_trace_mask();
        let src_prev_clock = eval.next_trace_mask();
        let dst_prev_val = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();
        let a = eval.next_trace_mask();
        let b = eval.next_trace_mask();
        let keep_0_1 = eval.next_trace_mask();
        let keep_0_2 = eval.next_trace_mask();
        let keep_1_2 = eval.next_trace_mask();
        let arc_short_lo = eval.next_trace_mask();
        let arc_short_hi = eval.next_trace_mask();
        let arc_long_lo = eval.next_trace_mask();
        let arc_long_hi = eval.next_trace_mask();
        let is_le = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // keep_0_1 is 1 or 0
        eval.add_constraint(keep_0_1.clone() * (one.clone() - keep_0_1.clone()));
        // keep_0_2 is 1 or 0
        eval.add_constraint(keep_0_2.clone() * (one.clone() - keep_0_2.clone()));
        // keep_1_2 is 1 or 0
        eval.add_constraint(keep_1_2.clone() * (one.clone() - keep_1_2.clone()));

        // Only one of keep flags must be 1
        eval.add_constraint(
            enabler.clone()
                * (keep_0_1.clone() + keep_0_2.clone() + keep_1_2.clone() - one.clone()),
        );

        // is_le is 1 or 0
        eval.add_constraint(is_le.clone() * (one.clone() - is_le.clone()));

        // Arc computations
        let arc_short = arc_short_lo.clone() + arc_short_hi.clone() * prime_over_3_high;
        let arc_long = arc_long_lo.clone() + arc_long_hi.clone() * prime_over_2_high;
        let arc_sum = arc_short.clone() + arc_long.clone();
        let arc_prod = arc_short * arc_long;

        // Arc constraints based on keep flags
        eval.add_constraint(
            keep_0_1.clone() * (arc_sum.clone() - (a.clone() + b.clone() - a.clone())),
        );
        eval.add_constraint(keep_0_1 * (arc_prod.clone() - a.clone() * (b.clone() - a.clone())));
        eval.add_constraint(
            keep_0_2.clone() * (arc_sum.clone() - (a.clone() - one.clone() - b.clone())),
        );
        eval.add_constraint(keep_0_2 * (arc_prod.clone() - a.clone() * (-one.clone() - b.clone())));
        eval.add_constraint(
            keep_1_2.clone() * (arc_sum - (b.clone() - a.clone() - one.clone() - b.clone())),
        );
        eval.add_constraint(
            keep_1_2 * (arc_prod - (b.clone() - a.clone()) * (-one.clone() - b.clone())),
        );

        // Assert (a, b) = (src_val, imm) if is_le else (imm, src_val)
        eval.add_constraint(
            enabler.clone()
                * (a - is_le.clone() * src_val.clone()
                    - (one.clone() - is_le.clone()) * imm.clone()),
        );
        eval.add_constraint(
            enabler.clone()
                * (b - is_le.clone() * imm.clone()
                    - (one.clone() - is_le.clone()) * src_val.clone()),
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
            &[pc.clone() + one.clone(), fp.clone(), clock.clone() + one],
        ));

        // Read instruction from memory
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                inst_prev_clock.clone(),
                opcode_constant.clone(),
                src_off.clone(),
                imm.clone(),
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
                src_off.clone(),
                imm,
                dst_off.clone(),
            ],
        ));

        // Read source value
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + src_off.clone(),
                src_prev_clock.clone(),
                src_val.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + src_off, clock.clone(), src_val],
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
            &[fp + dst_off, clock.clone(), is_le],
        ));

        // Range check 16 for arc limbs
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[arc_short_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[arc_short_hi],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[arc_long_lo],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_16,
            -E::EF::one(),
            &[arc_long_hi],
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
            &[clock.clone() - src_prev_clock - enabler.clone()],
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
