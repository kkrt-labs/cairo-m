use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;

/// The state of the VM, updated at each step.
///
/// * `pc` is the program counter, pointing to the current instruction.
/// * `fp` is the frame pointer, pointing to the current frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub pc: M31,
    pub fp: M31,
}

impl From<(M31, M31)> for State {
    fn from((pc, fp): (M31, M31)) -> Self {
        Self { pc, fp }
    }
}
