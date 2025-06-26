use num_traits::{One, Zero};
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{PackedM31, N_LANES};
use stwo_prover::core::fields::m31::M31;

use crate::adapter::ExecutionBundle;

/// The enabler column is a column of length `padding_offset.next_power_of_two()` where
/// 1. The first `padding_offset` elements are set to 1;
/// 2. The rest are set to 0.
#[derive(Debug, Clone)]
pub struct Enabler {
    pub padding_offset: usize,
}
impl Enabler {
    pub const fn new(padding_offset: usize) -> Self {
        Self { padding_offset }
    }

    pub fn packed_at(&self, vec_row: usize) -> PackedM31 {
        let row_offset = vec_row * N_LANES;
        if row_offset >= self.padding_offset {
            return PackedM31::zero();
        }
        if row_offset + N_LANES <= self.padding_offset {
            return PackedM31::one();
        }

        // The row is partially enabled.
        let mut res = [M31::zero(); N_LANES];
        for v in res.iter_mut().take(self.padding_offset - row_offset) {
            *v = M31::one();
        }
        PackedM31::from_array(res)
    }
}

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
    pub instr_prev_clock: PackedM31,
    pub instr_value_0: PackedM31,
    pub instr_value_1: PackedM31,
    pub instr_value_2: PackedM31,
    pub instr_value_3: PackedM31,

    // Memory arg 1 - Operand 0 (4 fields: address, value, prev_value, prev_clock)
    pub op1_address: PackedM31,
    pub op1_prev_value: PackedM31,
    pub op1_value: PackedM31,
    pub op1_prev_clock: PackedM31,

    // Memory arg 2 - Operand 1 (4 fields)
    pub op2_address: PackedM31,
    pub op2_prev_value: PackedM31,
    pub op2_value: PackedM31,
    pub op2_prev_clock: PackedM31,

    // Memory arg 3 - Operand 2 (4 fields)
    pub op3_address: PackedM31,
    pub op3_prev_value: PackedM31,
    pub op3_value: PackedM31,
    pub op3_prev_clock: PackedM31,
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

            // Pack instruction as memory arg 0
            instr_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.prev_clock
            })),
            instr_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.0 .0
            })),
            instr_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.0 .1
            })),
            instr_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.1 .0
            })),
            instr_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].instruction.value.1 .1
            })),

            // Pack operand 0 as memory arg 1
            op1_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.address)
            })),
            op1_prev_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_value)
            })),
            op1_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.value)
            })),
            op1_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[0].map_or_else(M31::zero, |op| op.prev_clock)
            })),

            // Pack operand 1 as memory arg 2
            op2_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.address)
            })),
            op2_prev_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_value)
            })),
            op2_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.value)
            })),
            op2_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[1].map_or_else(M31::zero, |op| op.prev_clock)
            })),

            // Pack operand 2 as memory arg 3
            op3_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.address)
            })),
            op3_prev_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_value)
            })),
            op3_value: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.value)
            })),
            op3_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].operands[2].map_or_else(M31::zero, |op| op.prev_clock)
            })),
        }
    }
}
