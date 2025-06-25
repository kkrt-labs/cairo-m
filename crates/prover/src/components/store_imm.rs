use cairo_m_common::Opcode;
use num_traits::identities::One;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
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
const N_REGISTERS_LOOKUPS: usize = 2; // - (pc_prev, fp_prev) & + (pc_next, fp_next)
const N_MEMORY_LOOKUPS: usize = 2 * 2; // - (prev_instruction) & + (next_instruction) & - (prev_write) & + (next_write)
const N_RANGE_CHECK_20_LOOKUPS: usize = 2;
const N_INTERACTION_COLUMNS: usize =
    (N_REGISTERS_LOOKUPS + N_MEMORY_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS).div_ceil(2);

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub log_size: u32,
}

pub struct ClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

/// A container to hold the looked up data during main trace generation.
/// It is then used to generate the interaction trace once the challenge has been drawn.
#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    pub range_check_20: [Vec<[PackedM31; 1]>; N_RANGE_CHECK_20_LOOKUPS],
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_log_sizes =
            vec![self.log_size; SECURE_EXTENSION_DEGREE * N_INTERACTION_COLUMNS];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    #[allow(non_snake_case)]
    pub fn write_trace<MC: MerkleChannel>(
        inputs: &mut Vec<StateData>,
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, ClaimData)
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

        // Pack inputs.
        inputs.resize(1 << log_size, StateData::default());
        let packed_inputs: Vec<PackedStateData> = inputs
            .chunks(N_LANES)
            .map(|chunk| {
                let array: [StateData; N_LANES] = chunk.try_into().unwrap();
                Pack::pack(array)
            })
            .collect();

        let zero = PackedM31::broadcast(M31::from(0));
        let one = PackedM31::broadcast(M31::from(1));
        let enabler_col = Enabler::new(non_padded_length);

        trace
            .par_iter_mut()
            .zip(packed_inputs.par_iter())
            .zip(lookup_data.par_iter_mut())
            .enumerate()
            .for_each(|(row_index, ((mut row, input), lookup_data))| {
                *row[0] = enabler_col.packed_at(row_index);
                *row[1] = input.pc;
                *row[2] = input.fp;
                *lookup_data.registers[0] = [input.pc, input.fp];
                *lookup_data.registers[1] = [input.pc + one, input.fp];

                // Memory read: instruction
                let opcode_id = input.mem0_value_0;
                let off0 = input.mem0_value_1;
                let off1 = input.mem0_value_2;
                let off2 = input.mem0_value_3;
                let clock = input.mem0_clock;
                let inst_prev_clock = input.mem0_prev_clock;
                *row[3] = opcode_id;
                *row[4] = off0;
                *row[5] = off1;
                *row[6] = off2;
                *row[7] = clock;
                *row[8] = inst_prev_clock;
                *lookup_data.memory[0] = [input.pc, inst_prev_clock, opcode_id, off0, off1, off2];
                *lookup_data.memory[1] = [input.pc, clock, opcode_id, off0, off1, off2];
                *lookup_data.range_check_20[0] = [clock - inst_prev_clock];

                // Memory write: [fp + off2]
                let dst_prev_val = input.mem1_prev_val_0;
                let dst_prev_clock = input.mem1_prev_clock;
                *row[9] = dst_prev_val;
                *row[10] = dst_prev_clock;
                *lookup_data.memory[2] = [
                    input.fp + off2,
                    dst_prev_clock,
                    dst_prev_val,
                    zero,
                    zero,
                    zero,
                ];
                *lookup_data.memory[3] = [input.fp + off2, clock, off0, zero, zero, zero];
                *lookup_data.range_check_20[1] = [clock - dst_prev_clock];
            });
        (
            Self { log_size },
            trace,
            ClaimData {
                lookup_data,
                non_padded_length,
            },
        )
    }
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        memory_relation: &relations::Memory,
        registers_relation: &relations::Registers,
        range_check_20_relation: &relations::RangeCheck_20,
        claim_data: &ClaimData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = claim_data.lookup_data.memory[0].len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(claim_data.non_padded_length);

        let mut col0 = interaction_trace.new_col();
        for (i, (value0, value1)) in claim_data.lookup_data.registers[0]
            .iter()
            .zip(&claim_data.lookup_data.registers[1])
            .enumerate()
        {
            let denom_0: PackedQM31 = registers_relation.combine(value0);
            let mult_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
            let denom_1: PackedQM31 = registers_relation.combine(value1);
            let mult_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

            col0.write_frac(i, mult_0 * denom_0 + mult_1 * denom_1, denom_0 * denom_1);
        }
        col0.finalize_col();

        let mut col1 = interaction_trace.new_col();
        for (i, (value0, value1)) in claim_data.lookup_data.memory[0]
            .iter()
            .zip(&claim_data.lookup_data.memory[1])
            .enumerate()
        {
            let denom_0: PackedQM31 = memory_relation.combine(value0);
            let mult_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
            let denom_1: PackedQM31 = memory_relation.combine(value1);
            let mult_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

            col1.write_frac(i, mult_0 * denom_0 + mult_1 * denom_1, denom_0 * denom_1);
        }
        col1.finalize_col();

        let mut col2 = interaction_trace.new_col();
        for (i, (value0, value1)) in claim_data.lookup_data.memory[2]
            .iter()
            .zip(&claim_data.lookup_data.memory[3])
            .enumerate()
        {
            let denom_0: PackedQM31 = memory_relation.combine(value0);
            let mult_0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
            let denom_1: PackedQM31 = memory_relation.combine(value1);
            let mult_1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));

            col2.write_frac(i, mult_0 * denom_0 + mult_1 * denom_1, denom_0 * denom_1);
        }
        col2.finalize_col();

        let mut col3 = interaction_trace.new_col();
        for (i, (value0, value1)) in claim_data.lookup_data.range_check_20[0]
            .iter()
            .zip(&claim_data.lookup_data.range_check_20[1])
            .enumerate()
        {
            let denom_0: PackedQM31 = range_check_20_relation.combine(value0);
            let denom_1: PackedQM31 = range_check_20_relation.combine(value1);

            col3.write_frac(
                i,
                PackedQM31::from(enabler_col.packed_at(i)) * (denom_0 + denom_1),
                denom_0 * denom_1,
            );
        }
        col3.finalize_col();

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

    #[allow(non_snake_case)]
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let one = E::F::from(M31::from(1));
        let expect_opcode_id = E::F::from(M31::from(Opcode::StoreImm as u32));

        // 11 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let opcode_id = eval.next_trace_mask();
        let off0 = eval.next_trace_mask();
        let off1 = eval.next_trace_mask();
        let off2 = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let inst_prev_clock = eval.next_trace_mask();
        let dst_prev_val = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one - enabler.clone()));

        // Opcode is StoreImm
        eval.add_constraint(enabler.clone() * (opcode_id.clone() - expect_opcode_id));

        // Update registers
        eval.add_to_relation(RelationEntry::new(
            &self.registers,
            -E::EF::from(enabler.clone()),
            &[pc.clone(), fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.registers,
            E::EF::from(enabler.clone()),
            &[pc.clone() + E::F::one(), fp.clone()],
        ));

        // Check that the opcode is read from the memory
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
            &[
                pc,
                clock.clone(),
                opcode_id,
                off0.clone(),
                off1,
                off2.clone(),
            ],
        ));

        // Check the write at fp + off2
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off2.clone(),
                dst_prev_clock.clone(),
                dst_prev_val,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler.clone()),
            &[fp + off2, clock.clone(), off0],
        ));

        // Check that the write and the read clocks are valid
        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            E::EF::from(enabler.clone()),
            &[clock.clone() - inst_prev_clock],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.range_check_20,
            E::EF::from(enabler),
            &[clock - dst_prev_clock],
        ));

        eval.finalize_logup_in_pairs();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
