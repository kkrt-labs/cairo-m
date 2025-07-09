use cairo_m_common::State as VmRegisters;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::Relation;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{SecureField, QM31};
use stwo_prover::core::fields::FieldExpOps;

use crate::adapter::{partial_merkle, ProverInput};
use crate::components::Relations;
use crate::relations;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicData {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub initial_memory_root: M31,
    pub final_memory_root: M31,
}

impl PublicData {
    pub fn new(input: &ProverInput) -> Self {
        Self {
            initial_registers: input.instructions.initial_registers,
            final_registers: input.instructions.final_registers,
            initial_memory_root: partial_merkle::get_merkle_root(
                &input.merkle_trees.initial_merkle_tree,
            ),
            final_memory_root: partial_merkle::get_merkle_root(
                &input.merkle_trees.final_merkle_tree,
            ),
        }
    }

    pub fn initial_logup_sum(&self, relations: &Relations) -> SecureField {
        let values_to_inverse = vec![
            <relations::Registers as Relation<M31, QM31>>::combine(
                &relations.registers,
                &[self.initial_registers.pc, self.initial_registers.fp],
            ),
            -<relations::Registers as Relation<M31, QM31>>::combine(
                &relations.registers,
                &[self.final_registers.pc, self.final_registers.fp],
            ),
        ];

        let inverted_values = QM31::batch_inverse(&values_to_inverse);
        inverted_values.iter().sum::<QM31>()
    }
}
