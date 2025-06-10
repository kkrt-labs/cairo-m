pub mod instructions;

use stwo_prover::core::fields::m31::M31;

/// The state of the VM, updated at each step.
///
/// * `pc` is the program counter, pointing to the current instruction.
/// * `fp` is the frame pointer, pointing to the current frame.
#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct State {
    fp: M31,
    pc: M31,
}

#[cfg(test)]
mod tests {
    use num_traits::Zero;
    use stwo_prover::core::fields::m31::M31;

    use crate::vm::State;

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.fp, M31::zero());
        assert_eq!(state.pc, M31::zero());
    }
}
