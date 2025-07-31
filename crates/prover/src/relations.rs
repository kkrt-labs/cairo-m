#![allow(non_camel_case_types)]
use stwo_constraint_framework::relation;

relation!(RangeCheck20, 1); // value
relation!(Memory, 6); // addr, clock, value0, value1, value2, value3
relation!(Registers, 2); // pc, fp
relation!(Merkle, 4); // index, layer, value, root
relation!(Poseidon2, 16); // state

/// Logup security is defined by the `QM31` space:
/// (~124 bits) + `INTERACTION_POW_BITS` - log2(number of relation terms).
///
/// E.g. assuming a 100-bit security target, the witness may contain up to
/// 1 << (24 + INTERACTION_POW_BITS) relation terms.
pub const INTERACTION_POW_BITS: u32 = 2;
