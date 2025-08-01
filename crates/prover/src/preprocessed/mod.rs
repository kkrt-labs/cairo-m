//! A collection of preprocessed columns, whose values are publicly acknowledged, and independent of
//! the proof.
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::fields::m31::BaseField;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::range_check::RangeCheck;

pub mod range_check;

pub trait PreProcessedColumn {
    fn log_size(&self) -> u32;
    fn id(&self) -> PreProcessedColumnId;
    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>;
}

#[derive(Default)]
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

pub struct PreProcessedTraceBuilder {
    columns: Vec<Box<dyn PreProcessedColumn>>,
}

impl PreProcessedTraceBuilder {
    pub fn new() -> Self {
        Self { columns: vec![] }
    }

    pub fn with_range_check(mut self, range: u32) -> Self {
        let range_check = RangeCheck::new(range);
        self.columns.push(Box::new(range_check));
        self
    }

    pub fn build(self) -> PreProcessedTrace {
        PreProcessedTrace::new(self.columns)
    }
}

impl Default for PreProcessedTraceBuilder {
    fn default() -> Self {
        Self::new().with_range_check(20)
    }
}
