//! Builds partial Merkle trees from memory for Poseidon2.
//!
//! # Columns
//!
//! - enabler
//! - root
//! - layer_log_size
//! - node_index
//! - quotient = node_index / 2
//! - remainder = node_index % 2
//! - x_0
//! - x_1
//! - parent_hash
//! - final_hash
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * final_hash is a bool
//!   * `final_hash * (1 - final_hash)`
//! * euclidean division is correct
//!   * `node_index - quotient * 2 + remainder`
//! * remainder is a bool
//!   * `remainder * (1 - remainder)`
//! * use left node
//!   * `- [root, layer_log_size, node_index, x_0, x_1, parent_hash]` in `Merkle` relation
//! * use right node
//!   * `- [root, layer_log_size, node_index, x_0, x_1, parent_hash]` in `Merkle` relation
//! * emit parent node
//!   * `+ [root, layer_log_size, node_index * (1 - final_hash / 2), x_0, x_1, parent_hash]` in `Merkle` relation
//! * poseidon2 hash computation
//!   * `+ [x_0, x_1]` in `Poseidon2` relation (emit hash input)
//!   * `- [parent_hash]` in `Poseidon2` relation (use hash output)

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

use crate::components::Relations;
use crate::hints::decommitments::MerkleDecommitmentHints;
use crate::utils::Enabler;

const N_TRACE_COLUMNS: usize = 10;
pub const N_INPUT_COLUMNS: usize = N_TRACE_COLUMNS - 1;
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
    pub merkle: [Vec<[PackedM31; 4]>; N_MERKLE_LOOKUPS],
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
        decommitment_hints: &MerkleDecommitmentHints,
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let non_padded_length = decommitment_hints.rows.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack merkle entries from the prover input
        let packed_inputs: Vec<[PackedM31; N_INPUT_COLUMNS]> = decommitment_hints
            .rows
            .iter()
            .map(|row| row.to_m31_array())
            .chain(std::iter::repeat([M31::zero(); N_INPUT_COLUMNS]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let zero = PackedM31::zero();
        let one = PackedM31::from(M31::one());
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
                let root = input[0];
                let layer_log_size = input[1];
                let node_index = input[2];
                let quotient = input[3];
                let remainder = input[4];
                let x_0 = input[5];
                let x_1 = input[6];
                let parent_hash = input[7];
                let final_hash = input[8];

                *row[0] = enabler;
                *row[1] = root;
                *row[2] = layer_log_size;
                *row[3] = node_index;
                *row[4] = quotient;
                *row[5] = remainder;
                *row[6] = x_0;
                *row[7] = x_1;
                *row[8] = parent_hash;
                *row[9] = final_hash;

                *lookup_data.merkle[0] = [root, layer_log_size, node_index, x_0];
                *lookup_data.merkle[1] = [root, layer_log_size, node_index, x_1];
                *lookup_data.merkle[2] = [
                    root,
                    layer_log_size - one,
                    node_index * (one - final_hash) + quotient * final_hash,
                    parent_hash,
                ];

                *lookup_data.poseidon2[0] = [x_0, x_1];
                *lookup_data.poseidon2[1] = [parent_hash, zero];
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
            &interaction_claim_data.lookup_data.poseidon2[0],
            &interaction_claim_data.lookup_data.poseidon2[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let num0: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom0: PackedQM31 = relations.poseidon2.combine(value0);
                let num1: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.poseidon2.combine(value1);

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
                let num: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom: PackedQM31 = relations.merkle.combine(value);

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
        let m31_2 = E::F::from(M31::from(2));

        let enabler = eval.next_trace_mask();
        let root = eval.next_trace_mask();
        let layer_log_size = eval.next_trace_mask();
        let node_index = eval.next_trace_mask();
        let quotient = eval.next_trace_mask();
        let remainder = eval.next_trace_mask();
        let x_0 = eval.next_trace_mask();
        let x_1 = eval.next_trace_mask();
        let parent_hash = eval.next_trace_mask();
        let final_hash = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Final hash is 0 or 1
        eval.add_constraint(final_hash.clone() * (one.clone() - final_hash.clone()));

        // Check euclidean division
        eval.add_constraint(node_index.clone() - quotient.clone() * m31_2 - remainder.clone());

        // Check that quotient is 0 or 1
        eval.add_constraint(remainder.clone() * (one.clone() - remainder));

        // Use current depth node
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            -E::EF::from(enabler.clone()),
            &[
                root.clone(),
                layer_log_size.clone(),
                node_index.clone(),
                x_0.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            -E::EF::from(enabler.clone()),
            &[
                root.clone(),
                layer_log_size.clone(),
                node_index.clone(),
                x_1.clone(),
            ],
        ));

        // Emit initial state of permutation
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            E::EF::from(enabler.clone()),
            &[x_0, x_1],
        ));
        // Use first element of last state (hash)
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            -E::EF::from(enabler.clone()),
            &[parent_hash.clone()],
        ));

        // Emit next layer
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from(enabler),
            &[
                root,
                layer_log_size - one.clone(),
                node_index * (one - final_hash.clone()) + quotient * final_hash,
                parent_hash,
            ],
        ));

        eval.finalize_logup_in_pairs();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
