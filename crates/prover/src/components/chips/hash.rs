use serde::{Deserialize, Serialize};
pub use stwo_constraint_framework::EvalAtRow;

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Hash {}

impl Hash {
    /// Implements a mock hash function that mimics XOR behavior using field operations.
    ///
    /// Since XOR is not defined for field elements in the constraint system, we simulate
    /// XOR-like behavior using addition. In the M31 field (2^31 - 1), addition provides
    /// a mixing function similar to XOR for testing purposes.
    ///
    /// Note: This is NOT a cryptographically secure hash function and should only be
    /// used for testing the Merkle tree structure in the constraint system.
    pub fn evaluate<E: EvalAtRow>([left_value, right_value]: [E::F; 2], _eval: &mut E) -> E::F {
        // Use field addition as a simple mixing function
        // This provides deterministic output that depends on both inputs
        left_value + right_value
    }
}
