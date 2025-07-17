//! Poseidon constants generated at compile-time and converted to M31 at runtime.
//! Constants are computed during build and cached using OnceLock for zero runtime cost.

use std::sync::OnceLock;

use stwo_prover::core::fields::m31::M31;

// Include the generated constants
include!(concat!(env!("OUT_DIR"), "/poseidon_constants_generated.rs"));

// Cached M31 constants
static ROUND_CONSTANTS_CACHE: OnceLock<Vec<M31>> = OnceLock::new();
static MDS_MATRIX_CACHE: OnceLock<Vec<Vec<M31>>> = OnceLock::new();

/// Get the round constants for Poseidon hash (converted from u32 to M31)
pub fn round_constants() -> &'static [M31] {
    ROUND_CONSTANTS_CACHE
        .get_or_init(|| ROUND_CONSTANTS_U32.iter().map(|&x| M31::from(x)).collect())
}

/// Get the MDS matrix for Poseidon hash (converted from u32 to M31)
pub fn mds_matrix() -> &'static Vec<Vec<M31>> {
    MDS_MATRIX_CACHE.get_or_init(|| {
        MDS_MATRIX_U32
            .iter()
            .map(|row| row.iter().map(|&x| M31::from(x)).collect())
            .collect()
    })
}
