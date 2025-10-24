use rayon::iter::ParallelIterator;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::core::fields::m31::{BaseField, M31};
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo::prover::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;

// Trait for components that provide range check data
pub trait RangeCheckProvider {
    /// Returns range_check_8 data if the component has it, otherwise returns an empty iterator
    fn get_range_check_8(&self) -> impl ParallelIterator<Item = &PackedM31> {
        rayon::iter::empty()
    }

    /// Returns range_check_16 data if the component has it, otherwise returns an empty iterator
    fn get_range_check_16(&self) -> impl ParallelIterator<Item = &PackedM31> {
        rayon::iter::empty()
    }

    /// Returns range_check_20 data if the component has it, otherwise returns an empty iterator
    fn get_range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        rayon::iter::empty()
    }
}
// Include the macro implementation
#[macro_use]
pub mod range_check_macro;

pub mod range_check_8 {
    use crate::relations::RangeCheck8;
    crate::define_range_check!(8, range_check_8, RangeCheck8);
}
pub mod range_check_16 {
    use crate::relations::RangeCheck16;
    crate::define_range_check!(16, range_check_16, RangeCheck16);
}
pub mod range_check_20 {
    use crate::relations::RangeCheck20;
    crate::define_range_check!(20, range_check_20, RangeCheck20);
}

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
