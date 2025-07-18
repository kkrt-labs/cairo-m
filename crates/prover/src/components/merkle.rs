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
use stwo_prover::core::fields::qm31::{SECURE_EXTENSION_DEGREE, SecureField};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::adapter::MerkleTrees;
use crate::adapter::merkle::MerkleHasher;
use crate::components::Relations;
use crate::components::chips::hash::Hash;
use crate::utils::Enabler;

const N_TRACE_COLUMNS: usize = 6; // enabler, index, depth, value_left, value_right, root
const N_MERKLE_LOOKUPS: usize = 3;
const N_INTERACTION_COLUMNS: usize = SECURE_EXTENSION_DEGREE * N_MERKLE_LOOKUPS.div_ceil(2);

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
    pub merkle: [Vec<[PackedM31; 4]>; N_MERKLE_LOOKUPS],
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

    pub fn write_trace<MC: MerkleChannel, H: MerkleHasher>(
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
                let root = input[4];

                *row[0] = enabler;
                *row[1] = index;
                *row[2] = depth;
                *row[3] = left_value;
                *row[4] = right_value;
                *row[5] = root;

                *lookup_data.merkle[0] = [index, depth, left_value, root];
                *lookup_data.merkle[1] = [index + one, depth, right_value, root];
                // Extract M31 values from PackedM31 for hashing
                let hash_values: Vec<M31> = (0..N_LANES)
                    .map(|i| {
                        let left = left_value.to_array()[i];
                        let right = right_value.to_array()[i];
                        H::hash(left, right)
                    })
                    .collect();
                let hashed = PackedM31::from_array(hash_values.try_into().unwrap());

                *lookup_data.merkle[2] = [index * m31_2_inv, depth - one, hashed, root];
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
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let num0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom0: PackedQM31 = relations.merkle.combine(value0);
                let num1: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.merkle.combine(value1);

                let numerator = num0 * denom1 + num1 * denom0;
                let denom = denom0 * denom1;

                writer.write_frac(numerator, denom);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.merkle[2],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value))| {
                let numerator: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom: PackedQM31 = relations.merkle.combine(value);

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
        let m31_2 = M31::from(2);
        let m31_2_inv = E::F::from(M31::inverse(&m31_2));

        let enabler = eval.next_trace_mask();
        let index = eval.next_trace_mask();
        let depth = eval.next_trace_mask();
        let left_value = eval.next_trace_mask();
        let right_value = eval.next_trace_mask();
        let root = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Use current depth node
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            -E::EF::from(enabler.clone()),
            &[
                index.clone(),
                depth.clone(),
                left_value.clone(),
                root.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            -E::EF::from(enabler.clone()),
            &[
                index.clone() + one.clone(),
                depth.clone(),
                right_value.clone(),
                root.clone(),
            ],
        ));

        // Compute hash
        let parent_hash = Hash::evaluate([left_value, right_value], &mut eval);

        // Emit next layer
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from(enabler),
            &[index * m31_2_inv, depth - one, parent_hash, root],
        ));
        eval.finalize_logup_in_pairs();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
