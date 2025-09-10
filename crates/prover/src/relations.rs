#![allow(non_camel_case_types)]
use stwo_constraint_framework::relation;

// 8-bit range check relation for field arithmetic bounds.
// Ensures values are within the valid range [0, 2^8).
// Structure: value (the field element to range check)
relation!(RangeCheck8, 1);

// 16-bit range check relation for field arithmetic bounds.
// Ensures values are within the valid range [0, 2^16).
// Structure: value (the field element to range check)
relation!(RangeCheck16, 1);

// 20-bit range check relation for field arithmetic bounds.
// Ensures values are within the valid range [0, 2^20).
// Structure: value (the field element to range check)
relation!(RangeCheck20, 1);

// Memory access relation for read/write operations.
// Tracks all memory operations with address, clock, and QM31 values.
// Structure: addr, clock, value0-3 (QM31 value components)
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

// Bitwise operation relation for 8-bit values.
// Handles AND (id=0), OR (id=1), and XOR (id=2) operations.
// Structure: operation_id, input1, input2, result
relation!(Bitwise, 4);

// SHA256 hash function relation for cryptographic computations.
relation!(SmallSigma0, 6);
relation!(SmallSigma1_0, 5);
relation!(SmallSigma1_1, 7);
relation!(BigSigma0, 7);
relation!(BigSigma1, 6);
relation!(XorSmallSigma0, 4);
relation!(XorSmallSigma1, 4);
relation!(XorBigSigma0, 3);
relation!(XorBigSigma1, 4);
relation!(Ch, 4);
relation!(Maj, 4);

/// Proof-of-work bits for interaction argument security.
pub const INTERACTION_POW_BITS: u32 = 2;
