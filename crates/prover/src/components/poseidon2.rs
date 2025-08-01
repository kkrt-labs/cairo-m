//! Component for Poseidon2 hash function permutation over M31 field.
//! Implements the complete Poseidon2 permutation with external and internal rounds.
//!
//! # Columns
//!
//! - enabler: Boolean flag indicating active rows
//! - initial_state: T elements of the input state (16 elements)
//! - first_half_full_rounds: 3 * T * (FULL_ROUNDS/2) intermediate values from external rounds
//!   - squared_state_1: First squaring of state after adding round constants
//!   - squared_state_2: Second squaring (completing x^4)
//!   - final_state: Result after multiplying by initial state (x^5) and applying external matrix
//! - partial_rounds: 3 * PARTIAL_ROUNDS intermediate values from internal rounds
//!   - squared_first_1: First squaring of state[0] after adding round constant
//!   - squared_first_2: Second squaring of state[0] (completing x^4)
//!   - sbox_result: Result after multiplying by initial state[0] (x^5)
//! - second_half_full_rounds: 3 * T * (FULL_ROUNDS/2) intermediate values from external rounds
//!   - Same structure as first half full rounds
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * first half full rounds:
//!   - `enabler * (state[i]^2 - squared_state_1)`
//!   - `enabler * (state[i]^2 - squared_state_2)`
//!   - `enabler * (state[i] * initial_state[i] - final_state)`
//! * partial rounds:
//!   - `enabler * (state[0]^2 - squared_state_1)`
//!   - `enabler * (state[0]^2 - squared_state_2)`
//!   - `enabler * (state[0] * initial_state[0] - sbox_result)`
//! * second half full rounds:
//!   - `enabler * (state[i]^2 - squared_state_1)`
//!   - `enabler * (state[i]^2 - squared_state_2)`
//!   - `enabler * (state[i] * initial_state[i] - final_state)`
//! * lookup relations:
//!   - `- enabler * [initial_state]` in `Poseidon2` relation
//!   - `+ enabler * [final_state[0]]` in `Poseidon2` relation

use std::ops::{Add, AddAssign, Mul, Sub};

use num_traits::Zero;
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
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{SecureField, SECURE_EXTENSION_DEGREE};
use stwo_prover::core::fields::FieldExpOps;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::components::Relations;
use crate::poseidon2::{
    EXTERNAL_ROUND_CONSTS, FULL_ROUNDS, INTERNAL_MATRIX, INTERNAL_ROUND_CONSTS, PARTIAL_ROUNDS, T,
};
use crate::utils::enabler::Enabler;

const N_TRACE_COLUMNS: usize = 1 + T * (1 + FULL_ROUNDS * 3) + 3 * PARTIAL_ROUNDS;
const N_POSEIDON2_LOOKUPS: usize = 2;
const N_INTERACTION_COLUMNS: usize = SECURE_EXTENSION_DEGREE * N_POSEIDON2_LOOKUPS.div_ceil(2);

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
    pub poseidon2: [Vec<[PackedM31; T]>; N_POSEIDON2_LOOKUPS],
}

#[inline(always)]
/// Applies the M4 MDS matrix described in <https://eprint.iacr.org/2023/323.pdf> 5.1.
fn apply_m4<F>(x: [F; 4]) -> [F; 4]
where
    F: Clone + AddAssign<F> + Add<F, Output = F> + Sub<F, Output = F> + Mul<M31, Output = F>,
{
    let t0 = x[0].clone() + x[1].clone();
    let t02 = t0.clone() + t0.clone();
    let t1 = x[2].clone() + x[3].clone();
    let t12 = t1.clone() + t1.clone();
    let t2 = x[1].clone() + x[1].clone() + t1;
    let t3 = x[3].clone() + x[3].clone() + t0;
    let t4 = t12.clone() + t12 + t3.clone();
    let t5 = t02.clone() + t02 + t2.clone();
    let t6 = t3 + t5.clone();
    let t7 = t2 + t4.clone();
    [t6, t5, t7, t4]
}

/// Applies the external round matrix.
/// See <https://eprint.iacr.org/2023/323.pdf> 5.1 and Appendix B.
fn apply_external_round_matrix<F>(state: &mut [F; 16])
where
    F: Clone + AddAssign<F> + Add<F, Output = F> + Sub<F, Output = F> + Mul<M31, Output = F>,
{
    // Applies circ(2M4, M4, M4, M4).
    for i in 0..4 {
        [
            state[4 * i],
            state[4 * i + 1],
            state[4 * i + 2],
            state[4 * i + 3],
        ] = apply_m4([
            state[4 * i].clone(),
            state[4 * i + 1].clone(),
            state[4 * i + 2].clone(),
            state[4 * i + 3].clone(),
        ]);
    }
    for j in 0..4 {
        let s =
            state[j].clone() + state[j + 4].clone() + state[j + 8].clone() + state[j + 12].clone();
        for i in 0..4 {
            state[4 * i + j] += s.clone();
        }
    }
}

// Applies the internal round matrix.
// See <https://eprint.iacr.org/2023/323.pdf> 5.2.
fn apply_internal_round_matrix<F>(state: &mut [F; 16])
where
    F: Clone + AddAssign<F> + Add<F, Output = F> + Sub<F, Output = F> + Mul<M31, Output = F>,
{
    let sum = state[1..]
        .iter()
        .cloned()
        .fold(state[0].clone(), |acc, s| acc + s);
    state.iter_mut().enumerate().for_each(|(i, s)| {
        *s = s.clone() * INTERNAL_MATRIX[i] + sum.clone();
    });
}

#[inline(always)]
fn square<F: FieldExpOps>(x: F) -> F {
    x.clone() * x
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
        inputs: &Vec<[M31; T]>,
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let non_padded_length = inputs.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack round data from the prover input
        let packed_inputs: Vec<[PackedM31; T]> = inputs
            .iter()
            .chain(std::iter::repeat(&[M31::zero(); T]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let enabler_col = Enabler::new(non_padded_length);
        let zero = PackedM31::zero();

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
            .for_each(|(row_index, (mut row, mut state, lookup_data))| {
                let mut col_index = 0;
                let enabler = enabler_col.packed_at(row_index);

                *row[col_index] = enabler;
                col_index += 1;

                // Add initial state to the trace and lookup data
                state.iter().for_each(|s| {
                    *row[col_index] = *s;
                    col_index += 1;
                });
                *lookup_data.poseidon2[0] = state;

                // Apply initial linear layer
                apply_external_round_matrix(&mut state);

                // Go through first half of full rounds
                (0..FULL_ROUNDS / 2).for_each(|round| {
                    // Add round constants
                    (0..T).for_each(|i| {
                        state[i] += PackedM31::broadcast(EXTERNAL_ROUND_CONSTS[round][i]);
                    });
                    let initial_state = state;
                    // Square the state and write to trace
                    state = std::array::from_fn(|i| square(state[i]));
                    state.iter().for_each(|s| {
                        *row[col_index] = *s;
                        col_index += 1;
                    });

                    // Again
                    state = std::array::from_fn(|i| square(state[i]));
                    state.iter().for_each(|s| {
                        *row[col_index] = *s;
                        col_index += 1;
                    });

                    // Multiply by the initial state for full s-box computation and apply external round matrix
                    state = std::array::from_fn(|i| state[i] * initial_state[i]);
                    apply_external_round_matrix(&mut state);
                    state.iter().for_each(|s| {
                        *row[col_index] = *s;
                        col_index += 1;
                    });
                });

                // Go through partial rounds
                (0..PARTIAL_ROUNDS).for_each(|round| {
                    // Add round constant (only to first element)
                    state[0] += PackedM31::broadcast(INTERNAL_ROUND_CONSTS[round]);
                    let initial_state = state[0];

                    // Square the first element and write to trace
                    state[0] = square(state[0]);
                    *row[col_index] = state[0];
                    col_index += 1;

                    // Again
                    state[0] = square(state[0]);
                    *row[col_index] = state[0];
                    col_index += 1;

                    // Finalize s-box computation and round
                    state[0] = initial_state * state[0];
                    *row[col_index] = state[0];
                    col_index += 1;
                    apply_internal_round_matrix(&mut state);
                });

                // Go through last half of full rounds
                (0..FULL_ROUNDS / 2).for_each(|round| {
                    // Add round constants
                    (0..T).for_each(|i| {
                        state[i] +=
                            PackedM31::broadcast(EXTERNAL_ROUND_CONSTS[round + FULL_ROUNDS / 2][i]);
                    });
                    let initial_state = state;
                    // Square the state and write to trace
                    state = std::array::from_fn(|i| square(state[i]));
                    state.iter().for_each(|s| {
                        *row[col_index] = *s;
                        col_index += 1;
                    });

                    // Again
                    state = std::array::from_fn(|i| square(state[i]));
                    state.iter().for_each(|s| {
                        *row[col_index] = *s;
                        col_index += 1;
                    });

                    // Multiply by the initial state for full s-box computation and apply external round matrix
                    state = std::array::from_fn(|i| state[i] * initial_state[i]);
                    apply_external_round_matrix(&mut state);
                    state.iter().for_each(|s| {
                        *row[col_index] = *s;
                        col_index += 1;
                    });
                });

                // Add digest to lookup data
                let mut final_state = [zero; T];
                final_state[0] = state[0];
                *lookup_data.poseidon2[1] = final_state;
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
        let log_size = interaction_claim_data.lookup_data.poseidon2[0]
            .len()
            .ilog2()
            + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

        let mut col = interaction_trace.new_col();
        (
            col.par_iter_mut(),
            &interaction_claim_data.lookup_data.poseidon2[0],
            &interaction_claim_data.lookup_data.poseidon2[1],
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(i, (writer, value0, value1))| {
                let num0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                let denom0: PackedQM31 = relations.poseidon2.combine(value0);
                let num1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
                let denom1: PackedQM31 = relations.poseidon2.combine(value1);

                let numerator = num0 * denom1 + num1 * denom0;
                let denom = denom0 * denom1;

                writer.write_frac(numerator, denom);
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
        let mut state: [_; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let initial_state = state.clone();

        // Apply initial linear layer
        apply_external_round_matrix(&mut state);

        // Go through first half of full rounds
        (0..FULL_ROUNDS / 2).for_each(|round| {
            // Add round constants
            (0..T).for_each(|i| {
                state[i] += EXTERNAL_ROUND_CONSTS[round][i];
            });
            let initial_state = state.clone();
            // Square the state and write to trace
            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            // Again
            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            // Multiply by the initial state for full s-box computation and apply external round matrix
            state = std::array::from_fn(|i| state[i].clone() * initial_state[i].clone());
            apply_external_round_matrix(&mut state);
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });
        });

        // Go through partial rounds
        (0..PARTIAL_ROUNDS).for_each(|round| {
            // Add round constant (only to first element)
            state[0] += INTERNAL_ROUND_CONSTS[round];
            let initial_state = state[0].clone();

            // Square the first element and write to trace
            let m = eval.next_trace_mask();
            eval.add_constraint(enabler.clone() * (square(state[0].clone()) - m.clone()));
            state[0] = m;

            // Again
            let m = eval.next_trace_mask();
            eval.add_constraint(enabler.clone() * (square(state[0].clone()) - m.clone()));
            state[0] = m;

            // Finalize s-box computation and round
            let m = eval.next_trace_mask();
            eval.add_constraint(enabler.clone() * (initial_state * state[0].clone() - m.clone()));
            state[0] = m;

            apply_internal_round_matrix(&mut state);
        });

        // Go through last half of full rounds
        (0..FULL_ROUNDS / 2).for_each(|round| {
            // Add round constants
            (0..T).for_each(|i| {
                state[i] += EXTERNAL_ROUND_CONSTS[FULL_ROUNDS / 2 + round][i];
            });
            let initial_state = state.clone();
            // Square the state and write to trace
            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            // Again
            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            // Multiply by the initial state for full s-box computation and apply external round matrix
            state = std::array::from_fn(|i| state[i].clone() * initial_state[i].clone());
            apply_external_round_matrix(&mut state);
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });
        });

        // Use input state
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            -E::EF::from(enabler.clone()),
            &initial_state,
        ));

        // Emit next state (only the first element for the final round)
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            E::EF::from(enabler),
            &[state[0].clone()],
        ));
        eval.finalize_logup_in_pairs();
        eval
    }
}
