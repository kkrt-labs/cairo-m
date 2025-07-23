use cairo_m_common::State as VmRegisters;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::Relation;
use stwo_prover::core::fields::FieldExpOps;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{QM31, SecureField};

use crate::adapter::ProverInput;
use crate::adapter::merkle::TREE_HEIGHT;
use crate::components::Relations;
use crate::relations;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicData {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub initial_root: M31,
    pub final_root: M31,
    pub public_entries: Vec<Option<(M31, QM31, M31)>>,
}

impl PublicData {
    pub fn new(input: &ProverInput) -> Self {
        // Extract public entries from final memory at public addresses
        let public_entries = input
            .public_addresses
            .iter()
            .map(|&addr| {
                // Look up the value in final memory at (addr, TREE_HEIGHT)
                input
                    .memory
                    .final_memory
                    .get(&(addr, M31::from(TREE_HEIGHT)))
                    .map(|&(value, clock, _)| (addr, value, clock))
            })
            .collect();

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
            public_entries,
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

        // Add memory relation entries for public addresses
        for (addr, value, clock) in self.public_entries.iter().flatten() {
            let value_array = value.to_m31_array();
            values_to_inverse.push(-<relations::Memory as Relation<M31, QM31>>::combine(
                &relations.memory,
                &[
                    *addr,
                    *clock,
                    value_array[0],
                    value_array[1],
                    value_array[2],
                    value_array[3],
                ],
            ));
        }

        let inverted_values = QM31::batch_inverse(&values_to_inverse);
        inverted_values.iter().sum::<QM31>()
    }
}
