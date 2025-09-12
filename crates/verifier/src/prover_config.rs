//! Configurations for the CSTARK prover.
//!
//! Conjecture of n-bit security level: `n = n_queries * log_blowup_factor + pow_bits`.

use stwo_prover::core::fri::FriConfig;
use stwo_prover::core::pcs::PcsConfig;

/// Configuration to achieve 96-bit security level, with PoW bits inferior to 20.
///
/// - The blowup factor greatly influences the proving time.
/// - The number of queries influences the proof size.
/// - The PoW bits influence the proving time, depending on the hardware and the number of bits to grind.
pub const REGULAR_96_BITS: PcsConfig = PcsConfig {
    pow_bits: 16,
    fri_config: FriConfig {
        log_last_layer_degree_bound: 0,
        log_blowup_factor: 1,
        n_queries: 80,
    },
};
