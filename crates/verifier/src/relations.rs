#![allow(non_camel_case_types)]
use stwo_constraint_framework::relation;

relation!(Poseidon2, 16); // state

/// Logup security is defined by the `QM31` space:
/// (~124 bits) + `INTERACTION_POW_BITS` - log2(number of relation terms).
///
/// E.g. assuming a 100-bit security target, the witness may contain up to
/// 1 << (24 + INTERACTION_POW_BITS) relation terms.
pub const INTERACTION_POW_BITS: u32 = 2;
