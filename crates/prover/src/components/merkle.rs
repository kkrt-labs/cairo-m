//! Builds partial Merkle trees from memory for Poseidon2.
//!
//! # Columns
//!
//! - enabler
//! - index
//! - depth
//! - left_value
//! - right_value
//! - parent_value
//! - left_multiplicity
//! - right_multiplicity
//! - parent_multiplicity
//! - root
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * use left node
//!   * `- [index, depth, left_value, root]` in `Memory` relation
//! * use right node
//!   * `- [index + 1, depth, right_value, root]` in `Memory` relation
//! * emit parent node
//!   * `+ [index / 2, depth - 1, parent_value, root]` in `Memory` relation
//! * poseidon2 hash computation
//!   * `+ [left_value, right_value]` in `Poseidon2` relation (emit hash input)
//!   * `- [parent_value]` in `Poseidon2` relation (use hash output)

use num_traits::{One, Zero};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
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

use crate::adapter::MerkleTrees;
use crate::components::Relations;
use crate::utils::enabler::Enabler;

const N_TRACE_COLUMNS: usize = 10;
const N_MERKLE_LOOKUPS: usize = 3;
const N_POSEIDON2_LOOKUPS: usize = 2;
const N_INTERACTION_COLUMNS: usize =
    SECURE_EXTENSION_DEGREE * (N_MERKLE_LOOKUPS + N_POSEIDON2_LOOKUPS).div_ceil(2);

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
    pub merkle: [Vec<[PackedM31; 5]>; N_MERKLE_LOOKUPS],
    pub poseidon2: [Vec<[PackedM31; 2]>; N_POSEIDON2_LOOKUPS],
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
        merkle_trees: &MerkleTrees,
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let initial_tree_len = merkle_trees.initial_tree.len();
        let non_padded_length = initial_tree_len + merkle_trees.final_tree.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack merkle entries from the prover input
        let packed_inputs: Vec<[PackedM31; N_TRACE_COLUMNS - 1]> = merkle_trees
            .initial_tree
            .iter()
            .chain(merkle_trees.final_tree.iter())
            .enumerate()
            .map(|(i, node_data)| {
                let root = if i < initial_tree_len {
                    merkle_trees.initial_root.unwrap()
                } else {
                    merkle_trees.final_root.unwrap()
                };
                let node_data_array = node_data.to_m31_array();
                [
                    node_data_array[0],
                    node_data_array[1],
                    node_data_array[2],
                    node_data_array[3],
                    node_data_array[4],
                    node_data_array[5],
                    node_data_array[6],
                    node_data_array[7],
                    root,
                ]
            })
            .chain(std::iter::repeat([M31::zero(); N_TRACE_COLUMNS - 1]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let zero = PackedM31::zero();
        let one = PackedM31::from(M31::one());
        let m31_2 = M31::from(2);
        let m31_2_inv = PackedM31::from(M31::inverse(&m31_2));
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
                let index = input[0];
                let depth = input[1];
                let left_value = input[2];
                let right_value = input[3];
                let parent_value = input[4];
                let left_multiplicity = input[5];
                let right_multiplicity = input[6];
                let parent_multiplicity = input[7];
                let root = input[8];

                *row[0] = enabler;
                *row[1] = index;
                *row[2] = depth;
                *row[3] = left_value;
                *row[4] = right_value;
                *row[5] = parent_value;
                *row[6] = left_multiplicity;
                *row[7] = right_multiplicity;
                *row[8] = parent_multiplicity;
                *row[9] = root;

                *lookup_data.merkle[0] = [index, depth, left_value, root, left_multiplicity];
                *lookup_data.merkle[1] =
                    [index + one, depth, right_value, root, right_multiplicity];
                *lookup_data.merkle[2] = [
                    index * m31_2_inv,
                    depth - one,
                    parent_value,
                    root,
                    parent_multiplicity,
                ];

                *lookup_data.poseidon2[0] = [left_value, right_value];
                *lookup_data.poseidon2[1] = [parent_value, zero];
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
        let log_size = interaction_claim_data.lookup_data.merkle[0].len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.merkle[0],
            &interaction_claim_data.lookup_data.merkle[1],
        )
            .into_par_iter()
            .for_each(|(writer, value0, value1)| {
                let num0: PackedQM31 = PackedQM31::from(value0[4]);
                let denom0: PackedQM31 = relations.merkle.combine(&value0[..4]);
                let num1: PackedQM31 = PackedQM31::from(value1[4]);
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
            &interaction_claim_data.lookup_data.poseidon2[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let num0: PackedQM31 = -PackedQM31::from(value0[4]);
                let denom0: PackedQM31 = relations.merkle.combine(&value0[..4]);
                let num1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.poseidon2.combine(value1);

                let numerator = num0 * denom1 + num1 * denom0;
                let denom = denom0 * denom1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.poseidon2[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let num: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom: PackedQM31 = relations.poseidon2.combine(value);

                writer.write_frac(num, denom);
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
        let m31_2 = M31::from(2);
        let m31_2_inv = E::F::from(M31::inverse(&m31_2));

        let enabler = eval.next_trace_mask();
        let index = eval.next_trace_mask();
        let depth = eval.next_trace_mask();
        let left_value = eval.next_trace_mask();
        let right_value = eval.next_trace_mask();
        let parent_value = eval.next_trace_mask();
        let left_multiplicity = eval.next_trace_mask();
        let right_multiplicity = eval.next_trace_mask();
        let parent_multiplicity = eval.next_trace_mask();
        let root = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // left multiplicity is 0, 1 or 2
        eval.add_constraint(
            left_multiplicity.clone()
                * (left_multiplicity.clone() - one.clone())
                * (left_multiplicity.clone() - one.clone() * E::F::from(m31_2)),
        );
        // right multiplicity is 0, 1 or 2
        eval.add_constraint(
            right_multiplicity.clone()
                * (right_multiplicity.clone() - one.clone())
                * (right_multiplicity.clone() - one.clone() * E::F::from(m31_2)),
        );
        // parent multiplicity is 0, 1 or 2
        eval.add_constraint(
            parent_multiplicity.clone()
                * (parent_multiplicity.clone() - one.clone())
                * (parent_multiplicity.clone() - one.clone() * E::F::from(m31_2)),
        );

        // Emit current depth node
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from(left_multiplicity),
            &[
                index.clone(),
                depth.clone(),
                left_value.clone(),
                root.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from(right_multiplicity),
            &[
                index.clone() + one.clone(),
                depth.clone(),
                right_value.clone(),
                root.clone(),
            ],
        ));

        // Use next layer
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            -E::EF::from(parent_multiplicity),
            &[index * m31_2_inv, depth - one, parent_value.clone(), root],
        ));

        // Emit initial state of permutation
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            E::EF::from(enabler.clone()),
            &[left_value, right_value],
        ));
        // Use first element of last state (hash)
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            -E::EF::from(enabler),
            &[parent_value],
        ));

        eval.finalize_logup_in_pairs();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
