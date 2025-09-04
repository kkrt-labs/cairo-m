use rayon::iter::ParallelIterator;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;

// Constants for bitwise operations
/// Number of bits in each operand for bitwise lookups
pub const BITWISE_OPERAND_BITS: u32 = 8;
/// Total bits for the lookup table (operand1 bits + operand2 bits)
pub const BITWISE_LOOKUP_BITS: u32 = BITWISE_OPERAND_BITS * 2;
/// Mask for extracting the lower operand
pub const BITWISE_OPERAND_MASK: u32 = (1 << BITWISE_OPERAND_BITS) - 1;

// Trait for components that provide bitwise data
pub trait BitwiseProvider {
    /// Returns bitwise AND data if the component has it, otherwise returns an empty iterator
    fn get_bitwise_and(&self) -> impl ParallelIterator<Item = &[[PackedM31; 2]]> {
        rayon::iter::empty()
    }

    /// Returns bitwise OR data if the component has it, otherwise returns an empty iterator
    fn get_bitwise_or(&self) -> impl ParallelIterator<Item = &[[PackedM31; 2]]> {
        rayon::iter::empty()
    }

    /// Returns bitwise XOR data if the component has it, otherwise returns an empty iterator
    fn get_bitwise_xor(&self) -> impl ParallelIterator<Item = &[[PackedM31; 2]]> {
        rayon::iter::empty()
    }
}

// Include the macro implementation
#[macro_use]
pub mod bitwise_macro;

// Define bitwise operations modules using the macro
pub mod bitwise_and {
    use crate::relations::BitwiseAnd;
    crate::define_bitwise!(and, bitwise_and, BitwiseAnd, |a: u32, b: u32| a & b);
}

pub mod bitwise_or {
    use crate::relations::BitwiseOr;
    crate::define_bitwise!(or, bitwise_or, BitwiseOr, |a: u32, b: u32| a | b);
}

pub mod bitwise_xor {
    use crate::relations::BitwiseXor;
    crate::define_bitwise!(xor, bitwise_xor, BitwiseXor, |a: u32, b: u32| a ^ b);
}

/// Enum to represent different bitwise operations
#[derive(Clone, Copy)]
pub enum BitwiseOperation {
    And,
    Or,
    Xor,
}

impl BitwiseOperation {
    const fn apply(&self, a: u32, b: u32) -> u32 {
        match self {
            Self::And => a & b,
            Self::Or => a | b,
            Self::Xor => a ^ b,
        }
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
        }
    }
}

pub struct Bitwise {
    operation: BitwiseOperation,
    col_index: usize,
}

impl Bitwise {
    pub const fn new(operation: BitwiseOperation, col_index: usize) -> Self {
        assert!(col_index < 3, "col_index must be in range 0..=2");
        Self {
            operation,
            col_index,
        }
    }
}

impl PreProcessedColumn for Bitwise {
    fn log_size(&self) -> u32 {
        BITWISE_LOOKUP_BITS // for all 8-bit × 8-bit combinations
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        // Generate values based on column index:
        // Column 0: input1 values (high bits)
        // Column 1: input2 values (low bits)
        // Column 2: result values (operation(input1, input2))
        let values: Vec<M31> = (0..1 << BITWISE_LOOKUP_BITS)
            .map(|i| {
                let input1 = i >> BITWISE_OPERAND_BITS;
                let input2 = i & BITWISE_OPERAND_MASK;
                match self.col_index {
                    0 => M31(input1),
                    1 => M31(input2),
                    2 => {
                        let result = self.operation.apply(input1, input2);
                        M31(result)
                    }
                    _ => unreachable!(),
                }
            })
            .collect();

        CircleEvaluation::new(
            CanonicCoset::new(self.log_size()).circle_domain(),
            BaseColumn::from_iter(values),
        )
    }

    fn id(&self) -> PreProcessedColumnId {
        PreProcessedColumnId {
            id: format!("bitwise_{}_col_{}", self.operation.name(), self.col_index),
        }
    }
}
