use num_traits::{One, Zero};
use stwo_prover::core::backend::simd::conversion::Pack;
use stwo_prover::core::backend::simd::m31::{PackedM31, N_LANES};
use stwo_prover::core::fields::m31::M31;

use crate::adapter::StateData;

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
        if self.padding_offset <= row_offset {
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

// Flattened PackedStateData that contains all the M31 components as separate PackedM31 vectors
// This structure is optimized for SIMD operations
#[derive(Debug, Clone, Copy)]
pub struct PackedStateData {
    // VM registers (2 fields)
    pub pc: PackedM31,
    pub fp: PackedM31,

    // Memory arg 0 (10 fields: address, 4 prev_val M31s, 4 value M31s, prev_clock, clock)
    pub mem0_address: PackedM31,
    pub mem0_prev_val_0: PackedM31,
    pub mem0_prev_val_1: PackedM31,
    pub mem0_prev_val_2: PackedM31,
    pub mem0_prev_val_3: PackedM31,
    pub mem0_value_0: PackedM31,
    pub mem0_value_1: PackedM31,
    pub mem0_value_2: PackedM31,
    pub mem0_value_3: PackedM31,
    pub mem0_prev_clock: PackedM31,
    pub mem0_clock: PackedM31,

    // Memory arg 1 (10 fields)
    pub mem1_address: PackedM31,
    pub mem1_prev_val_0: PackedM31,
    pub mem1_prev_val_1: PackedM31,
    pub mem1_prev_val_2: PackedM31,
    pub mem1_prev_val_3: PackedM31,
    pub mem1_value_0: PackedM31,
    pub mem1_value_1: PackedM31,
    pub mem1_value_2: PackedM31,
    pub mem1_value_3: PackedM31,
    pub mem1_prev_clock: PackedM31,
    pub mem1_clock: PackedM31,

    // Memory arg 2 (10 fields)
    pub mem2_address: PackedM31,
    pub mem2_prev_val_0: PackedM31,
    pub mem2_prev_val_1: PackedM31,
    pub mem2_prev_val_2: PackedM31,
    pub mem2_prev_val_3: PackedM31,
    pub mem2_value_0: PackedM31,
    pub mem2_value_1: PackedM31,
    pub mem2_value_2: PackedM31,
    pub mem2_value_3: PackedM31,
    pub mem2_prev_clock: PackedM31,
    pub mem2_clock: PackedM31,

    // Memory arg 3 (10 fields)
    pub mem3_address: PackedM31,
    pub mem3_prev_val_0: PackedM31,
    pub mem3_prev_val_1: PackedM31,
    pub mem3_prev_val_2: PackedM31,
    pub mem3_prev_val_3: PackedM31,
    pub mem3_value_0: PackedM31,
    pub mem3_value_1: PackedM31,
    pub mem3_value_2: PackedM31,
    pub mem3_value_3: PackedM31,
    pub mem3_prev_clock: PackedM31,
    pub mem3_clock: PackedM31,
}

impl Pack for StateData {
    type SimdType = PackedStateData;

    fn pack(inputs: [Self; N_LANES]) -> Self::SimdType {
        PackedStateData {
            // Pack VM registers
            pc: PackedM31::from_array(std::array::from_fn(|i| inputs[i].registers.pc)),
            fp: PackedM31::from_array(std::array::from_fn(|i| inputs[i].registers.fp)),

            // Pack memory arg 0
            mem0_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].address
            })),
            mem0_prev_val_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].prev_val.to_m31_array()[0]
            })),
            mem0_prev_val_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].prev_val.to_m31_array()[1]
            })),
            mem0_prev_val_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].prev_val.to_m31_array()[2]
            })),
            mem0_prev_val_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].prev_val.to_m31_array()[3]
            })),
            mem0_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].value.to_m31_array()[0]
            })),
            mem0_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].value.to_m31_array()[1]
            })),
            mem0_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].value.to_m31_array()[2]
            })),
            mem0_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].value.to_m31_array()[3]
            })),
            mem0_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].prev_clock
            })),
            mem0_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[0].clock
            })),

            // Pack memory arg 1
            mem1_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].address
            })),
            mem1_prev_val_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].prev_val.to_m31_array()[0]
            })),
            mem1_prev_val_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].prev_val.to_m31_array()[1]
            })),
            mem1_prev_val_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].prev_val.to_m31_array()[2]
            })),
            mem1_prev_val_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].prev_val.to_m31_array()[3]
            })),
            mem1_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].value.to_m31_array()[0]
            })),
            mem1_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].value.to_m31_array()[1]
            })),
            mem1_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].value.to_m31_array()[2]
            })),
            mem1_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].value.to_m31_array()[3]
            })),
            mem1_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].prev_clock
            })),
            mem1_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[1].clock
            })),

            // Pack memory arg 2
            mem2_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].address
            })),
            mem2_prev_val_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].prev_val.to_m31_array()[0]
            })),
            mem2_prev_val_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].prev_val.to_m31_array()[1]
            })),
            mem2_prev_val_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].prev_val.to_m31_array()[2]
            })),
            mem2_prev_val_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].prev_val.to_m31_array()[3]
            })),
            mem2_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].value.to_m31_array()[0]
            })),
            mem2_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].value.to_m31_array()[1]
            })),
            mem2_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].value.to_m31_array()[2]
            })),
            mem2_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].value.to_m31_array()[3]
            })),
            mem2_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].prev_clock
            })),
            mem2_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[2].clock
            })),

            // Pack memory arg 3
            mem3_address: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].address
            })),
            mem3_prev_val_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].prev_val.to_m31_array()[0]
            })),
            mem3_prev_val_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].prev_val.to_m31_array()[1]
            })),
            mem3_prev_val_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].prev_val.to_m31_array()[2]
            })),
            mem3_prev_val_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].prev_val.to_m31_array()[3]
            })),
            mem3_value_0: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].value.to_m31_array()[0]
            })),
            mem3_value_1: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].value.to_m31_array()[1]
            })),
            mem3_value_2: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].value.to_m31_array()[2]
            })),
            mem3_value_3: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].value.to_m31_array()[3]
            })),
            mem3_prev_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].prev_clock
            })),
            mem3_clock: PackedM31::from_array(std::array::from_fn(|i| {
                inputs[i].memory_args[3].clock
            })),
        }
    }
}
