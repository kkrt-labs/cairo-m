pub mod instructions;

use num_traits::One;
use stwo_prover::core::fields::m31::M31;

#[derive(Clone, Copy, Debug, Default)]
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

#[cfg(test)]
mod tests {
    use num_traits::{One, Zero};
    use stwo_prover::core::fields::m31::M31;

    use crate::vm::State;

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.fp, M31::zero());
        assert_eq!(state.pc, M31::zero());
    }

    #[test]
    fn test_state_advance() {
        let state = State::default();
        let new_state = state.advance();
        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());
    }
}
