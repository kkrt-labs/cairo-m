use cairo_m_common::State as VmRegisters;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::Relation;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{SecureField, QM31};
use stwo_prover::core::fields::FieldExpOps;

use crate::adapter::ProverInput;
use crate::components::Relations;
use crate::relations;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicData {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub initial_root: M31,
    pub final_root: M31,
}

impl PublicData {
    pub const fn new(input: &ProverInput) -> Self {
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
