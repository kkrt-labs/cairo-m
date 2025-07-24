use std::collections::HashMap;
use std::ops::Range;

use cairo_m_common::{PublicAddressRanges, State as VmRegisters};
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::Relation;
use stwo_prover::core::fields::FieldExpOps;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{QM31, SecureField};

use crate::adapter::ProverInput;
use crate::adapter::merkle::TREE_HEIGHT;
use crate::components::Relations;
use crate::relations;

/// Structured public entries for initial and final memory
///
/// This struct is used to store the public entries for the initial and final memory.
/// It containts:
/// - program: the instructions
/// - input: the main arguments
/// - output: the return values
///
/// The entries are stored as a vector of tuples, where the first element is the address,
/// the second element is the value, and the third element is the clock.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicEntries {
    pub program: Vec<Option<(M31, QM31, M31)>>,
    pub input: Vec<Option<(M31, QM31, M31)>>,
    pub output: Vec<Option<(M31, QM31, M31)>>,
}

impl PublicEntries {
    pub fn new(
        memory: &HashMap<(M31, M31), (QM31, M31, M31)>,
        public_address_ranges: &PublicAddressRanges,
    ) -> Self {
        // Pre-allocate with known sizes for better memory efficiency
        let program = Self::extract_range_with_capacity(memory, &public_address_ranges.program);
        let input = Self::extract_range_with_capacity(memory, &public_address_ranges.input);
        let output = Self::extract_range_with_capacity(memory, &public_address_ranges.output);

        Self {
            program,
            input,
            output,
        }
    }

    fn extract_range_with_capacity(
        memory: &HashMap<(M31, M31), (QM31, M31, M31)>,
        range: &Range<u32>,
    ) -> Vec<Option<(M31, QM31, M31)>> {
        let capacity = (range.end - range.start) as usize;
        let mut result = Vec::with_capacity(capacity);

        for addr_u32 in range.start..range.end {
            let addr = M31::from(addr_u32);
            let entry = memory
                .get(&(addr, M31::from(TREE_HEIGHT)))
                .map(|&(value, clock, _)| (addr, value, clock));
            result.push(entry);
        }

        result
    }

    /// Returns the program output values
    pub fn get_output_values(&self) -> Vec<Option<QM31>> {
        self.output
            .iter()
            .map(|entry| entry.map(|(_, value, _)| value))
            .collect()
    }

    /// Returns the program input values
    pub fn get_input_values(&self) -> Vec<Option<QM31>> {
        self.input
            .iter()
            .map(|entry| entry.map(|(_, value, _)| value))
            .collect()
    }

    /// Returns the program instructions
    pub fn get_program_values(&self) -> Vec<Option<QM31>> {
        self.program
            .iter()
            .map(|entry| entry.map(|(_, value, _)| value))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicData {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub initial_root: M31,
    pub final_root: M31,
    pub initial_public_entries: PublicEntries,
    pub final_public_entries: PublicEntries,
}

impl PublicData {
    pub fn new(input: &ProverInput) -> Self {
        Self {
            initial_registers: input.instructions.initial_registers,
            final_registers: input.instructions.final_registers,
            initial_root: input
                .merkle_trees
                .initial_root
                .expect("Initial memory root is required"),
            final_root: input
                .merkle_trees
                .final_root
                .expect("Final memory root is required"),
            initial_public_entries: PublicEntries::new(
                &input.memory.initial_memory,
                &input.public_address_ranges,
            ),
            final_public_entries: PublicEntries::new(
                &input.memory.final_memory,
                &input.public_address_ranges,
            ),
        }
    }

    pub fn initial_logup_sum(&self, relations: &Relations) -> SecureField {
        let mut values_to_inverse = vec![
            <relations::Registers as Relation<M31, QM31>>::combine(
                &relations.registers,
                &[self.initial_registers.pc, self.initial_registers.fp],
            ),
            -<relations::Registers as Relation<M31, QM31>>::combine(
                &relations.registers,
                &[self.final_registers.pc, self.final_registers.fp],
            ),
            -<relations::Merkle as Relation<M31, QM31>>::combine(
                &relations.merkle,
                &[
                    M31::zero(),
                    M31::zero(),
                    self.initial_root,
                    self.initial_root,
                ],
            ),
            -<relations::Merkle as Relation<M31, QM31>>::combine(
                &relations.merkle,
                &[M31::zero(), M31::zero(), self.final_root, self.final_root],
            ),
        ];

        let mut add_to_memory_relation =
            |entries: &[Option<(M31, QM31, M31)>], multiplicity: QM31| {
                for (addr, value, clock) in entries.iter().flatten() {
                    let value_array = value.to_m31_array();
                    values_to_inverse.push(
                        multiplicity
                            * <relations::Memory as Relation<M31, QM31>>::combine(
                                &relations.memory,
                                &[
                                    *addr,
                                    *clock,
                                    value_array[0],
                                    value_array[1],
                                    value_array[2],
                                    value_array[3],
                                ],
                            ),
                    );
                }
            };

        // Emit the initial public memory
        add_to_memory_relation(&self.initial_public_entries.program, QM31::one());
        add_to_memory_relation(&self.initial_public_entries.input, QM31::one());
        add_to_memory_relation(&self.initial_public_entries.output, QM31::one());

        // Use the final public memory
        add_to_memory_relation(&self.final_public_entries.program, -QM31::one());
        add_to_memory_relation(&self.final_public_entries.input, -QM31::one());
        add_to_memory_relation(&self.final_public_entries.output, -QM31::one());

        let inverted_values = QM31::batch_inverse(&values_to_inverse);
        inverted_values.iter().sum::<QM31>()
    }
}
