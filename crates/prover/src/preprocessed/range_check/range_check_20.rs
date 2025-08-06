use std::sync::atomic::{AtomicU32, Ordering};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SecureField, SECURE_EXTENSION_DEGREE};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::range_check::RangeCheck;
use crate::preprocessed::PreProcessedColumn;
use crate::relations::RangeCheck20;

pub const LOG_SIZE_RC_20: u32 = 20;

pub struct InteractionClaimData {
    pub range_check_20: Vec<[PackedM31; 2]>,
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

    /// Writes the preprocessed range_check_20 trace
    ///
    /// lookup_data contains all range_checks made in other components during main trace generation
    /// write_trace creates a column with all values from 0 to 2**20 - 1 included and counts how many times other components
    /// have range_checked each values: every occurence of a range_checked value increases by 1 its multiplicity.
    /// These multiplicities are stored in a new column.
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
        let mults_atomic: Vec<AtomicU32> = (0..1 << LOG_SIZE_RC_20)
            .map(|_| AtomicU32::new(0))
            .collect();

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
                        M31((chunk_idx * N_LANES + i) as u32)
                    })),
                    PackedM31::from_array(chunk.try_into().unwrap()),
                ]
            })
            .collect();

        let domain = CanonicCoset::new(LOG_SIZE_RC_20).circle_domain();
        (
            Self {
                log_size: LOG_SIZE_RC_20,
            },
            [CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                domain,
                BaseColumn::from_iter(mults),
            )],
            InteractionClaimData {
                range_check_20: mults_packed,
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
        range_check_20: &RangeCheck20,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    ) {
        let log_size = interaction_claim_data.range_check_20.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &interaction_claim_data.range_check_20)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = range_check_20.combine(&[value[0]]);
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
    pub relation: RangeCheck20,
}
impl Eval {
    pub const fn new(claim: Claim, relation: RangeCheck20) -> Self {
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
        let value = eval.get_preprocessed_column(RangeCheck::new(20).id());
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
