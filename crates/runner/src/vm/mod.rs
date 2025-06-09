pub mod instructions;

use num_traits::One;
use stwo_prover::core::fields::m31::M31;

#[derive(Clone, Copy, Debug)]
pub struct State {
    fp: M31,
    pc: M31,
}

impl State {
    pub fn advance(self) -> Self {
        Self {
            fp: self.fp,
            pc: self.pc + M31::one(),
        }
    }
}
