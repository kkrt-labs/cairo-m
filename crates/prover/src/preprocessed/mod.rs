//! A collection of preprocessed columns, whose values are publicly acknowledged, and independent of
//! the proof.
//!
//! They are similar to regular components but are entirely known by the verifier.
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::fields::m31::BaseField;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::bitwise::Bitwise;
use crate::preprocessed::range_check::RangeCheck;

pub mod bitwise;
pub mod ch_maj;
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

    pub fn with_bitwise(mut self, operand_bits: u32) -> Self {
        // Create the bitwise preprocessed columns
        // This adds 4 columns: operation_id, input1, input2, result
        let bitwise = Bitwise::new(operand_bits);
        for column in bitwise.columns() {
            self.columns.push(Box::new(column));
        }
        self
    }

    pub fn with_sha256(mut self) -> Self {
        // Add ch preprocessed columns for SHA256 - 6 variants (l0, l1, l2, h0, h1, h2)
        // Each variant adds 4 columns: e, f, g, result

        // Add ch_l0 columns
        let ch_l0 = ch_maj::ch_l0::Columns::default();
        for column in ch_l0.columns() {
            self.columns.push(Box::new(column));
        }

        // Add ch_l1 columns
        let ch_l1 = ch_maj::ch_l1::Columns::default();
        for column in ch_l1.columns() {
            self.columns.push(Box::new(column));
        }

        // Add ch_l2 columns
        let ch_l2 = ch_maj::ch_l2::Columns::default();
        for column in ch_l2.columns() {
            self.columns.push(Box::new(column));
        }

        // Add ch_h0 columns
        let ch_h0 = ch_maj::ch_h0::Columns::default();
        for column in ch_h0.columns() {
            self.columns.push(Box::new(column));
        }

        // Add ch_h1 columns
        let ch_h1 = ch_maj::ch_h1::Columns::default();
        for column in ch_h1.columns() {
            self.columns.push(Box::new(column));
        }

        // Add ch_h2 columns
        let ch_h2 = ch_maj::ch_h2::Columns::default();
        for column in ch_h2.columns() {
            self.columns.push(Box::new(column));
        }

        // Add maj preprocessed columns for SHA256 - 6 variants (l0, l1, l2, h0, h1, h2)
        // Each variant adds 4 columns: a, b, c, result

        // Add maj_l0 columns
        let maj_l0 = ch_maj::maj_l0::Columns::default();
        for column in maj_l0.columns() {
            self.columns.push(Box::new(column));
        }

        // Add maj_l1 columns
        let maj_l1 = ch_maj::maj_l1::Columns::default();
        for column in maj_l1.columns() {
            self.columns.push(Box::new(column));
        }

        // Add maj_l2 columns
        let maj_l2 = ch_maj::maj_l2::Columns::default();
        for column in maj_l2.columns() {
            self.columns.push(Box::new(column));
        }

        // Add maj_h0 columns
        let maj_h0 = ch_maj::maj_h0::Columns::default();
        for column in maj_h0.columns() {
            self.columns.push(Box::new(column));
        }

        // Add maj_h1 columns
        let maj_h1 = ch_maj::maj_h1::Columns::default();
        for column in maj_h1.columns() {
            self.columns.push(Box::new(column));
        }

        // Add maj_h2 columns
        let maj_h2 = ch_maj::maj_h2::Columns::default();
        for column in maj_h2.columns() {
            self.columns.push(Box::new(column));
        }

        self
    }

    pub fn build(self) -> PreProcessedTrace {
        PreProcessedTrace::new(self.columns)
    }
}

impl Default for PreProcessedTraceBuilder {
    fn default() -> Self {
        Self::new()
            .with_bitwise(8)
            .with_range_check(8)
            .with_range_check(16)
            .with_range_check(20)
            .with_sha256()
    }
}

impl PreProcessedTraceBuilder {
    /// Creates a preprocessed trace builder configured for SHA256
    pub fn for_sha256() -> Self {
        Self::new().with_range_check(16).with_sha256()
    }
}
