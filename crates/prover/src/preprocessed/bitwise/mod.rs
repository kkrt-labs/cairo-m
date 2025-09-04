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

pub struct Bitwise<F>
where
    F: Fn(u32, u32) -> u32 + Send + Sync,
{
    operation: F,
    name: String,
}

impl<F> Bitwise<F>
where
    F: Fn(u32, u32) -> u32 + Send + Sync,
{
    pub const fn new(operation: F, name: String) -> Self {
        Self { operation, name }
    }
}

impl<F> PreProcessedColumn for Bitwise<F>
where
    F: Fn(u32, u32) -> u32 + Send + Sync,
{
    fn log_size(&self) -> u32 {
        BITWISE_LOOKUP_BITS // for all 8-bit × 8-bit combinations
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        // Generate all 8-bit × 8-bit combinations and their results
        // The layout is: for each row i, we have:
        // - input1 = i >> BITWISE_OPERAND_BITS (high 8 bits)
        // - input2 = i & BITWISE_OPERAND_MASK (low 8 bits)
        // - result = operation(input1, input2)
        let values: Vec<M31> = (0..1 << BITWISE_LOOKUP_BITS)
            .map(|i| {
                let input1 = i >> BITWISE_OPERAND_BITS;
                let input2 = i & BITWISE_OPERAND_MASK;
                let result = (self.operation)(input1, input2);
                // We store the result in the column
                // The inputs are implicit from the row index
                M31(result)
            })
            .collect();

        CircleEvaluation::new(
            CanonicCoset::new(self.log_size()).circle_domain(),
            BaseColumn::from_iter(values),
        )
    }

    fn id(&self) -> PreProcessedColumnId {
        PreProcessedColumnId {
            id: format!("bitwise_{}", self.name),
        }
    }
}
