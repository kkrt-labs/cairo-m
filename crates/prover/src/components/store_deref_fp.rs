use cairo_m_common::Opcode;
use num_traits::{One, Zero};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
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
use crate::utils::PackedStateData;

const N_TRACE_COLUMNS: usize = 13;
// -(pc, [opcode_id, off0, off1, off2], prev_clock) || +(pc, [opcode_id, off0, off1, off2], clock)
// -(fp+off2, [prev_value, 0, 0, 0], prev_clock) || +(fp+off2, [value, 0, 0, 0], clock)
// -(fp+off0, [value, 0, 0, 0], prev_clock) || +(fp+off0, [value, 0, 0, 0], clock)
const N_MEMORY_LOOKUPS: usize = 2 * 3;
const N_REGISTERS_LOOKUPS: usize = 2; // -(pc, fp) || +(pc+1, fp)

const LOOKUPS_COLUMNS: usize = N_MEMORY_LOOKUPS + N_REGISTERS_LOOKUPS;

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    // 6 elements: addr, value_0, value_1, value_2, value_3, clock
    pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
    // 2 elements: pc, fp
    pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
}

/// The enabler column is a column of length `padding_offset.next_power_of_two()` where
/// 1. The first `padding_offset` elements are set to 1;
/// 2. The rest are set to 0.
#[derive(Debug, Clone)]
pub struct Enabler {
    pub padding_offset: usize,
}
impl Enabler {
    pub const fn new(padding_offset: usize) -> Self {
        Self { padding_offset }
    }

    pub fn packed_at(&self, vec_row: usize) -> PackedM31 {
        let row_offset = vec_row * N_LANES;
        if self.padding_offset <= row_offset {
            return PackedM31::zero();
        }
        if self.padding_offset >= row_offset + N_LANES {
            return PackedM31::one();
        }

        // The row is partially enabled.
        let mut res = [M31::zero(); N_LANES];
        for v in res.iter_mut().take(self.padding_offset - row_offset) {
            *v = M31::one();
        }
        PackedM31::from_array(res)
    }
}

#[derive(Clone, Default)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; N_TRACE_COLUMNS];
        // TODO: check the correct width of vector for the interaction trace
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
                *row[0] = enabler_col.packed_at(row_index);
                *row[1] = input.pc;
                *row[2] = input.fp;
                *lookup_data.registers[0] = [input.pc, input.fp];
                *lookup_data.registers[1] = [input.pc + one, input.fp];

                let opcode_id = input.mem0_value_0;
                let off0 = input.mem0_value_1;
                let off1 = input.mem0_value_2;
                let off2 = input.mem0_value_3;
                let instruction_prev_clock = input.mem0_prev_clock;
                let clock = input.mem0_clock;

                *row[3] = opcode_id;
                *row[4] = off0;
                *row[5] = off1;
                *row[6] = off2;
                *row[7] = instruction_prev_clock;
                *row[8] = clock;

                *lookup_data.memory[0] = [
                    input.pc,
                    opcode_id,
                    off0,
                    off1,
                    off2,
                    instruction_prev_clock,
                ];
                *lookup_data.memory[1] = [input.pc, opcode_id, off0, off1, off2, clock];

                // We get the new_value from a read, which is written in the memory arguments before writes - hence the mem1_value_0.
                let dst_prev_value = input.mem2_prev_val_0;
                let dst_prev_clock = input.mem2_prev_clock;
                let src_value = input.mem1_value_0;
                let src_prev_clock = input.mem1_prev_clock;

                *row[9] = dst_prev_value;
                *row[10] = dst_prev_clock;
                *row[11] = src_value;
                *row[12] = src_prev_clock;

                *lookup_data.memory[2] =
                    [input.fp + off0, src_value, zero, zero, zero, src_prev_clock];
                *lookup_data.memory[3] = [input.fp + off0, src_value, zero, zero, zero, clock];

                *lookup_data.memory[4] = [
                    input.fp + off2,
                    dst_prev_value,
                    zero,
                    zero,
                    zero,
                    dst_prev_clock,
                ];
                *lookup_data.memory[5] = [input.fp + off2, src_value, zero, zero, zero, clock];
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

#[derive(Clone)]
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
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = interaction_claim_data.lookup_data.memory[0].len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

        // Column for registers[0] lookup
        let mut read_registers_col = interaction_trace.new_col();
        (
            read_registers_col.par_iter_mut(),
            &interaction_claim_data.lookup_data.registers[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = registers_relation.combine(value);
                let mult: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        read_registers_col.finalize_col();

        // Column for registers[1] lookup
        let mut write_new_registers_col = interaction_trace.new_col();
        (
            write_new_registers_col.par_iter_mut(),
            &interaction_claim_data.lookup_data.registers[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = registers_relation.combine(value);
                let mult: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        write_new_registers_col.finalize_col();

        // Column for memory[0] lookup
        let mut read_instruction_memory_col = interaction_trace.new_col();
        (
            read_instruction_memory_col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = memory_relation.combine(value);
                let mult: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        read_instruction_memory_col.finalize_col();

        // Column for memory[1] lookup
        let mut write_instruction_memory_col = interaction_trace.new_col();
        (
            write_instruction_memory_col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = memory_relation.combine(value);
                let mult: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        write_instruction_memory_col.finalize_col();

        // Column for memory[2] lookup
        let mut read_deref_memory_col = interaction_trace.new_col();
        (
            read_deref_memory_col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[2],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = memory_relation.combine(value);
                let mult: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        read_deref_memory_col.finalize_col();

        // Column for memory[3] lookup
        let mut write_deref_memory_col = interaction_trace.new_col();
        (
            write_deref_memory_col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[3],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = memory_relation.combine(value);
                let mult: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        write_deref_memory_col.finalize_col();

        // Column for memory[4] lookup
        let mut read_prev_fp_at_offset = interaction_trace.new_col();
        (
            read_prev_fp_at_offset.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[4],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = memory_relation.combine(value);
                let mult: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        read_prev_fp_at_offset.finalize_col();

        // Column for memory[5] lookup
        let mut write_new_fp_at_offset = interaction_trace.new_col();
        (
            write_new_fp_at_offset.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[5],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let denom: PackedQM31 = memory_relation.combine(value);
                let mult: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                writer.write_frac(mult, denom);
            });
        write_new_fp_at_offset.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, Self { claimed_sum })
    }
}

pub struct Eval {
    pub claim: Claim,
    pub memory: relations::Memory,
    pub registers: relations::Registers,
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
        let zero = E::F::from(M31::zero());
        let expected_opcode_id = E::F::from(M31::from(Opcode::StoreDerefFp));

        // 13 columns
        let enabler = eval.next_trace_mask();
        let pc = eval.next_trace_mask();
        let fp = eval.next_trace_mask();
        let opcode_id = eval.next_trace_mask();
        let off0 = eval.next_trace_mask();
        let off1 = eval.next_trace_mask();
        let off2 = eval.next_trace_mask();
        let instruction_prev_clock = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let dst_prev_value = eval.next_trace_mask();
        let dst_prev_clock = eval.next_trace_mask();
        let src_value = eval.next_trace_mask();
        let src_prev_clock = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one - enabler.clone()));

        // Opcode id is StoreDerefFp
        eval.add_constraint(opcode_id.clone() - expected_opcode_id);

        // Check that the opcode is read from the memory
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                pc.clone(),
                opcode_id.clone(),
                off0.clone(),
                off1.clone(),
                off2.clone(),
                instruction_prev_clock,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler.clone()),
            &[
                pc,
                opcode_id,
                off0.clone(),
                off1,
                off2.clone(),
                clock.clone(),
            ],
        ));

        // Check the read at deref memory
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off0.clone(),
                src_value.clone(),
                zero.clone(),
                zero.clone(),
                zero.clone(),
                src_prev_clock,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler.clone()),
            &[
                fp.clone() + off0,
                src_value,
                zero.clone(),
                zero.clone(),
                zero.clone(),
                clock.clone(),
            ],
        ));
        // Check the write at fp + off2
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(enabler.clone()),
            &[
                fp.clone() + off2.clone(),
                dst_prev_value.clone(),
                zero.clone(),
                zero.clone(),
                zero.clone(),
                dst_prev_clock,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::from(enabler),
            &[
                fp + off2,
                dst_prev_value,
                zero.clone(),
                zero.clone(),
                zero,
                clock,
            ],
        ));

        eval.finalize_logup();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
