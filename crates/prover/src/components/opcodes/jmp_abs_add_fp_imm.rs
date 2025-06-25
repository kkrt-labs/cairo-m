//! This component is used to prove the JmpAbsAddFpImm opcode.
//! jmp abs [fp + off0] + off1
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - opcode_id
//! - off0
//! - off1
//! - off2
//! - op0_prev_clock
//! - op0_val
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * registers update is regular
//!   * `- [pc, fp] + [op0_val + off1, fp]` in `Registers` relation
//! * read instruction from memory
//!   * `- [pc, inst_prev_clk, opcode_id, off0, off1, off2] + [pc, clk, opcode_id, off0, off1, off2]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck_20` relation
//! * assert opcode id
//!   * `opcode_id - 17`
//! * read op0
//!   * `- [fp + off0, op0_prev_clk, op0_val] + [fp + off0, clk, op0_val]`
//!   * `- [clk - op0_prev_clk - 1]` in `RangeCheck_20` relation

use cairo_m_common::Opcode;
use num_traits::{One, Zero};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::adapter::StateData;
use crate::relations;
use crate::utils::{Enabler, PackedStateData};

const N_TRACE_COLUMNS: usize = 11;
const N_MEMORY_LOOKUPS: usize = 4;
const N_REGISTERS_LOOKUPS: usize = 2;
const N_RANGE_CHECK_20_LOOKUPS: usize = 2;

const N_LOOKUPS_COLUMNS: usize =
    SECURE_EXTENSION_DEGREE * (N_MEMORY_LOOKUPS + N_REGISTERS_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
    pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
}

#[derive(Clone, Default, Serialize, Deserialize)]
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

    pub fn write_trace<MC: MerkleChannel>(
        inputs: &mut Vec<StateData>,
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
        inputs.resize(1 << log_size, StateData::default());
        let packed_inputs: Vec<PackedStateData> = inputs
            .chunks(N_LANES)
            .map(|chunk| {
                let array: [StateData; N_LANES] = chunk.try_into().unwrap();
                Pack::pack(array)
            })
            .collect();

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
                let clock = input.mem0_clock;
                let inst_prev_clock = input.mem0_prev_clock;
                let opcode_id = input.mem0_value_0;
                let off0 = input.mem0_value_1;
                let off1 = input.mem0_value_2;
                let off2 = input.mem0_value_3;
                let op0_prev_clock = input.mem1_prev_clock;
                let op0_val = input.mem1_value_0;

                *row[0] = enabler;
                *row[1] = pc;
                *row[2] = fp;
                *row[3] = clock;
                *row[4] = inst_prev_clock;
                *row[5] = opcode_id;
                *row[6] = off0;
                *row[7] = off1;
                *row[8] = off2;
                *row[9] = op0_prev_clock;
                *row[10] = op0_val;

                *lookup_data.registers[0] = [input.pc, input.fp];
                *lookup_data.registers[1] = [op0_val + off1, input.fp];

                *lookup_data.memory[0] = [input.pc, inst_prev_clock, opcode_id, off0, off1, off2];
                *lookup_data.memory[1] = [input.pc, clock, opcode_id, off0, off1, off2];

                *lookup_data.memory[2] = [fp + off0, op0_prev_clock, op0_val, zero, zero, zero];
                *lookup_data.memory[3] = [fp + off0, clock, op0_val, zero, zero, zero];

                *lookup_data.range_check_20[0] = clock - inst_prev_clock - one;
                *lookup_data.range_check_20[1] = clock - op0_prev_clock - one;
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

#[derive(Clone, Serialize, Deserialize)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}
impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        registers_relation: &relations::Registers,
        memory_relation: &relations::Memory,
        range_check_20_relation: &relations::RangeCheck_20,
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
                let denom_prev: PackedQM31 = registers_relation.combine(registers_prev);
                let denom_new: PackedQM31 = registers_relation.combine(registers_new);

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
                let denom_prev: PackedQM31 = memory_relation.combine(memory_prev);
                let denom_new: PackedQM31 = memory_relation.combine(memory_new);

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
                let denom_prev: PackedQM31 = memory_relation.combine(memory_prev);
                let denom_new: PackedQM31 = memory_relation.combine(memory_new);

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
            .for_each(|(i, (writer, range_check_20_0, range_check_20_1))| {
                let num = -PackedQM31::from(enabler_col.packed_at(i));
                let denom_0: PackedQM31 = range_check_20_relation.combine(&[*range_check_20_0]);
                let denom_1: PackedQM31 = range_check_20_relation.combine(&[*range_check_20_1]);

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
    pub memory: relations::Memory,
    pub registers: relations::Registers,
    pub range_check_20: relations::RangeCheck_20,
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
        let expected_opcode_id = E::F::from(M31::from(Opcode::JmpAbsAddFpImm));

        // 11 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let opcode_id = eval.next_trace_mask();
        let off0 = eval.next_trace_mask();
        let off1 = eval.next_trace_mask();
        let off2 = eval.next_trace_mask();
        let op0_prev_clock = eval.next_trace_mask();
        let op0_val = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Opcode id is JmpAbsAddFpImm
        eval.add_constraint(enabler.clone() * (opcode_id.clone() - expected_opcode_id));

        // Registers update
        eval.add_to_relation(RelationEntry::new(
            &self.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.registers,
            E::EF::from(enabler.clone()),
            &[op0_val.clone() + off1.clone(), fp.clone()],
        ));

        // Read instruction from memory
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
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
            &self.memory,
            E::EF::from(enabler.clone()),
            &[pc, clock.clone(), opcode_id, off0.clone(), off1, off2],
        ));

        // Read op0
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off0.clone(),
                op0_prev_clock.clone(),
                op0_val.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler),
            &[fp + off0, clock.clone(), op0_val],
        ));

        // Range check 20
        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            -E::EF::one(),
            &[clock.clone() - inst_prev_clock - one.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            -E::EF::one(),
            &[clock - op0_prev_clock - one],
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
