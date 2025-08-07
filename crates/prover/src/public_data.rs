//! # Public Data Management for Cairo-M STARK Proofs
//!
//! ## What is Public Data?
//!
//! In STARK systems, public data represents values that both prover and verifier
//! know and agree upon. This includes:
//!
//! - **Program Inputs**: Initial values provided to the program
//! - **Program Outputs**: Final results produced by execution
//! - **Program Code**: The actual instructions being executed
//! - **VM State**: Initial and final register values
//! - **Memory Commitments**: Merkle roots of initial and final memory states
//!
//! ## Role in Proof Verification
//!
//! The verifier uses public data to:
//! 1. **Verify Execution Bounds**: Check that execution started and ended correctly
//! 2. **Validate Memory Commitments**: Ensure memory state transitions are consistent
//! 3. **Check Input/Output Correctness**: Verify program consumed correct inputs and produced expected outputs
//! 4. **Check Program Correctness**: Verify that the program executed is correct
//! 5. **Maintain Lookup Consistency**: Ensure public values are properly accounted for in constraint system
//!
//! ## Logup Integration
//!
//! Public data participates in the lookup argument system by "consuming" public
//! memory entries, register states, and Merkle commitments that are "emitted" by
//! various components. This ensures that public values are properly integrated
//! into the overall constraint system.

use cairo_m_common::State as VmRegisters;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::Relation;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::{SecureField, QM31};
use stwo_prover::core::fields::FieldExpOps;

use crate::adapter::merkle::TREE_HEIGHT;
use crate::adapter::ProverInput;
use crate::components::Relations;
use crate::relations;

/// Public data accompanying a Cairo-M proof.
///
/// This structure contains all the information that must be publicly known
/// for proof verification. It serves as the "public input" to the verification
/// process, allowing verifiers to check proof validity without access to
/// private execution details.
///
/// ## Components
///
/// ### VM State Boundaries
/// - Initial and final register states establish execution boundaries
///
/// ### Memory Commitments
/// - Merkle roots commit to initial and final memory contents
/// - Checks for memory consistency between continuations
///
/// ### Public Memory Values
/// - Program inputs, outputs, and code that verifier needs to see
/// - Each entry contains (address, value, timestamp) tuple
/// - Integrated into lookup argument system for consistency
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicData {
    /// VM register state at start of execution (PC, FP)
    pub initial_registers: VmRegisters,
    /// VM register state at end of execution (PC, FP)
    pub final_registers: VmRegisters,
    /// Merkle root hash of initial memory state
    pub initial_root: M31,
    /// Merkle root hash of final memory state
    pub final_root: M31,
    /// Public memory entries: (address, value, clock) or None if unused
    /// Includes program code, inputs, and outputs that verifier must see
    pub public_entries: Vec<Option<(M31, M31, M31)>>,
}

impl PublicData {
    /// Constructs public data from prover input.
    ///
    /// This method extracts all the information that must be publicly visible
    /// for proof verification.
    ///
    /// ## Arguments
    /// * `input` - Complete prover input containing execution trace and memory state
    ///
    /// ## Returns
    /// Public data structure ready for proof generation and verification
    ///
    /// ## Panics
    /// Panics if initial or final Merkle roots are missing, as these are
    /// required for proof verification.
    pub fn new(input: &ProverInput) -> Self {
        // Extract public memory entries at specified addresses
        // Some addresses may not have been accessed during execution (None entries)
        let public_entries = input
            .public_addresses
            .iter()
            .map(|&addr| {
                // Look up the final state of this public address
                input
                    .memory
                    .final_memory
                    .get(&(addr, M31::from(TREE_HEIGHT)))
                    .map(|&(value, clock, _multiplicity)| (addr, value, clock))
            })
            .collect();

        Self {
            initial_registers: input.instructions.initial_registers,
            final_registers: input.instructions.final_registers,
            initial_root: input
                .merkle_trees
                .initial_root
                .expect("Initial memory root is required for verification"),
            final_root: input
                .merkle_trees
                .final_root
                .expect("Final memory root is required for verification"),
            public_entries,
        }
    }

    /// Computes the initial logup sum for public data in the lookup argument system.
    ///
    /// This method calculates the contribution of public data to the overall lookup
    /// argument sum. Public data consumes (or emits) values that are emitted (or consumed) by various
    /// components, ensuring that public values are properly accounted for in the
    /// constraint system.
    ///
    /// ## Lookup Integration
    ///
    /// 1. **Initial Registers**: Emitted by first opcode execution
    /// 2. **Final Registers**: Consumed to balance final state
    /// 3. **Memory Roots**: Consumed to balance Merkle tree emissions
    /// 4. **Public Memory**: Consumed to balance memory component emissions
    ///
    /// ## Arguments
    /// * `relations` - contains data for combining entries
    ///
    /// ## Returns
    /// The initial logup sum contribution from public data, used in the
    /// overall lookup argument verification.
    pub fn initial_logup_sum(&self, relations: &Relations) -> SecureField {
        let mut values_to_inverse = vec![
            // Emit initial registers
            <relations::Registers as Relation<M31, QM31>>::combine(
                &relations.registers,
                &[self.initial_registers.pc, self.initial_registers.fp],
            ),
            // Consume final registers
            -<relations::Registers as Relation<M31, QM31>>::combine(
                &relations.registers,
                &[self.final_registers.pc, self.final_registers.fp],
            ),
            // Consume initial memory root
            -<relations::Merkle as Relation<M31, QM31>>::combine(
                &relations.merkle,
                &[
                    M31::zero(),       // Root node index
                    M31::zero(),       // Root layer (depth 0)
                    self.initial_root, // Root node value
                    self.initial_root, // Tree root
                ],
            ),
            // Consume final memory root
            -<relations::Merkle as Relation<M31, QM31>>::combine(
                &relations.merkle,
                &[M31::zero(), M31::zero(), self.final_root, self.final_root],
            ),
        ];

        // Consume public memory entries
        for (addr, value, clock) in self.public_entries.iter().flatten() {
            values_to_inverse.push(-<relations::Memory as Relation<M31, QM31>>::combine(
                &relations.memory,
                &[*addr, *clock, *value],
            ));
        }

        // Batch invert for efficiency and sum all contributions
        let inverted_values = QM31::batch_inverse(&values_to_inverse);
        inverted_values.iter().sum::<QM31>()
    }
}
