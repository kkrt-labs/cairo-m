use std::sync::atomic::{AtomicU32, Ordering};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{LOG_N_LANES, N_LANES, PackedM31};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SECURE_EXTENSION_DEGREE, SecureField};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};

use crate::preprocessed::PreProcessedColumn;
use crate::relations;
use crate::utils::poseidon::poseidon_constants::round_constants;
use crate::utils::poseidon::{FULL_ROUNDS, PARTIAL_ROUNDS, T};
pub struct PoseidonRc {
    t: usize,        // state size
    n_rounds: usize, // number of rounds
}

impl PoseidonRc {
    pub const fn new() -> Self {
        Self {
            t: T,
            n_rounds: FULL_ROUNDS + PARTIAL_ROUNDS,
        }
    }
}

impl PreProcessedColumn for PoseidonRc {
    fn log_size(&self) -> u32 {
        (self.t * self.n_rounds).next_power_of_two().ilog2()
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        let values = BaseColumn::from_iter(round_constants().iter().map(|rc| M31::from(rc.0)));
        CircleEvaluation::new(CanonicCoset::new(self.log_size()).circle_domain(), values)
    }

    fn id(&self) -> PreProcessedColumnId {
        PreProcessedColumnId {
            id: String::from("poseidon_rc"),
        }
    }
}

pub struct InteractionClaimData {
    pub poseidon_rc: Vec<[PackedM31; 2]>,
}

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; 1];
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<'a, MC: MerkleChannel>(
        lookup_data: impl ParallelIterator<Item = &'a PackedM31>,
    ) -> (
        Self,
        [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let log_size = (T * (FULL_ROUNDS + PARTIAL_ROUNDS))
            .next_power_of_two()
            .ilog2();
        let mults_atomic: Vec<AtomicU32> = (0..1 << log_size).map(|_| AtomicU32::new(0)).collect();

        lookup_data.for_each(|entry| {
            for element in entry.to_array() {
                mults_atomic[element.0 as usize].fetch_add(1, Ordering::Relaxed);
            }
        });

        let mults: Vec<M31> = mults_atomic
            .into_par_iter()
            .map(|atomic| M31(atomic.into_inner()))
            .collect();

        let mults_packed: Vec<[PackedM31; 2]> = mults
            .par_chunks(N_LANES)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                [
                    PackedM31::from_array(std::array::from_fn(|i| {
                        round_constants()[chunk_idx * N_LANES + i]
                    })),
                    PackedM31::from_array(chunk.try_into().unwrap()),
                ]
            })
            .collect();

        let domain = CanonicCoset::new(log_size).circle_domain();
        (
            Self { log_size },
            [CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                domain,
                BaseColumn::from_iter(mults),
            )],
            InteractionClaimData {
                poseidon_rc: mults_packed,
            },
        )
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}
impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        poseidon_rc: &relations::PoseidonRc,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    ) {
        let log_size = interaction_claim_data.poseidon_rc.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &interaction_claim_data.poseidon_rc)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = poseidon_rc.combine(&[value[0]]);
                writer.write_frac(value[1].into(), denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
    }
}

#[derive(Clone)]
pub struct Eval {
    pub claim: Claim,
    pub relation: relations::PoseidonRc,
}
impl Eval {
    pub const fn new(claim: Claim, relation: relations::PoseidonRc) -> Self {
        Self { claim, relation }
    }
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let value = eval.get_preprocessed_column(PoseidonRc::new().id());
        let multiplicity = eval.next_trace_mask();

        eval.add_to_relation(RelationEntry::new(
            &self.relation,
            E::EF::from(multiplicity),
            &[value],
        ));

        eval.finalize_logup();
        eval
    }
}
pub type Component = FrameworkComponent<Eval>;
