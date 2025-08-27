use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;

pub mod range_check_16;
pub mod range_check_20;
pub mod range_check_8;

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
