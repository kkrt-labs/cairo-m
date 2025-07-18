use num_traits::Zero;
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{N_LANES, PackedM31};
use stwo_prover::core::fields::m31::M31;

use crate::adapter::ExecutionBundle;

// Flattened PackedExecutionBundle that contains all the M31 components as separate PackedM31 vectors
// This structure is optimized for SIMD operations
// Note: For operands (mem1-3), we only store single M31 values since DataAccess only has M31 fields
#[derive(Debug, Clone, Copy)]
pub struct PackedExecutionBundle {
    // VM registers (2 fields)
    pub pc: PackedM31,
    pub fp: PackedM31,

    pub clock: PackedM31,

    // Memory arg 0 - Instruction (8 fields: 4 value M31s, prev_clock)
    pub inst_prev_clock: PackedM31,
    pub inst_value_0: PackedM31,
    pub inst_value_1: PackedM31,
    pub inst_value_2: PackedM31,
    pub inst_value_3: PackedM31,

    // Memory arg 1 - Operand 0 (4 fields: address, value, prev_value, prev_clock)
    pub mem1_prev_value: PackedM31,
    pub mem1_value: PackedM31,
    pub mem1_prev_clock: PackedM31,

    // Memory arg 2 - Operand 1 (4 fields)
    pub mem2_prev_value: PackedM31,
    pub mem2_value: PackedM31,
    pub mem2_prev_clock: PackedM31,

    // Memory arg 3 - Operand 2 (4 fields)
    pub mem3_prev_value: PackedM31,
    pub mem3_value: PackedM31,
    pub mem3_prev_clock: PackedM31,
}

impl Pack for ExecutionBundle {
    type SimdType = PackedExecutionBundle;

    fn pack(inputs: [Self; N_LANES]) -> Self::SimdType {
        PackedExecutionBundle {
            // Pack VM registers
            pc: PackedM31::from_array(std::array::from_fn(|i| inputs[i].registers.pc)),
            fp: PackedM31::from_array(std::array::from_fn(|i| inputs[i].registers.fp)),

            // Pack clock
            clock: PackedM31::from_array(std::array::from_fn(|i| inputs[i].clock)),

            // Memory arg 0 - Instruction (value is QM31)
            inst_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.prev_clock
            })),
            inst_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.0.0
            })),
            inst_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.0.1
            })),
            inst_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.1.0
            })),
            inst_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.1.1
            })),

            // Memory arg 1
            mem1_prev_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_value)
            })),
            mem1_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.value)
            })),
            mem1_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_clock)
            })),

            // Memory arg 2
            mem2_prev_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_value)
            })),
            mem2_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.value)
            })),
            mem2_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_clock)
            })),

            // Memory arg 3
            mem3_prev_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_value)
            })),
            mem3_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.value)
            })),
            mem3_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_clock)
            })),
        }
    }
}
