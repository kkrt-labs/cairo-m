use stwo_prover::core::fields::m31::M31;

use cairo_m_prover::poseidon2::T;

pub type HashInput = [M31; T];

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ProverInput {
    pub poseidon2_inputs: Vec<HashInput>,
}
