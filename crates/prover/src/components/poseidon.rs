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
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{SECURE_EXTENSION_DEGREE, SecureField};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::components::Relations;
use crate::utils::enabler::Enabler;
use crate::utils::poseidon::poseidon_constants::mds_matrix;
use crate::utils::poseidon::{PoseidonRoundData, T};

const N_TRACE_COLUMNS: usize = T * 5 + 3; // enabler, state, inter_state, inter_state_sq, inter_state_quad, s_box_out_state, full_round, final_round
const INPUT_SIZE: usize = T * 5 + 2; // same but without the enabler
const N_POSEIDON_LOOKUPS: usize = 2;
const N_INTERACTION_COLUMNS: usize = SECURE_EXTENSION_DEGREE * N_POSEIDON_LOOKUPS;

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
    pub poseidon_round: [Vec<[PackedM31; T]>; N_POSEIDON_LOOKUPS],
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

    #[allow(clippy::needless_range_loop)]
    pub fn write_trace<MC: MerkleChannel>(
        inputs: &[PoseidonRoundData],
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let non_padded_length = inputs.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack round data from the prover input
        let packed_inputs: Vec<[PackedM31; INPUT_SIZE]> = inputs
            .iter()
            .map(|round_data| {
                let mut packed = [M31::zero(); INPUT_SIZE];
                packed[0..T].copy_from_slice(&round_data.state);
                packed[T..2 * T].copy_from_slice(&round_data.inter_state);
                packed[2 * T..3 * T].copy_from_slice(&round_data.inter_state_sq);
                packed[3 * T..4 * T].copy_from_slice(&round_data.inter_state_quad);
                packed[4 * T..5 * T].copy_from_slice(&round_data.s_box_out_state);
                packed[5 * T] = round_data.full_round;
                packed[5 * T + 1] = round_data.final_round;
                packed
            })
            .chain(std::iter::repeat([M31::zero(); INPUT_SIZE]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let enabler_col = Enabler::new(non_padded_length);
        let one = PackedM31::one();

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
                let initial_state: [PackedM31; T] = input[0..T].try_into().unwrap();
                let s_box_out_state: [PackedM31; T] = input[4 * T..5 * T].try_into().unwrap();
                let final_round = input[5 * T + 1];

                *row[0] = enabler;

                for i in 0..INPUT_SIZE {
                    *row[1 + i] = input[i];
                }

                // Initial state lookup
                *lookup_data.poseidon_round[0] = initial_state;

                // MDS * s_box_out_state lookup
                // Compute MDS matrix multiplication
                let mds = mds_matrix();
                let mut final_state = [PackedM31::zero(); T];

                for i in 0..T {
                    for j in 0..T {
                        final_state[i] += PackedM31::from(mds[i][j]) * s_box_out_state[j];
                    }
                }

                for element in final_state.iter_mut().skip(1) {
                    *element *= one - final_round;
                }

                *lookup_data.poseidon_round[1] = final_state;
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

impl InteractionClaim {
    pub fn write_interaction_trace(
        relations: &Relations,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        Vec<CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
    ) {
        let log_size = interaction_claim_data.lookup_data.poseidon_round[0]
            .len()
            .ilog2()
            + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.poseidon_round[0],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0))| {
                let num0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom0: PackedQM31 = relations.poseidon_round.combine(value0);

                writer.write_frac(num0, denom0);
            });
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.poseidon_round[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value1))| {
                let num1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.poseidon_round.combine(value1);

                writer.write_frac(num1, denom1);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }
}

pub type Component = FrameworkComponent<Eval>;

#[derive(Clone)]
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

    #[allow(clippy::needless_range_loop)]
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let enabler = eval.next_trace_mask();
        let state: [E::F; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let inter_state: [E::F; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let inter_state_sq: [E::F; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let inter_state_quad: [E::F; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let s_box_out_state: [E::F; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let full_round: E::F = eval.next_trace_mask();
        let final_round: E::F = eval.next_trace_mask();

        let one = E::F::one();

        //enabler is 1 or 0
        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        //full_round is 1 or 0
        eval.add_constraint(full_round.clone() * (one.clone() - full_round.clone()));

        //final_round is 1 or 0
        eval.add_constraint(final_round.clone() * (one.clone() - final_round.clone()));

        // inter_state_0 * inter_state_0 - inter_state_sq_0
        eval.add_constraint(
            inter_state[0].clone() * inter_state[0].clone() - inter_state_sq[0].clone(),
        );

        // inter_state_sq_0 * inter_state_sq_0 - inter_state_quad_0
        eval.add_constraint(
            inter_state_sq[0].clone() * inter_state_sq[0].clone() - inter_state_quad[0].clone(),
        );

        // inter_state_0 * inter_state_quad_0 - s_box_out_state_0
        eval.add_constraint(
            inter_state[0].clone() * inter_state_quad[0].clone() - s_box_out_state[0].clone(),
        );

        // For full rounds and i > 0: inter_state_i * inter_state_i - inter_state_sq_i
        for i in 1..T {
            eval.add_constraint(
                full_round.clone()
                    * (inter_state[i].clone() * inter_state[i].clone() - inter_state_sq[i].clone()),
            );
        }

        // For full rounds and i > 0: inter_state_sq_i * inter_state_sq_i - inter_state_quad_i
        for i in 1..T {
            eval.add_constraint(
                full_round.clone()
                    * (inter_state_sq[i].clone() * inter_state_sq[i].clone()
                        - inter_state_quad[i].clone()),
            );
        }

        // For full rounds and i > 0: inter_state_i * inter_state_quad_i - s_box_out_state_i
        for i in 1..T {
            eval.add_constraint(
                full_round.clone()
                    * (inter_state[i].clone() * inter_state_quad[i].clone()
                        - s_box_out_state[i].clone()),
            );
        }

        // For partial rounds, other elements pass through unchanged
        for i in 1..T {
            eval.add_constraint(
                (one.clone() - full_round.clone())
                    * (inter_state[i].clone() - s_box_out_state[i].clone()),
            );
        }

        let mut final_state: [E::F; T] = core::array::from_fn(|_| E::F::zero());
        let mds = mds_matrix();
        for i in 0..T {
            for j in 0..T {
                final_state[i] += E::F::from(mds[i][j]) * s_box_out_state[j].clone();
            }
        }
        for i in 1..T {
            final_state[i] *= one.clone() - final_round.clone();
        }

        // Use current state
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon_round,
            -E::EF::from(enabler.clone()),
            &state,
        ));
        // Emit next state (only the first element for the final round)
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon_round,
            E::EF::from(enabler),
            &final_state,
        ));
        eval.finalize_logup();
        eval
    }
}
