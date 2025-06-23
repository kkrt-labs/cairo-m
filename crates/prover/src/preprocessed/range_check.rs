use std::iter::zip;
use std::simd::Simd;

use stwo_prover::constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::fields::m31::{BaseField, MODULUS_BITS};
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;

pub const SIMD_ENUMERATION_0: Simd<u32, N_LANES> =
    Simd::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

pub struct RangeCheck<const N: usize> {
    ranges: [u32; N],
    column_idx: usize,
}

impl<const N: usize> RangeCheck<N> {
    pub fn new(ranges: [u32; N], column_idx: usize) -> Self {
        assert!(ranges.iter().all(|&r| r > 0));
        assert!(column_idx < N);
        Self { ranges, column_idx }
    }

    /// Generates the map from 0..2^(sum_bits) to the corresponding value's partition segments.
    pub fn generate_partitioned_enumeration(&self) -> [Vec<PackedM31>; N] {
        let sum_bits = self.ranges.iter().sum::<u32>();
        assert!(sum_bits < MODULUS_BITS);

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
