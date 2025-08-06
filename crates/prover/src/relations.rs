#![allow(non_camel_case_types)]
use stwo_constraint_framework::relation;

// 20-bit range check relation for field arithmetic bounds.
//
// Ensures values are within the valid range [0, 2^20).
//
// ## Structure
// - **value**: The field element to range check
relation!(RangeCheck20, 1);

// Memory access relation for read/write operations.
//
// Tracks all memory operations with address, clock, and QM31 values.
//
// ## Structure
// - **addr**: Memory address (M31)
// - **clock**: Access timestamp (M31)
// - **value0-3**: QM31 value components (4 M31 elements)
relation!(Memory, 6);

// VM register state relation for PC and FP tracking.
//
// Maintains consistency of program counter and frame pointer updates.
//
// ## Structure
// - **pc**: Program counter (M31)
// - **fp**: Frame pointer (M31)
relation!(Registers, 2);

// Merkle tree node relation for memory commitments.
//
// Ensures correct Merkle tree construction and hash computations.
//
// ## Structure
// - **index**: Node index at tree level (M31)
// - **layer**: Tree depth/layer (M31)
// - **value**: Hash value at this node (M31)
// - **root**: Tree root hash (M31)
relation!(Merkle, 4);

// Poseidon2 hash function relation for cryptographic computations.
//
// Connects the Poseidon2 component to the rest of the components: Poseidon2 component uses the initial
// state and emits the final digest. Other component that need to prove Poseidon2 computation should
// emit the initial state and comsume the final digest.
//
// ## Structure
// - **state**: 16-element state array for Poseidon2 permutation
relation!(Poseidon2, 16);

// Proof-of-work bits for interaction argument security.
pub const INTERACTION_POW_BITS: u32 = 2;
