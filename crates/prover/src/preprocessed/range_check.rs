use std::collections::HashMap;
use std::iter::zip;
use std::simd::Simd;

use num_traits::One;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
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
use stwo_prover::core::fields::m31::{BaseField, M31, MODULUS_BITS};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::ExtensionOf;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;
use crate::relations::RangeCheck_20;

const N_TRACE_COLUMNS: usize = 1;
const LOG_RANGE: u32 = 20;

const SECURE_EXTENSION_DEGREE: usize = <SecureField as ExtensionOf<BaseField>>::EXTENSION_DEGREE;

pub const SIMD_ENUMERATION_0: Simd<u32, N_LANES> =
    Simd::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

pub struct RangeCheck<const N: usize> {
    ranges: [u32; N],
    column_idx: usize,
}

impl<const N: usize> RangeCheck<N> {
    pub fn new(ranges: [u32; N], column_idx: usize) -> Self {
        debug_assert!(ranges.iter().all(|&r| r > 0));
        debug_assert!(column_idx < N);
        Self { ranges, column_idx }
    }

    /// Generates the map from 0..2^(sum_bits) to the corresponding value's partition segments.
    pub fn generate_partitioned_enumeration(&self) -> [Vec<PackedM31>; N] {
        let sum_bits = self.ranges.iter().sum::<u32>();
        debug_assert!(sum_bits < MODULUS_BITS);

        let mut res = std::array::from_fn(|_| vec![]);
        for vec_row in 0..1 << (sum_bits - LOG_N_LANES) {
            let value = SIMD_ENUMERATION_0 + Simd::splat(vec_row * N_LANES as u32);
            let segments = self.partition_into_bit_segments(value);
            for i in 0..N {
                res[i].push(unsafe { PackedM31::from_simd_unchecked(segments[i]) });
            }
        }
        res
    }

    /// Partitions a number into 'N' bit segments.
    ///
    /// For example: partition_into_bit_segments(0b110101010, [3, 4, 2]) -> [0b110, 0b1010, 0b10]
    ///
    ///
    /// # Arguments
    pub fn partition_into_bit_segments(
        &self,
        mut value: Simd<u32, N_LANES>,
    ) -> [Simd<u32, N_LANES>; N] {
        let mut segments = [Simd::splat(0); N];
        for (segment, segment_n_bits) in zip(&mut segments, self.ranges).rev() {
            let mask = Simd::splat((1 << segment_n_bits) - 1);
            *segment = value & mask;
            value >>= segment_n_bits;
        }
        segments
    }
}

impl<const N: usize> PreProcessedColumn for RangeCheck<N> {
    fn log_size(&self) -> u32 {
        self.ranges.iter().sum()
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        let partitions = self.generate_partitioned_enumeration();
        let column = partitions
            .into_iter()
            .nth(self.column_idx)
            .expect("column_idx >= N; which violates the invariant that column_idx < N");
        CircleEvaluation::new(
            CanonicCoset::new(self.log_size()).circle_domain(),
            BaseColumn::from_simd(column),
        )
    }

    fn id(&self) -> PreProcessedColumnId {
        let ranges = self
            .ranges
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join("_");
        PreProcessedColumnId {
            id: format!("range_check_{}_column_{}", ranges, self.column_idx),
        }
    }
}

pub struct LookupData {
    pub range_check_data: Vec<[PackedM31; N_TRACE_COLUMNS + 1]>,
}

#[derive(Copy, Clone)]
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
        let trace = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn write_trace<MC: MerkleChannel>(
        &mut self,
        lookup_data: &Vec<PackedM31>,
    ) -> (
        [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let mut counts: HashMap<u32, u32> = HashMap::new();
        for entry in lookup_data {
            for element in entry.to_array() {
                *counts.entry(element.0).or_insert(0) += 1;
            }
        }

        let unique_values = counts.into_iter().collect::<Vec<_>>();
        let mut mults = Vec::new();
        let mut range_check_data = Vec::new();
        for chunk in unique_values.chunks(N_LANES) {
            let mut mult_lane = [M31(0); N_LANES];
            let mut range_check_data_lane = [M31(0); N_LANES];
            for (i, &(value, mult)) in chunk.iter().enumerate() {
                mult_lane[i] = M31(mult);
                range_check_data_lane[i] = M31(value);
            }
            mults.push(PackedM31::from_array(mult_lane));
            range_check_data.push(PackedM31::from_array(range_check_data_lane));
        }

        self.log_size = mults.len().ilog2() + LOG_N_LANES;
        let domain = CanonicCoset::new(self.log_size).circle_domain();
        (
            [CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                domain,
                BaseColumn::from_simd(mults.clone()),
            )],
            LookupData {
                range_check_data: range_check_data
                    .into_iter()
                    .zip(mults)
                    .map(Into::into)
                    .collect(),
            },
        )
    }
}

#[derive(Clone)]
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
        let log_size = lookup_data.range_check_data.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.range_check_data)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = relation.combine(&[value[0]]);
                writer.write_frac(-PackedQM31::one() * value[1], denom);
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
        let value = eval.get_preprocessed_column(RangeCheck::new([LOG_RANGE], 0).id());
        let multiplicity = eval.next_trace_mask();

        eval.add_to_relation(RelationEntry::new(
            &self.relation,
            E::EF::from(-multiplicity),
            &[value],
        ));

        eval.finalize_logup();
        eval
    }
}
pub type Component = FrameworkComponent<Eval>;
