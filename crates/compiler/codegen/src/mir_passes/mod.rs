//! Target-specific MIR passes that prepare MIR for CASM lowering.
//!
//! Passes in this module are intended to keep the generic MIR clean and
//! move backend quirks or instruction-set constraints into codegen.

pub mod legalize;
