use cairo_m_common::State;
use stwo::core::fields::m31::M31;

pub trait VmState {
    fn advance_by(self, offset: u32) -> Self;
    fn jump_abs(self, offset: M31) -> Self;
    fn jump_rel(self, offset: M31) -> Self;
    fn call_abs(self, pc: M31, fp_offset: M31) -> Self;
    fn call_rel(self, pc_offset: M31, fp_offset: M31) -> Self;
    fn ret(self, pc: M31, fp: M31) -> Self;
}

impl VmState for State {
    /// Regular register update.
    /// Advance the program counter by the given offset (in QM31 memory units).
    fn advance_by(self, offset: u32) -> Self {
        Self {
            pc: self.pc + M31::from(offset),
            fp: self.fp,
        }
    }

    /// Absolute jump register update.
    /// Set the program counter to the offset.
    fn jump_abs(self, offset: M31) -> Self {
        Self {
            pc: offset,
            fp: self.fp,
        }
    }

    /// Relative jump register update.
    /// Increment the program counter by the offset.
    fn jump_rel(self, offset: M31) -> Self {
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
    fn call_abs(self, pc: M31, fp_offset: M31) -> Self {
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
    fn call_rel(self, pc_offset: M31, fp_offset: M31) -> Self {
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
    fn ret(self, pc: M31, fp: M31) -> Self {
        Self { pc, fp }
    }
}

#[cfg(test)]
mod tests {
    use cairo_m_common::State;
    use num_traits::{One, Zero};
    use stwo::core::fields::m31::M31;

    use crate::vm::state::VmState;

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
    fn test_state_advance_by() {
        let state = State::default();
        let expected_state = State {
            pc: M31::one(),
            fp: M31::zero(),
        };

        let new_state = state.advance_by(1);

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
