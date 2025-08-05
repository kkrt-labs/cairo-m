//! Component for private memory management (emit and use boundary memory values).
//! Emits intermediate nodes and leaves of merkle trees.
//!
//! # Columns
//!
//! - enabler
//! - address
//! - clock
//! - value0
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
//!   * `+ or - [address, clock, value]` in `Memory` relation
//! * emit leaves and intermediate nodes of merkle trees
//!   * `[address * intermediate_node_flag, depth, value0, root]` in `Merkle` relation
//!   * `(1 - intermediate_node_flag) * enabler * [4 * address + 1, depth, value1, root]` in `Merkle` relation
//!   * `(1 - intermediate_node_flag) * enabler * [4 * address + 2, depth, value2, root]` in `Merkle` relation
//!   * `(1 - intermediate_node_flag) * enabler * [4 * address + 3, depth, value3, root]` in `Merkle` relation

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

use crate::adapter::memory::Memory;
use crate::adapter::merkle::TREE_HEIGHT;
use crate::adapter::MerkleTrees;
use crate::components::Relations;
use crate::utils::enabler::Enabler;

const N_TRACE_COLUMNS: usize = 8;
const N_INPUT_COLUMNS: usize = 7; // same without the enabler
const N_MEMORY_LOOKUPS: usize = 1;
const N_MERKLE_LOOKUPS: usize = 1;
const N_INTERACTION_COLUMNS: usize =
    SECURE_EXTENSION_DEGREE * (N_MEMORY_LOOKUPS + N_MERKLE_LOOKUPS);

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
    pub memory: [Vec<[PackedM31; 4]>; N_MEMORY_LOOKUPS],
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
                [
                    *address,
                    *clock,
                    *value,
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
                let multiplicity = input[3];
                let depth = input[4];
                let root = input[5];
                let intermediate_node_flag = input[6];

                *row[0] = enabler;
                *row[1] = address;
                *row[2] = clock;
                *row[3] = value0;
                *row[4] = multiplicity;
                *row[5] = depth;
                *row[6] = root;
                *row[7] = intermediate_node_flag;

                *lookup_data.memory[0] = [address, clock, value0, multiplicity];
                *lookup_data.merkle[0] = [address, depth, value0, root, intermediate_node_flag];
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
        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.memory[0],
        )
            .into_par_iter()
            .for_each(|(writer, value)| {
                let numerator = PackedQM31::from(value[3]);
                let denom = relations.memory.combine(&value[..3]);

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
                let depth = PackedQM31::from(value[1]);
                let value0 = PackedQM31::from(value[2]);
                let root = PackedQM31::from(value[3]);

                let numerator = PackedQM31::from(enabler_col.packed_at(i));
                let denom: PackedQM31 = relations.merkle.combine(&[
                    address * intermediate_node_flag,
                    depth,
                    value0,
                    root,
                ]);

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
        let enabler = eval.next_trace_mask();
        let address = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let value0 = eval.next_trace_mask();
        let multiplicity = eval.next_trace_mask();
        let depth = eval.next_trace_mask();
        let root = eval.next_trace_mask();
        let intermediate_node_flag = eval.next_trace_mask();

        // Enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        // Intermediate node flag is 1 or 0
        eval.add_constraint(
            intermediate_node_flag.clone() * (one - intermediate_node_flag.clone()),
        );

        // Emit initial values and use final ones
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(multiplicity),
            &[address.clone(), clock, value0.clone()],
        ));

        // Emit memory leaf or intermediate nodes of merkle trees
        eval.add_to_relation(RelationEntry::new(
            &self.relations.merkle,
            E::EF::from(enabler),
            &[address * intermediate_node_flag, depth, value0, root],
        ));
        eval.finalize_logup();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
