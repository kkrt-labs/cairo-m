use num_traits::Zero;
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{N_LANES, PackedM31};
use stwo_prover::core::fields::m31::M31;

use crate::adapter::ExecutionBundle;

// Flattened PackedExecutionBundle that contains all the M31 components as separate PackedM31 vectors
// This structure is optimized for SIMD operations
// Supports multi-limb values (U32) and variable-sized instructions (up to 5 M31s)
#[derive(Debug, Clone, Copy)]
pub struct PackedExecutionBundle {
    // VM registers (2 fields)
    pub pc: PackedM31,
    pub fp: PackedM31,

    pub clock: PackedM31,

    // Memory arg 0 - Instruction (up to 5 M31s for variable-sized instructions)
    pub inst_prev_clock: PackedM31,
    pub inst_value_0: PackedM31,
    pub inst_value_1: PackedM31,
    pub inst_value_2: PackedM31,
    pub inst_value_3: PackedM31,
    pub inst_value_4: PackedM31,

    // Memory arg 1 - Operand 0 (supports multi-limb values)
    pub mem1_addr: PackedM31,
    pub mem1_prev_clock: PackedM31,
    pub mem1_prev_value_limb0: PackedM31,
    pub mem1_prev_value_limb1: PackedM31,
    pub mem1_value_limb0: PackedM31,
    pub mem1_value_limb1: PackedM31,

    // Memory arg 2 - Operand 1 (supports multi-limb values)
    pub mem2_addr: PackedM31,
    pub mem2_prev_clock: PackedM31,
    pub mem2_prev_value_limb0: PackedM31,
    pub mem2_prev_value_limb1: PackedM31,
    pub mem2_value_limb0: PackedM31,
    pub mem2_value_limb1: PackedM31,

    // Memory arg 3 - Operand 2 (supports multi-limb values)
    pub mem3_addr: PackedM31,
    pub mem3_prev_clock: PackedM31,
    pub mem3_prev_value_limb0: PackedM31,
    pub mem3_prev_value_limb1: PackedM31,
    pub mem3_value_limb0: PackedM31,
    pub mem3_value_limb1: PackedM31,
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

            // Memory arg 0 - Instruction (variable size, up to 5 M31s)
            inst_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.prev_clock
            })),
            // Pack instruction M31 values with padding for smaller instructions
            inst_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                let inst_values = inputs[i].instruction.instruction.to_smallvec();
                inst_values.first().copied().unwrap_or_else(M31::zero)
            })),
            inst_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                let inst_values = inputs[i].instruction.instruction.to_smallvec();
                inst_values.get(1).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                let inst_values = inputs[i].instruction.instruction.to_smallvec();
                inst_values.get(2).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                let inst_values = inputs[i].instruction.instruction.to_smallvec();
                inst_values.get(3).copied().unwrap_or_else(M31::zero)
            })),
            inst_value_4: PackedM31::from_array(std::array::from_fn(|i| {
                let inst_values = inputs[i].instruction.instruction.to_smallvec();
                inst_values.get(4).copied().unwrap_or_else(M31::zero)
            })),

            // Memory arg 1 (multi-limb support)
            mem1_addr: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.address)
            })),
            mem1_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_clock)
            })),
            mem1_prev_value_limb0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_value.limb0)
            })),
            mem1_prev_value_limb1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_value.limb1)
            })),
            mem1_value_limb0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.value.limb0)
            })),
            mem1_value_limb1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.value.limb1)
            })),

            // Memory arg 2 (multi-limb support)
            mem2_addr: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.address)
            })),
            mem2_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_clock)
            })),
            mem2_prev_value_limb0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_value.limb0)
            })),
            mem2_prev_value_limb1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_value.limb1)
            })),
            mem2_value_limb0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.value.limb0)
            })),
            mem2_value_limb1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.value.limb1)
            })),

            // Memory arg 3 (multi-limb support)
            mem3_addr: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.address)
            })),
            mem3_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_clock)
            })),
            mem3_prev_value_limb0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_value.limb0)
            })),
            mem3_prev_value_limb1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_value.limb1)
            })),
            mem3_value_limb0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.value.limb0)
            })),
            mem3_value_limb1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.value.limb1)
            })),
        }
    }
}
