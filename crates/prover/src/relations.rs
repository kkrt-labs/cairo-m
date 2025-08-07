#![allow(non_camel_case_types)]
use stwo_constraint_framework::relation;

// 20-bit range check relation for field arithmetic bounds.
// Ensures values are within the valid range [0, 2^20).
// Structure: value (the field element to range check)
relation!(RangeCheck20, 1);

// Memory access relation for read/write operations.
// Tracks all memory operations with address, clock, and M31 values.
// Structure: addr, clock, value0-3 (M31 value components)
relation!(Memory, 6);

// VM register state relation for PC and FP tracking.
// Maintains consistency of program counter and frame pointer updates.
// Structure: pc (program counter), fp (frame pointer)
relation!(Registers, 2);

// Merkle tree node relation for memory commitments.
// Ensures correct Merkle tree construction and hash computations.
// Structure: index (node index), layer (tree depth), value (hash), root (tree root hash)
relation!(Merkle, 4);

// Poseidon2 hash function relation for cryptographic computations.
// Connects the Poseidon2 component to the rest of the components: Poseidon2 component uses the initial
// state and emits the final digest. Other component that need to prove Poseidon2 computation should
// emit the initial state and consume the final digest.
// Structure: 16-element state array for Poseidon2 permutation
relation!(Poseidon2, 16);

/// Proof-of-work bits for interaction argument security.
pub const INTERACTION_POW_BITS: u32 = 2;
