use num_traits::Zero;
use stwo::prover::backend::simd::conversion::Pack;
use stwo::prover::backend::simd::m31::{PackedM31, N_LANES};
use stwo::core::fields::m31::M31;

use crate::adapter::ExecutionBundle;
use cairo_m_common::instruction::INSTRUCTION_MAX_SIZE;

// Flattened PackedExecutionBundle that contains all the M31 components as separate PackedM31 vectors
// This structure is optimized for SIMD operations
#[derive(Debug, Clone, Copy)]
pub struct PackedExecutionBundle {
    // VM registers (2 fields)
    pub pc: PackedM31,
    pub fp: PackedM31,

    pub clock: PackedM31,

    pub inst_prev_clock: PackedM31,
    pub inst_value_0: PackedM31,
    pub inst_value_1: PackedM31,
    pub inst_value_2: PackedM31,
    pub inst_value_3: PackedM31,
    pub inst_value_4: PackedM31,
    pub inst_value_5: PackedM31,

    pub span_start: [usize; N_LANES],
    pub span_len: [u16; N_LANES],
}

impl Pack for ExecutionBundle {
    type SimdType = PackedExecutionBundle;

    fn pack(inputs: [Self; N_LANES]) -> Self::SimdType {
        // Cache instruction M31 vectors once per lane
        let inst_values: [smallvec::SmallVec<[M31; INSTRUCTION_MAX_SIZE]>; N_LANES] =
            std::array::from_fn(|i| inputs[i].instruction.instruction.to_smallvec());

        PackedExecutionBundle {
            // Pack VM registers
            pc: PackedM31::from_array(std::array::from_fn(|i| inputs[i].registers.pc)),
            fp: PackedM31::from_array(std::array::from_fn(|i| inputs[i].registers.fp)),

            // Pack clock
            clock: PackedM31::from_array(std::array::from_fn(|i| inputs[i].clock)),

            // Memory arg 0 - Instruction (variable size, up to 5 M31s)
            inst_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.prev_clock
            })),
            // Pack instruction M31 values with padding for smaller instructions
            inst_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inst_values[i].first().copied().unwrap_or_else(M31::zero)
            })),
            inst_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inst_values[i].get(1).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inst_values[i].get(2).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inst_values[i].get(3).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_4: PackedM31::from_array(std::array::from_fn(|i| {
                inst_values[i].get(4).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_5: PackedM31::from_array(std::array::from_fn(|i| {
                inst_values[i].get(5).copied().unwrap_or_else(M31::zero)
            })),

            span_start: std::array::from_fn(|i| inputs[i].access_span.start as usize),
            span_len: std::array::from_fn(|i| inputs[i].access_span.len),
        }
    }
}
