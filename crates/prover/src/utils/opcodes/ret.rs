use num_traits::Zero;
use stwo_prover::core::backend::simd::m31::{PackedM31, N_LANES};
use stwo_prover::core::fields::m31::M31;

use crate::adapter::ExecutionBundle;

#[derive(Debug, Clone, Copy)]
pub struct PackedExecutionBundle {
    pub pc: PackedM31,
    pub fp: PackedM31,
    pub clock: PackedM31,
    pub inst_prev_clock: PackedM31,
    pub opcode_id: PackedM31,
    pub off0: PackedM31,
    pub off1: PackedM31,
    pub off2: PackedM31,
    pub op0_prev_clock: PackedM31,
    pub op0_val: PackedM31,
    pub op1_prev_clock: PackedM31,
    pub op1_val: PackedM31,
}

impl PackedExecutionBundle {
    pub fn pack_from(bundles: [ExecutionBundle; N_LANES]) -> Self {
        Self {
            pc: PackedM31::from_array(std::array::from_fn(|i| bundles[i].registers.pc)),
            fp: PackedM31::from_array(std::array::from_fn(|i| bundles[i].registers.fp)),
            clock: PackedM31::from_array(std::array::from_fn(|i| bundles[i].clock)),
            inst_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                bundles[i].instruction.prev_clock
            })),
            opcode_id: PackedM31::from_array(std::array::from_fn(|i| {
                bundles[i].instruction.value.0 .0
            })),
            off0: PackedM31::from_array(std::array::from_fn(|i| bundles[i].instruction.value.0 .1)),
            off1: PackedM31::from_array(std::array::from_fn(|i| bundles[i].instruction.value.1 .0)),
            off2: PackedM31::from_array(std::array::from_fn(|i| bundles[i].instruction.value.1 .1)),
            op0_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                bundles[i].operands[0].map_or_else(M31::zero, |op| op.prev_clock)
            })),
            op0_val: PackedM31::from_array(std::array::from_fn(|i| {
                bundles[i].operands[0].map_or_else(M31::zero, |op| op.value)
            })),
            op1_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                bundles[i].operands[1].map_or_else(M31::zero, |op| op.prev_clock)
            })),
            op1_val: PackedM31::from_array(std::array::from_fn(|i| {
                bundles[i].operands[1].map_or_else(M31::zero, |op| op.value)
            })),
        }
    }
}
