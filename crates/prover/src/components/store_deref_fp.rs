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

const N_TRACE_COLUMNS: usize = 13;
// -(pc, [opcode_id, off0, off1, off2], prev_clock) || +(pc, [opcode_id, off0, off1, off2], clock)
// -(fp+off2, [prev_value, 0, 0, 0], prev_clock) || +(fp+off2, [value, 0, 0, 0], clock)
// -(fp+off0, [value, 0, 0, 0], prev_clock) || +(fp+off0, [value, 0, 0, 0], clock)
const N_MEMORY_LOOKUPS: usize = 2 * 3;
const N_REGISTERS_LOOKUPS: usize = 2; // -(pc, fp) || +(pc+1, fp)
const N_RANGE_CHECK_20_LOOKUPS: usize = 3; // opcode clock transition, src clock transition, dst clock transition

const LOOKUPS_COLUMNS: usize =
    (N_MEMORY_LOOKUPS + N_REGISTERS_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS).div_ceil(2);

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    // 6 elements: addr, clock, value_0, value_1, value_2, value_3
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    // 2 elements: pc, fp
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
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE * LOOKUPS_COLUMNS];
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

        let one = PackedM31::from(M31::one());
        let zero = PackedM31::from(M31::zero());
        let enabler_col = Enabler::new(non_padded_length);
        trace
            .par_iter_mut()
            .zip(packed_inputs.par_iter())
            .zip(lookup_data.par_iter_mut())
            .enumerate()
            .for_each(|(row_index, ((mut row, input), lookup_data))| {
                let enabler = enabler_col.packed_at(row_index);
                *row[0] = enabler;
                *row[1] = input.pc;
                *row[2] = input.fp;

                let clock = input.mem0_clock;
                *row[3] = clock;

                *lookup_data.registers[0] = [input.pc, input.fp];
                *lookup_data.registers[1] = [input.pc + one, input.fp];

                let opcode_id = input.mem0_value_0;
                let off0 = input.mem0_value_1;
                let off1 = input.mem0_value_2;
                let off2 = input.mem0_value_3;
                let instruction_prev_clock = input.mem0_prev_clock;

                *lookup_data.range_check_20[0] = clock - instruction_prev_clock - enabler;

                *row[4] = instruction_prev_clock;
                *row[5] = opcode_id;
                *row[6] = off0;
                *row[7] = off1;
                *row[8] = off2;

                *lookup_data.memory[0] = [
                    input.pc,
                    instruction_prev_clock,
                    opcode_id,
                    off0,
                    off1,
                    off2,
                ];
                *lookup_data.memory[1] = [input.pc, clock, opcode_id, off0, off1, off2];

                let src_prev_clock = input.mem1_prev_clock;
                let src_value = input.mem1_value_0;
                let dst_prev_clock = input.mem2_prev_clock;
                let dst_prev_value = input.mem2_prev_val_0;

                *lookup_data.range_check_20[1] = clock - src_prev_clock - enabler;
                *lookup_data.range_check_20[2] = clock - dst_prev_clock - enabler;

                *row[9] = src_prev_clock;
                *row[10] = src_value;
                *row[11] = dst_prev_clock;
                *row[12] = dst_prev_value;

                *lookup_data.memory[2] =
                    [input.fp + off0, src_prev_clock, src_value, zero, zero, zero];
                *lookup_data.memory[3] = [input.fp + off0, clock, src_value, zero, zero, zero];

                *lookup_data.memory[4] = [
                    input.fp + off2,
                    dst_prev_clock,
                    dst_prev_value,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[5] = [input.fp + off2, clock, src_value, zero, zero, zero];
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
        memory_relation: &relations::Memory,
        registers_relation: &relations::Registers,
        range_check_20_relation: &relations::RangeCheck_20,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = interaction_claim_data.lookup_data.memory[0].len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

        // Combined column for both register lookups using cleaner zip_eq syntax
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            interaction_claim_data.lookup_data.registers[0].par_iter(),
            interaction_claim_data.lookup_data.registers[1].par_iter(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let denom_0: PackedQM31 = registers_relation.combine(value0);
                let num_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom_1: PackedQM31 = registers_relation.combine(value1);
                let num_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

                writer.write_frac(num_0 * denom_1 + num_1 * denom_0, denom_0 * denom_1);
            });
        col.finalize_col();

        // Combined column for memory[0] + memory[1] (instruction memory read/write)
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            interaction_claim_data.lookup_data.memory[0].par_iter(),
            interaction_claim_data.lookup_data.memory[1].par_iter(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let denom_0: PackedQM31 = memory_relation.combine(value0);
                let num_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom_1: PackedQM31 = memory_relation.combine(value1);
                let num_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

                writer.write_frac(num_0 * denom_1 + num_1 * denom_0, denom_0 * denom_1);
            });
        col.finalize_col();

        // Combined column for memory[2] + memory[3] (deref memory read/write)
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            interaction_claim_data.lookup_data.memory[2].par_iter(),
            interaction_claim_data.lookup_data.memory[3].par_iter(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let denom_0: PackedQM31 = memory_relation.combine(value0);
                let num_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom_1: PackedQM31 = memory_relation.combine(value1);
                let num_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

                writer.write_frac(num_0 * denom_1 + num_1 * denom_0, denom_0 * denom_1);
            });
        col.finalize_col();

        // Combined column for memory[4] + memory[5] (fp+offset memory read/write)
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            interaction_claim_data.lookup_data.memory[4].par_iter(),
            interaction_claim_data.lookup_data.memory[5].par_iter(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let denom_0: PackedQM31 = memory_relation.combine(value0);
                let num_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom_1: PackedQM31 = memory_relation.combine(value1);
                let num_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

                writer.write_frac(num_0 * denom_1 + num_1 * denom_0, denom_0 * denom_1);
            });
        col.finalize_col();

        // Combined column for range_check_20[0] + range_check_20[1] (opcode + src clock transitions)
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            interaction_claim_data.lookup_data.range_check_20[0].par_iter(),
            interaction_claim_data.lookup_data.range_check_20[1].par_iter(),
        )
            .into_par_iter()
            .for_each(|(writer, value0, value1)| {
                let denom_0: PackedQM31 = range_check_20_relation.combine(&[*value0]);
                let num_0: PackedQM31 = -PackedQM31::one();
                let denom_1: PackedQM31 = range_check_20_relation.combine(&[*value1]);
                let num_1: PackedQM31 = -PackedQM31::one();

                writer.write_frac(num_0 * denom_1 + num_1 * denom_0, denom_0 * denom_1);
            });
        col.finalize_col();

        // Single column for range_check_20[2] (dst clock transition)
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            interaction_claim_data.lookup_data.range_check_20[2].par_iter(),
        )
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = range_check_20_relation.combine(&[*value]);
                let num: PackedQM31 = -PackedQM31::one();
                writer.write_frac(num, denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, Self { claimed_sum })
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
        let expected_opcode_id = E::F::from(M31::from(Opcode::StoreDerefFp));

        // 13 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let instruction_prev_clock = eval.next_trace_mask();
        let opcode_id = eval.next_trace_mask();
        let off0 = eval.next_trace_mask();
        let off1 = eval.next_trace_mask();
        let off2 = eval.next_trace_mask();
        let src_prev_clock = eval.next_trace_mask();
        let src_value = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();
        let dst_prev_value = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Opcode id is StoreDerefFp (only check when row is enabled)
        eval.add_constraint(enabler.clone() * (opcode_id.clone() - expected_opcode_id));

        eval.add_to_relation(RelationEntry::new(
            &self.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.registers,
            E::EF::from(enabler.clone()),
            &[pc.clone() + one, fp.clone()],
        ));

        // Check that the opcode is read from the memory
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                instruction_prev_clock.clone(),
                opcode_id.clone(),
                off0.clone(),
                off1.clone(),
                off2.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
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

        // Check the read at deref memory
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off0.clone(),
                src_prev_clock.clone(),
                src_value.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler.clone()),
            &[fp.clone() + off0, clock.clone(), src_value.clone()],
        ));

        // Check the write at fp + off2
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off2.clone(),
                dst_prev_clock.clone(),
                dst_prev_value,
            ],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler.clone()),
            &[fp + off2, clock.clone(), src_value],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            -E::EF::one(),
            &[clock.clone() - instruction_prev_clock - enabler.clone()],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            -E::EF::one(),
            &[clock.clone() - src_prev_clock - enabler.clone()],
        ));

        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            -E::EF::one(),
            &[clock - dst_prev_clock - enabler],
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
