//! This component is used to prove the StoreXFpImm opcodes.
//!
//! [fp + off2] = [fp + off0] + imm : StoreAddFpImm
//! [fp + off2] = [fp + off0] - imm : StoreSubFpImm
//! [fp + off2] = [fp + off0] * imm : StoreMulFpImm
//! [fp + off2] = [fp + off0] / imm : StoreDivFpImm
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - off0
//! - off1
//! - off2
//! - op0_prev_clock
//! - op0_val
//! - off1_inv
//! - dst_prev_clock
//! - dst_prev_val
//! - dst_val
//! - opcode_flag_0
//! - opcode_flag_1
//! - prod
//! - div
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * opcode_flag_0 is a bool
//!   * `opcode_flag_0 * (1 - opcode_flag_0)`
//! * opcode_flag_1 is a bool
//!   * `opcode_flag_1 * (1 - opcode_flag_1)`
//! * prod is the product of op0 and off1
//!   * `prod - op0 * off1`
//! * div is the division of op0 and off1
//!   * `div - op0 * off1_inv`
//! * off1_inv is the inverse of off1 or off1 is 0
//!   * `off1 * (off1_inv * off1 - 1)`
//!   * `off1_inv * (off1_inv * off1 - 1)`
//! * dst_val is the result of the operation
//!   * `dst_val - (1 - opcode_flag_0) * (1 - opcode_flag_1) * (op0 + off1) // (0, 0) => StoreAddFpImm
//!   * `    - (1 - opcode_flag_0) * opcode_flag_1 * (op0 - off1) // (0, 1) => StoreSubFpImm
//!   * `    - opcode_flag_0 * (1 - opcode_flag_1) * prod // (1, 0) => StoreMulFpImm
//!   * `    - opcode_flag_0 * opcode_flag_1 * div // (1, 1) => StoreDivFpImm
//! * registers update is regular
//!   * `- [pc, fp] + [pc + 1, fp]` in `Registers` relation
//! * read instruction from memory
//!   * `opcode_id - (base_opcode + opcode_flag_0 * 2 + opcode_flag_1)`
//!   * `- [pc, inst_prev_clk, opcode_id, off0, off1, off2] + [pc, clk, opcode_id, off0, off1, off2]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read op0
//!   * `- [fp + off0, op0_prev_clk, op0_val] + [fp + off0, clk, op0_val]` in `Memory` relation
//!   * `- [clk - op0_prev_clk - 1]` in `RangeCheck20` relation
//! * write dst in [fp + off2]
//!   * `- [fp + off2, dst_prev_clk, dst_prev_val] + [fp + off2, clk, dst_val]` in `Memory` relation
//!   * `- [clk - dst_prev_clk - 1]` in `RangeCheck20` relation

use cairo_m_common::instruction::{RET, STORE_ADD_FP_IMM};
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
use crate::components::opcodes::RangeCheckProvider;
use crate::components::Relations;
use crate::preprocessed::range_check::RangeCheckProvider;
use crate::utils::enabler::Enabler;
use crate::utils::execution_bundle::PackedExecutionBundle;

const N_TRACE_COLUMNS: usize = 18;
const N_MEMORY_LOOKUPS: usize = 6;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 3;

const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_MEMORY_LOOKUPS + N_REGISTERS_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS).div_ceil(2);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

impl RangeCheckProvider for InteractionClaimData {}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
    pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
}

// Implement RangeCheckProvider to expose range_check_20 data
impl RangeCheckProvider for InteractionClaimData {
    fn get_range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.lookup_data.range_check_20.par_iter().flatten()
    }
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

    /// Writes the trace for the StoreXFpImm opcodes.
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
        let log_size = std::cmp::max(LOG_N_LANES, non_padded_length.next_power_of_two().ilog2());

        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
                LookupData::uninitialized(log_size - LOG_N_LANES),
            )
        };
        inputs.resize(1 << log_size, ExecutionBundle::default());
        let packed_inputs: Vec<(PackedExecutionBundle, PackedM31, PackedM31, PackedM31)> = inputs
            .par_chunks_exact(N_LANES)
            .map(|chunk| {
                let array: [ExecutionBundle; N_LANES] = chunk.try_into().unwrap();
                let off1_inverses = PackedM31::from_array(array.map(|x| {
                    let off1 = if x.instruction.instruction.opcode_value() == RET {
                        M31::zero()
                    } else {
                        x.instruction.instruction.operands()[1]
                    };
                    if off1 != M31::zero() {
                        off1.inverse()
                    } else {
                        M31::zero()
                    }
                }));
                let opcode_flag_0 = PackedM31::from_array(array.map(|x| {
                    let flag = x
                        .instruction
                        .instruction
                        .opcode_value()
                        .saturating_sub(STORE_ADD_FP_IMM);
                    M31(flag / 2)
                }));
                let opcode_flag_1 = PackedM31::from_array(array.map(|x| {
                    let flag = x
                        .instruction
                        .instruction
                        .opcode_value()
                        .saturating_sub(STORE_ADD_FP_IMM);
                    M31(flag % 2)
                }));
                (
                    Pack::pack(array),
                    off1_inverses,
                    opcode_flag_0,
                    opcode_flag_1,
                )
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
            .for_each(
                |(
                    row_index,
                    (mut row, (input, off1_inverses, opcode_flag_0, opcode_flag_1), lookup_data),
                )| {
                    let enabler = enabler_col.packed_at(row_index);
                    let pc = input.pc;
                    let fp = input.fp;
                    let clock = input.clock;
                    let inst_prev_clock = input.inst_prev_clock;
                    let opcode_id = input.inst_value_0;
                    let off0 = input.inst_value_1;
                    let off1 = input.inst_value_2;
                    let off2 = input.inst_value_3;
                    let op0_prev_clock = input.mem1_prev_clock;
                    let op0_val = input.mem1_value_limb0;
                    let off1_inv = *off1_inverses;
                    let dst_prev_clock = input.mem2_prev_clock;
                    let dst_prev_val = input.mem2_prev_value_limb0;
                    let dst_val = input.mem2_value_limb0;
                    let prod = op0_val * off1;
                    let div = op0_val * *off1_inverses;

                    *row[0] = enabler;
                    *row[1] = pc;
                    *row[2] = fp;
                    *row[3] = clock;
                    *row[4] = inst_prev_clock;
                    *row[5] = off0;
                    *row[6] = off1;
                    *row[7] = off2;
                    *row[8] = op0_prev_clock;
                    *row[9] = op0_val;
                    *row[10] = off1_inv;
                    *row[11] = dst_prev_clock;
                    *row[12] = dst_prev_val;
                    *row[13] = dst_val;
                    *row[14] = *opcode_flag_0 * enabler;
                    *row[15] = *opcode_flag_1 * enabler;
                    *row[16] = prod;
                    *row[17] = div;

                    *lookup_data.registers[0] = [input.pc, input.fp];
                    *lookup_data.registers[1] = [input.pc + one, input.fp];

                    *lookup_data.memory[0] =
                        [input.pc, inst_prev_clock, opcode_id, off0, off1, off2];
                    *lookup_data.memory[1] = [input.pc, clock, opcode_id, off0, off1, off2];

                    *lookup_data.memory[2] = [fp + off0, op0_prev_clock, op0_val, zero, zero, zero];
                    *lookup_data.memory[3] = [fp + off0, clock, op0_val, zero, zero, zero];

                    *lookup_data.memory[4] =
                        [fp + off2, dst_prev_clock, dst_prev_val, zero, zero, zero];
                    *lookup_data.memory[5] = [fp + off2, clock, dst_val, zero, zero, zero];

                    *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                    *lookup_data.range_check_20[1] = clock - op0_prev_clock - enabler;
                    *lookup_data.range_check_20[2] = clock - dst_prev_clock - enabler;
                },
            );

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

        // 18 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let off0 = eval.next_trace_mask();
        let off1 = eval.next_trace_mask();
        let off2 = eval.next_trace_mask();
        let op0_prev_clock = eval.next_trace_mask();
        let op0_val = eval.next_trace_mask();
        let off1_inv = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();
        let dst_prev_val = eval.next_trace_mask();
        let dst_val = eval.next_trace_mask();
        let opcode_flag_0 = eval.next_trace_mask();
        let opcode_flag_1 = eval.next_trace_mask();
        let prod = eval.next_trace_mask();
        let div = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // opcode_flag_0 is 0 or 1
        eval.add_constraint(opcode_flag_0.clone() * (one.clone() - opcode_flag_0.clone()));

        // opcode_flag_1 is 0 or 1
        eval.add_constraint(opcode_flag_1.clone() * (one.clone() - opcode_flag_1.clone()));

        // prod is op0 * off1
        eval.add_constraint(prod.clone() - op0_val.clone() * off1.clone());

        // off1_inv is the inverse of off1 or off1 is 0
        eval.add_constraint(off1.clone() * (off1_inv.clone() * off1.clone() - one.clone()));

        // off1_inv is the inverse of off1 or off1_inv is 0
        eval.add_constraint(off1_inv.clone() * (off1_inv.clone() * off1.clone() - one.clone()));

        // div is op0 / off1
        eval.add_constraint(div.clone() - op0_val.clone() * off1_inv);

        // dst_val is
        // Add: (1 - opcode_flag_0) * (1 - opcode_flag_1) * (op0 + off1)
        // Sub: (1 - opcode_flag_0) * opcode_flag_1 * (op0 - off1)
        // Mul: opcode_flag_0 * (1 - opcode_flag_1) * prod
        // Div: opcode_flag_0 * opcode_flag_1 * div
        let is_add = eval.add_intermediate(
            (one.clone() - opcode_flag_0.clone()) * (one.clone() - opcode_flag_1.clone()),
        );
        let is_sub =
            eval.add_intermediate((one.clone() - opcode_flag_0.clone()) * opcode_flag_1.clone());
        let is_mul =
            eval.add_intermediate(opcode_flag_0.clone() * (one.clone() - opcode_flag_1.clone()));
        let is_div = eval.add_intermediate(opcode_flag_0.clone() * opcode_flag_1.clone());
        let opcode_id = eval.add_intermediate(
            E::F::from(M31::from(STORE_ADD_FP_IMM))
                + E::F::from(M31::from_u32_unchecked(2)) * opcode_flag_0
                + opcode_flag_1,
        );
        let res = eval.add_intermediate(
            is_add * (op0_val.clone() + off1.clone())
                + is_sub * (op0_val.clone() - off1.clone())
                + is_mul * prod
                + is_div * div,
        );
        eval.add_constraint(dst_val.clone() - res);

        // Registers update
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.registers,
            E::EF::from(enabler.clone()),
            &[pc.clone() + one, fp.clone()],
        ));

        // Read instruction from memory
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                inst_prev_clock.clone(),
                opcode_id.clone(),
                off0.clone(),
                off1.clone(),
                off2.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[
                pc,
                clock.clone(),
                opcode_id,
                off0.clone(),
                off1,
                off2.clone(),
            ],
        ));

        // Read op0
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off0.clone(),
                op0_prev_clock.clone(),
                op0_val.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + off0, clock.clone(), op0_val],
        ));

        // Write dst
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off2.clone(),
                dst_prev_clock.clone(),
                dst_prev_val,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp + off2, clock.clone(), dst_val],
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
