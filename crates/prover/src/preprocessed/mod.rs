//! A collection of preprocessed columns, whose values are publicly acknowledged, and independent of
//! the proof.
use stwo_prover::constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::fields::m31::BaseField;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::range_check_20::RangeCheck;

pub mod range_check_20;

pub trait PreProcessedColumn {
    fn log_size(&self) -> u32;
    fn id(&self) -> PreProcessedColumnId;
    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>;
}

pub struct PreProcessedTrace {
    columns: Vec<Box<dyn PreProcessedColumn>>,
}
impl PreProcessedTrace {
    pub fn new(columns: Vec<Box<dyn PreProcessedColumn>>) -> Self {
        Self { columns }
    }

    pub fn log_sizes(&self) -> Vec<u32> {
        self.columns.iter().map(|c| c.log_size()).collect()
    }

    pub fn gen_trace(&self) -> Vec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        self.columns.iter().map(|c| c.gen_column_simd()).collect()
    }

    pub fn ids(&self) -> Vec<PreProcessedColumnId> {
        self.columns.iter().map(|c| c.id()).collect()
    }
}

impl Default for PreProcessedTrace {
    fn default() -> Self {
        let columns = vec![];
        Self::new(columns)
    }
}

#[derive(Default)]
pub struct PreProcessedTraceBuilder {
    columns: Vec<Box<dyn PreProcessedColumn>>,
}

impl PreProcessedTraceBuilder {
    pub fn with_range_check<const N: usize>(mut self, ranges: [u32; N]) -> Self {
        for column_idx in 0..N {
            let range_check = RangeCheck::new(ranges, column_idx);
            self.columns.push(Box::new(range_check));
        }
        self
    }

    pub fn build(self) -> PreProcessedTrace {
        PreProcessedTrace::new(self.columns)
    }
}
