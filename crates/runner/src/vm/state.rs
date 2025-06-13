use num_traits::One;
use stwo_prover::core::fields::m31::M31;

/// The state of the VM, updated at each step.
///
/// * `pc` is the program counter, pointing to the current instruction.
/// * `fp` is the frame pointer, pointing to the current frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub pc: M31,
    pub fp: M31,
}

impl State {
    /// Regular register update.
    /// Advance the program counter by 1.
    pub fn advance(self) -> Self {
        Self {
            fp: self.fp,
            pc: self.pc + M31::one(),
        }
    }

    /// Absolute jump register update.
    /// Set the program counter to the offset.
    pub const fn jump_abs(self, offset: M31) -> Self {
        Self {
            pc: offset,
            fp: self.fp,
        }
    }

    /// Relative jump register update.
    /// Increment the program counter by the offset.
    pub fn jump_rel(self, offset: M31) -> Self {
        Self {
            pc: self.pc + offset,
            fp: self.fp,
        }
    }

    /// Call register update.
    /// The update of PC is absolute.
    /// Set the program counter to the given value.
    /// Update the frame pointer to the given value.
    /// * pc - The next PC.
    /// * fp_offset - The offset to the new frame pointer.
    pub fn call_abs(self, pc: M31, fp_offset: M31) -> Self {
        Self {
            pc,
            fp: self.fp + fp_offset,
        }
    }

    /// Call register update.
    /// The update of PC is relative to the current PC.
    /// Update the program counter by the given offset.
    /// Update the frame pointer to the given value.
    /// * pc_offset - The offset to the next PC.
    /// * fp_offset - The offset to the new frame pointer.
    pub fn call_rel(self, pc_offset: M31, fp_offset: M31) -> Self {
        Self {
            pc: self.pc + pc_offset,
            fp: self.fp + fp_offset,
        }
    }

    /// Function return register update.
    /// Set the program counter to the given value.
    /// Set the frame pointer to the given value.
    /// * pc - The next PC.
    /// * fp - The caller's frame pointer.
    pub fn ret(self, pc: M31, fp: M31) -> Self {
        Self { pc, fp }
    }
}

#[cfg(test)]
mod tests {
    use num_traits::{One, Zero};

    use crate::vm::State;
    use stwo_prover::core::fields::m31::M31;

    #[test]
    fn test_state_default() {
        let state = State::default();
        let expected_state = State {
            pc: M31::zero(),
            fp: M31::zero(),
        };
        assert_eq!(state, expected_state);
    }

    #[test]
    fn test_state_advance() {
        let state = State::default();
        let expected_state = State {
            pc: M31::one(),
            fp: M31::zero(),
        };

        let new_state = state.advance();

        assert_eq!(new_state, expected_state);
    }

    #[test]
    fn test_state_jump_abs() {
        let state = State::default();
        let expected_state = State {
            pc: M31(5),
            fp: M31::zero(),
        };

        let new_state = state.jump_abs(M31(5));

        assert_eq!(new_state, expected_state);
    }

    #[test]
    fn test_state_jump_rel() {
        let state = State {
            pc: M31(10),
            fp: M31::zero(),
        };
        let expected_state = State {
            pc: M31(15),
            fp: M31::zero(),
        };

        let new_state = state.jump_rel(M31(5));

        assert_eq!(new_state, expected_state);
    }
}
