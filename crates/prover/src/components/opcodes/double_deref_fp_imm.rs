//! This component is used to prove the StoreToDoubleDerefFp and StoreDoubleDerefFp opcodes:
//! * [[fp + base_off] + imm] = [fp + src_off]
//! * [fp + dst_off] = [[fp + base_off] + imm]
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - opcode_constant
//! - off0
//! - off1
//! - off2
//! - val0
//! - prev_clock0
//! - addr1
//! - val1
//! - prev_clock1
//! - addr2
//! - prev_val2
//! - prev_clock2
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * write_lhs is a bool
//!   * `write_lhs * (1 - write_lhs)`
//! * write_lhs is correctly computed
//!   * let write_lhs = (opcode_constant - STORE_DOUBLE_DEREF_FP_IMM) * delta_inv
//!   * `write_lhs * (1 - write_lhs)`
//! * registers update is regular
//!   * `- [pc, fp, clock] + [pc + 1, fp, clock + 1]` in `Registers` relation
//! * read instruction from memory
//!   * `- [pc, inst_prev_clk, opcode_constant, off0, off1, off2] + [pc, clk, opcode_constant, off0, off1, off2]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck20` relation
//! * read val0
//!   * `- [fp + off0, prev_clock0, val0] + [fp + off0, clk, val0]` in `Memory` relation
//!   * `- [clk - prev_clock0 - 1]` in `RangeCheck20` relation
//! * read val1
//!   * `addr1 - write_lhs * (fp + off2) - (1 - write_lhs) * (val0 + off1)`
//!   * `- [addr1, prev_clock1, val1] + [addr1, clk, val1]` in `Memory` relation
//!   * `- [clk - prev_clock1 - 1]` in `RangeCheck20` relation
//! * write val1 in [addr2]
//!   * `addr2 - write_lhs * (val0 + off1) - (1 - write_lhs) * (fp + off2)`
//!   * `- [addr2, prev_clock2, prev_val2] + [addr2, clk, val1]` in `Memory` relation
//!   * `- [clk - prev_clock2 - 1]` in `RangeCheck20` relation

use cairo_m_common::instruction::{STORE_DOUBLE_DEREF_FP, STORE_TO_DOUBLE_DEREF_FP_IMM};
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
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{LOG_N_LANES, N_LANES, PackedM31};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SECURE_EXTENSION_DEGREE, SecureField};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::adapter::ExecutionBundle;
use crate::adapter::memory::DataAccess;
use crate::components::Relations;
use crate::preprocessed::bitwise::BitwiseProvider;
use crate::preprocessed::range_check::RangeCheckProvider;
use crate::utils::data_accesses::{get_prev_clock, get_prev_value, get_value};
use crate::utils::enabler::Enabler;
use crate::utils::execution_bundle::PackedExecutionBundle;

const N_TRACE_COLUMNS: usize = 17;
const N_MEMORY_LOOKUPS: usize = 8;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 4;

const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_MEMORY_LOOKUPS + N_REGISTERS_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS).div_ceil(2);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

// Implement RangeCheckProvider to expose range_check_20 data
impl RangeCheckProvider for InteractionClaimData {
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

    /// Writes the trace for the StoreDoubleDerefFp and StoreToDoubleDerefFp opcodes.
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
        let store_double_deref_fp = PackedM31::from(M31::from(STORE_DOUBLE_DEREF_FP));
        let delta_inv = PackedM31::from(
            M31::from(STORE_TO_DOUBLE_DEREF_FP_IMM - STORE_DOUBLE_DEREF_FP).inverse(),
        );
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
                        M31::from(STORE_DOUBLE_DEREF_FP)
                    } else {
                        x
                    }
                }));
                let off0 = input.inst_value_1;
                let off1 = input.inst_value_2;
                let off2 = input.inst_value_3;
                let val0 = get_value(input, data_accesses, 0);
                let prev_clock0 = get_prev_clock(input, data_accesses, 0);
                let val1 = get_value(input, data_accesses, 1);
                let prev_clock1 = get_prev_clock(input, data_accesses, 1);
                let prev_val2 = get_prev_value(input, data_accesses, 2);
                let prev_clock2 = get_prev_clock(input, data_accesses, 2);

                let write_lhs = (opcode_constant - store_double_deref_fp) * delta_inv;
                let addr1 = write_lhs * (fp + off2) + (one - write_lhs) * (val0 + off1);
                let addr2 = write_lhs * (val0 + off1) + (one - write_lhs) * (fp + off2);

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = opcode_constant;
                *row[6] = off0;
                *row[7] = off1;
                *row[8] = off2;
                *row[9] = val0;
                *row[10] = prev_clock0;
                *row[11] = addr1;
                *row[12] = val1;
                *row[13] = prev_clock1;
                *row[14] = addr2;
                *row[15] = prev_val2;
                *row[16] = prev_clock2;

                *lookup_data.registers[0] = [input.pc, input.fp, input.clock];
                *lookup_data.registers[1] = [input.pc + one, input.fp, input.clock + one];

                *lookup_data.memory[0] =
                    [input.pc, inst_prev_clock, opcode_constant, off0, off1, off2];
                *lookup_data.memory[1] = [input.pc, clock, opcode_constant, off0, off1, off2];

                *lookup_data.memory[2] = [fp + off0, prev_clock0, val0, zero, zero, zero];
                *lookup_data.memory[3] = [fp + off0, clock, val0, zero, zero, zero];

                *lookup_data.memory[4] = [addr1, prev_clock1, val1, zero, zero, zero];
                *lookup_data.memory[5] = [addr1, clock, val1, zero, zero, zero];

                *lookup_data.memory[6] = [addr2, prev_clock2, prev_val2, zero, zero, zero];
                *lookup_data.memory[7] = [addr2, clock, val1, zero, zero, zero];

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
                *lookup_data.range_check_20[1] = clock - prev_clock0 - enabler;
                *lookup_data.range_check_20[2] = clock - prev_clock1 - enabler;
                *lookup_data.range_check_20[3] = clock - prev_clock2 - enabler;
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

        // Range check 20 lookups
        for i in 0..N_RANGE_CHECK_20_LOOKUPS / 2 {
            let mut col = interaction_trace.new_col();
            (
                col.par_iter_mut(),
                &interaction_claim_data.lookup_data.range_check_20[2 * i],
                &interaction_claim_data.lookup_data.range_check_20[2 * i + 1],
            )
                .into_par_iter()
                .for_each(|(writer, range_check_20_prev, range_check_20_new)| {
                    let num_prev = -PackedQM31::one();
                    let denom_prev: PackedQM31 =
                        relations.range_check_20.combine(&[*range_check_20_prev]);
                    let denom_new: PackedQM31 =
                        relations.range_check_20.combine(&[*range_check_20_new]);

                    let numerator = num_prev * (denom_prev + denom_new);
                    let denom = denom_prev * denom_new;

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
        let delta_inv =
            E::F::from(M31::from(STORE_TO_DOUBLE_DEREF_FP_IMM - STORE_DOUBLE_DEREF_FP).inverse());
        let store_double_deref_fp = E::F::from(M31::from(STORE_DOUBLE_DEREF_FP));

        // 17 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let opcode_constant = eval.next_trace_mask();
        let off0 = eval.next_trace_mask();
        let off1 = eval.next_trace_mask();
        let off2 = eval.next_trace_mask();
        let val0 = eval.next_trace_mask();
        let prev_clock0 = eval.next_trace_mask();
        let addr1 = eval.next_trace_mask();
        let val1 = eval.next_trace_mask();
        let prev_clock1 = eval.next_trace_mask();
        let addr2 = eval.next_trace_mask();
        let prev_val2 = eval.next_trace_mask();
        let prev_clock2 = eval.next_trace_mask();

        let write_lhs = (opcode_constant.clone() - store_double_deref_fp) * delta_inv;

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // write_lhs is a bool
        eval.add_constraint(write_lhs.clone() * (one.clone() - write_lhs.clone()));

        // addr1 is correctly computed
        eval.add_constraint(
            enabler.clone()
                * (addr1.clone()
                    - write_lhs.clone() * (fp.clone() + off2.clone())
                    - (one.clone() - write_lhs.clone()) * (val0.clone() + off1.clone())),
        );

        // addr2 is correctly computed
        eval.add_constraint(
            enabler.clone()
                * (addr2.clone()
                    - write_lhs.clone() * (val0.clone() + off1.clone())
                    - (one.clone() - write_lhs) * (fp.clone() + off2.clone())),
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
                off0.clone(),
                off1.clone(),
                off2.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[pc, clock.clone(), opcode_constant, off0.clone(), off1, off2],
        ));

        // Read val0
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[fp.clone() + off0.clone(), prev_clock0.clone(), val0.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[fp + off0, clock.clone(), val0],
        ));

        // Read val1
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[addr1.clone(), prev_clock1.clone(), val1.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[addr1, clock.clone(), val1.clone()],
        ));

        // Write val1 in [addr2]
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            -E::EF::from(enabler.clone()),
            &[addr2.clone(), prev_clock2.clone(), prev_val2],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(enabler.clone()),
            &[addr2, clock.clone(), val1],
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
            &[clock.clone() - prev_clock0 - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock.clone() - prev_clock1 - enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.range_check_20,
            -E::EF::one(),
            &[clock - prev_clock2 - enabler],
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
