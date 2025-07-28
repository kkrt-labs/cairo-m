//! Component for private memory management (emit and use boundary memory values).
//! Emits intermediate nodes and leaves of merkle trees.
//!
//! # Columns
//!
//! - enabler
//! - address
//! - clock
//! - value0
//! - value1
//! - value2
//! - value3
//! - multiplicity
//! - depth
//! - root
//! - intermediate_node_flag
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * intermediate_node_flag is a bool
//!   * `intermediate_node_flag * (1 - intermediate_node_flag)`
//! * emit or use boundary memory values
//!   * `+ or - [address, prev_clock, prev_value]` in `Memory` relation
//! * emit leaves and intermediate nodes of merkle trees
//!   * `[ address * (4 - 3 * intermediate_node_flag), depth, value, root]` in `Merkle` relation
//!   * `(1 - intermediate_node_flag) * enabler * [ 4 * address + 1, depth, value, root]` in `Merkle` relation
//!   * `(1 - intermediate_node_flag) * enabler * [ 4 * address + 2, depth, value, root]` in `Merkle` relation
//!   * `(1 - intermediate_node_flag) * enabler * [ 4 * address + 3, depth, value, root]` in `Merkle` relation

use num_traits::{One, Zero};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::simd::m31::{LOG_N_LANES, N_LANES, PackedM31};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{QM31, SECURE_EXTENSION_DEGREE, SecureField};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::adapter::MerkleTrees;
use crate::adapter::memory::Memory;
use crate::adapter::merkle::TREE_HEIGHT;
use crate::components::Relations;
use crate::utils::enabler::Enabler;

const N_TRACE_COLUMNS: usize = 11;
const N_INPUT_COLUMNS: usize = 10; // same as trace but without the enabler
const N_MEMORY_LOOKUPS: usize = 1;
const N_MERKLE_LOOKUPS: usize = 4;
const N_INTERACTION_COLUMNS: usize =
    SECURE_EXTENSION_DEGREE * (N_MEMORY_LOOKUPS + N_MERKLE_LOOKUPS).div_ceil(2);

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: [Vec<[PackedM31; 7]>; N_MEMORY_LOOKUPS],
    pub merkle: [Vec<[PackedM31; 5]>; N_MERKLE_LOOKUPS],
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_log_sizes = vec![self.log_size; N_INTERACTION_COLUMNS];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<MC: MerkleChannel>(
        inputs: &Memory,
        merkle_trees: &MerkleTrees,
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let initial_memory_len = inputs.initial_memory.len();
        let non_padded_length = initial_memory_len + inputs.final_memory.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack memory entries from the prover input
        let packed_inputs: Vec<[PackedM31; N_INPUT_COLUMNS]> = inputs
            .initial_memory
            .iter()
            .chain(inputs.final_memory.iter())
            .enumerate()
            .map(|(i, ((address, depth), (value, clock, multiplicity)))| {
                let root = if i < initial_memory_len {
                    merkle_trees.initial_root.unwrap()
                } else {
                    merkle_trees.final_root.unwrap()
                };
                let value_array = value.to_m31_array();
                [
                    *address,
                    *clock,
                    value_array[0],
                    value_array[1],
                    value_array[2],
                    value_array[3],
                    *multiplicity,
                    *depth,
                    root,
                    if depth.0 == TREE_HEIGHT {
                        M31::from(0)
                    } else {
                        M31::from(1)
                    },
                ]
            })
            .chain(std::iter::repeat([M31::zero(); N_INPUT_COLUMNS]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let one = PackedM31::from(M31::one());
        let m31_2 = PackedM31::from(M31::from(2));
        let m31_3 = PackedM31::from(M31::from(3));
        let m31_4 = PackedM31::from(M31::from(4));
        let enabler_col = Enabler::new(non_padded_length);

        // Generate lookup data and fill the trace
        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
                LookupData::uninitialized(log_size - LOG_N_LANES),
            )
        };
        (
            trace.par_iter_mut(),
            packed_inputs.into_par_iter(),
            lookup_data.par_iter_mut(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(row_index, (mut row, input, lookup_data))| {
                let enabler = enabler_col.packed_at(row_index);
                let address = input[0];
                let clock = input[1];
                let value0 = input[2];
                let value1 = input[3];
                let value2 = input[4];
                let value3 = input[5];
                let multiplicity = input[6];
                let depth = input[7];
                let root = input[8];
                let intermediate_node_flag = input[9];

                *row[0] = enabler;
                *row[1] = address;
                *row[2] = clock;
                *row[3] = value0;
                *row[4] = value1;
                *row[5] = value2;
                *row[6] = value3;
                *row[7] = multiplicity;
                *row[8] = depth;
                *row[9] = root;
                *row[10] = intermediate_node_flag;

                *lookup_data.memory[0] =
                    [address, clock, value0, value1, value2, value3, multiplicity];
                *lookup_data.merkle[0] = [address, depth, value0, root, intermediate_node_flag];
                *lookup_data.merkle[1] = [
                    address * m31_4 + one,
                    depth,
                    value1,
                    root,
                    intermediate_node_flag,
                ];
                *lookup_data.merkle[2] = [
                    address * m31_4 + m31_2,
                    depth,
                    value2,
                    root,
                    intermediate_node_flag,
                ];
                *lookup_data.merkle[3] = [
                    address * m31_4 + m31_3,
                    depth,
                    value3,
                    root,
                    intermediate_node_flag,
                ];
            });

        // Return the trace and lookup data
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
        let one = PackedQM31::one();
        let qm31_3 = PackedQM31::from(QM31::from(3));
        let qm31_4 = PackedQM31::from(QM31::from(4));
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[0],
            &interaction_claim_data.lookup_data.merkle[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let num0: PackedQM31 = PackedQM31::from(value0[6]);
                let denom0: PackedQM31 = relations.memory.combine(&value0[..6]);

                let intermediate_node_flag1 = PackedQM31::from(value1[4]);
                let num1: PackedQM31 =
                    (one - intermediate_node_flag1) * PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.merkle.combine(&value1[..4]);

                let numerator = num0 * denom1 + num1 * denom0;
                let denom = denom0 * denom1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.merkle[2],
            &interaction_claim_data.lookup_data.merkle[3],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let intermediate_node_flag0 = PackedQM31::from(value0[4]);
                let num0: PackedQM31 =
                    (one - intermediate_node_flag0) * PackedQM31::from(enabler_col.packed_at(i));
                let denom0: PackedQM31 = relations.merkle.combine(&value0[..4]);

                let intermediate_node_flag1 = PackedQM31::from(value1[4]);
                let num1: PackedQM31 =
                    (one - intermediate_node_flag1) * PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.merkle.combine(&value1[..4]);

                let numerator = num0 * denom1 + num1 * denom0;
                let denom = denom0 * denom1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        // It's important for this column to contain only the first leaf as single lookup
        // for matters of constraint degree
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.merkle[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let intermediate_node_flag = PackedQM31::from(value[4]);
                let address = PackedQM31::from(value[0]);
                let index = address * (qm31_4 - qm31_3 * intermediate_node_flag);
                let depth = PackedQM31::from(value[1]);
                let value0 = PackedQM31::from(value[2]);
                let root = PackedQM31::from(value[3]);

                let numerator = PackedQM31::from(enabler_col.packed_at(i));
                let denom: PackedQM31 = relations.merkle.combine(&[index, depth, value0, root]);

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
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
        let one = E::F::one();
        let m31_2 = E::F::from(M31::from(2));
        let m31_3 = E::F::from(M31::from(3));
        let m31_4 = E::F::from(M31::from(4));
        let enabler = eval.next_trace_mask();
        let address = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let value0 = eval.next_trace_mask();
        let value1 = eval.next_trace_mask();
        let value2 = eval.next_trace_mask();
        let value3 = eval.next_trace_mask();
        let multiplicity = eval.next_trace_mask();
        let depth = eval.next_trace_mask();
        let root = eval.next_trace_mask();
        let intermediate_node_flag = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Intermediate node flag is 1 or 0
        eval.add_constraint(
            intermediate_node_flag.clone() * (one.clone() - intermediate_node_flag.clone()),
        );

        // Emit initial values and use final ones
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(multiplicity),
            &[
                address.clone(),
                clock,
                value0.clone(),
                value1.clone(),
                value2.clone(),
                value3.clone(),
            ],
        ));

        // Emit leaves and intermediate nodes
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from((one.clone() - intermediate_node_flag.clone()) * enabler.clone()),
            &[
                address.clone() * m31_4.clone() + one.clone(),
                depth.clone(),
                value1,
                root.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from((one.clone() - intermediate_node_flag.clone()) * enabler.clone()),
            &[
                address.clone() * m31_4.clone() + m31_2,
                depth.clone(),
                value2,
                root.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from((one - intermediate_node_flag.clone()) * enabler.clone()),
            &[
                address.clone() * m31_4.clone() + m31_3.clone(),
                depth.clone(),
                value3,
                root.clone(),
            ],
        ));
        // First leaf must be added in last to not be batched with another lookup
        // in order to keep the constraint degree equal to 2
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from(enabler),
            &[
                address * (m31_4 - m31_3 * intermediate_node_flag),
                depth,
                value0,
                root,
            ],
        ));
        eval.finalize_logup_in_pairs();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
