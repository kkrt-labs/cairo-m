use std::sync::atomic::{AtomicU32, Ordering};

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use rayon::slice::ParallelSlice;
use serde::{Deserialize, Serialize};
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
use stwo_prover::constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::ExtensionOf;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;
use crate::relations::RangeCheck_20;

const LOG_SIZE_RC_20: u32 = 20;

const SECURE_EXTENSION_DEGREE: usize = <SecureField as ExtensionOf<BaseField>>::EXTENSION_DEGREE;

pub struct RangeCheck {
    range: u32,
}

impl RangeCheck {
    pub fn new(range: u32) -> Self {
        debug_assert!(range > 0);
        Self { range }
    }
}

impl PreProcessedColumn for RangeCheck {
    fn log_size(&self) -> u32 {
        self.range
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        CircleEvaluation::new(
            CanonicCoset::new(self.log_size()).circle_domain(),
            BaseColumn::from_iter((0..1 << self.range).map(M31)),
        )
    }

    fn id(&self) -> PreProcessedColumnId {
        PreProcessedColumnId {
            id: format!("range_check_{}", self.range),
        }
    }
}

pub struct LookupData {
    pub range_check_20: Vec<[PackedM31; 2]>,
}

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn new(log_size: u32) -> Self {
        debug_assert!(
            log_size >= LOG_N_LANES,
            "log_size must be at least LOG_N_LANES"
        );
        Self { log_size }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; 1];
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<MC: MerkleChannel>(
        lookup_data: &Vec<PackedM31>,
    ) -> (
        Self,
        [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let mults_atomic: Vec<AtomicU32> = (0..1 << LOG_SIZE_RC_20)
            .map(|_| AtomicU32::new(0))
            .collect();

        lookup_data.par_iter().for_each(|entry| {
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
            LookupData {
                range_check_20: mults_packed,
            },
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}
impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        relation: &RangeCheck_20,
        lookup_data: &LookupData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = lookup_data.range_check_20.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.range_check_20)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = relation.combine(&[value[0]]);
                writer.write_frac(value[1].into(), denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, Self { claimed_sum })
    }
}

#[derive(Clone)]
pub struct Eval {
    pub claim: Claim,
    pub relation: RangeCheck_20,
}
impl Eval {
    pub const fn new(claim: Claim, relation: RangeCheck_20) -> Self {
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
