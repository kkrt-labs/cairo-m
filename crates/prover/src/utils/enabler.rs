use num_traits::{One, Zero};
use stwo_prover::core::backend::simd::m31::{PackedM31, N_LANES};
use stwo_prover::core::fields::m31::M31;

/// Used to select/enable active rows when padding traces to powers of 2.
///
/// The enabler creates a boolean column where the first `padding_offset` rows
/// are set to 1 (enabled) and the remaining rows are set to 0 (disabled).
/// This allows constraints to be selectively applied only to real execution data.
///
/// ## Trace Layout
/// ```text
/// Row:     0   1   2  ...  padding_offset-1  padding_offset  ...  trace_length-1
/// Enabled: 1   1   1  ...        1                 0         ...        0
///          ^-- Real execution --^              ^-- Padding rows --^
/// ```
///
/// ## SIMD Optimization
/// The enabler works with SIMD-packed field elements, handling partial
/// enablement within SIMD lanes when the boundary falls within a packed element.
#[derive(Debug, Clone)]
pub struct Enabler {
    /// Number of active (non-padded) rows in the trace
    pub padding_offset: usize,
}
impl Enabler {
    /// Creates a new enabler for a trace with the specified number of active rows.
    ///
    /// ## Arguments
    /// * `padding_offset` - Number of real execution rows (non-padded)
    ///
    /// ## Returns
    /// An enabler that will produce 1 for the first `padding_offset` rows
    /// and 0 for all subsequent padding rows.
    pub const fn new(padding_offset: usize) -> Self {
        Self { padding_offset }
    }

    /// Returns the packed enabler values for a SIMD row.
    ///
    /// ## Arguments
    /// * `vec_row` - The SIMD row index (not individual element index)
    ///
    /// ## Returns
    /// `PackedM31` containing enabler values for all SIMD lanes:
    /// - `1` for active execution rows
    /// - `0` for padding rows
    ///
    /// ## Example
    /// ```text
    /// padding_offset = 10, N_LANES = 8
    ///
    /// vec_row 0: lanes 0-7   → all enabled  → PackedM31::one()
    /// vec_row 1: lanes 8-15  → lanes 8,9 enabled, 10-15 disabled → mixed
    /// vec_row 2: lanes 16-23 → all disabled → PackedM31::zero()
    /// ```
    pub fn packed_at(&self, vec_row: usize) -> PackedM31 {
        let row_offset = vec_row * N_LANES;

        // Case 1: All SIMD lanes are in padding region
        if row_offset >= self.padding_offset {
            return PackedM31::zero();
        }

        // Case 2: All SIMD lanes are in active region
        if row_offset + N_LANES <= self.padding_offset {
            return PackedM31::one();
        }

        // Case 3: Partial enablement - some lanes active, some padding
        let mut res = [M31::zero(); N_LANES];
        let enabled_lanes = self.padding_offset - row_offset;
        res[..enabled_lanes].fill(M31::one());
        PackedM31::from_array(res)
    }
}
