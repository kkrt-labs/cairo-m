//! # Stack Frame Layout Calculation
//!
//! This module implements the stack frame layout, which is crucial for mapping
//! MIR's `ValueId` to CASM's fp-relative memory model.
//!
//! The layout is calculated from the callee's perspective, where all arguments
//! and return value slots reside at negative offsets from the frame pointer (`fp`),
//! and all local variables and temporaries reside at positive offsets.
//!
//! ## Frame Layout (Callee's Perspective)
//!
//! A function with `M` arguments that returns `K` values has the following layout:
//!
//! ```text
//! | Address                    | Content                | Description                 |
//! |----------------------------|------------------------|-----------------------------|
//! | ...                        | ...                    |                             |
//! | fp - M - K - 2             | Argument 0             | First argument from caller  |
//! | ...                        | ...                    |                             |
//! | fp - K - 3                 | Argument M-1           | Last argument from caller   |
//! | fp - K - 2                 | Return value 0 slot    | First return value slot     |
//! | ...                        | ...                    |                             |
//! | fp - 3                     | Return value K-1 slot  | Last return value slot      |
//! | fp - 2                     | Caller's Frame Ptr     | Saved by `call` instruction |
//! | fp - 1                     | Return Address (PC)    | Saved by `call` instruction |
//! |----------------------------|------------------------|-----------------------------|
//! | fp + 0                     | Local/Temp 0           | First local variable        |
//! | fp + 1                     | Local/Temp 1           | Second local variable       |
//! | ...                        | ...                    |                             |
//! ```

use cairo_m_compiler_mir::{MirFunction, ValueId};
use rustc_hash::FxHashMap;

use crate::{CodegenError, CodegenResult};

/// Maps every ValueId in a function to its fp-relative memory offset.
#[derive(Debug, Clone)]
pub struct FunctionLayout {
    /// Maps ValueId to fp-relative offset.
    value_offsets: FxHashMap<ValueId, i32>,
    /// The current size of the local variable area on the stack (grows as locals are allocated).
    current_frame_usage: i32,
    /// Number of parameters this function takes.
    num_parameters: usize,
    /// Number of values this function returns.
    num_return_values: usize,
}

impl FunctionLayout {
    /// Creates a new layout for a function, allocating slots for its parameters.
    pub fn new(function: &MirFunction) -> CodegenResult<Self> {
        let mut layout = Self {
            value_offsets: FxHashMap::default(),
            current_frame_usage: 0,
            num_parameters: function.parameters.len(),
            // TODO: Support multiple return values properly from MIR type info.
            num_return_values: if function.return_value.is_some() {
                1
            } else {
                0
            },
        };

        layout.allocate_parameters(function)?;

        Ok(layout)
    }

    /// Allocates memory slots for function parameters at negative offsets according to the
    /// calling convention.
    fn allocate_parameters(&mut self, function: &MirFunction) -> CodegenResult<()> {
        let m = self.num_parameters as i32;
        let k = self.num_return_values as i32;

        for (i, &param_value_id) in function.parameters.iter().enumerate() {
            // According to the convention, arg `i` is at `[fp - M - K - 2 + i]`.
            let offset = (i as i32) - m - k - 2;
            self.value_offsets.insert(param_value_id, offset);
        }

        Ok(())
    }

    /// Allocates a new local variable at the next available positive offset from `fp`.
    pub fn allocate_local(&mut self, value_id: ValueId, size: usize) -> CodegenResult<i32> {
        // If this value is a parameter, it's already allocated. Return its offset.
        if let Some(&offset) = self.value_offsets.get(&value_id) {
            return Ok(offset);
        }

        let offset = self.current_frame_usage;
        self.value_offsets.insert(value_id, offset);
        self.current_frame_usage += size as i32;

        Ok(offset)
    }

    /// Manually maps a `ValueId` to a specific offset. Used by the caller to map
    /// return value destinations.
    pub fn map_value(&mut self, value_id: ValueId, offset: i32) {
        self.value_offsets.insert(value_id, offset);
        // Update current_frame_usage if this offset extends beyond it
        if offset >= self.current_frame_usage {
            self.current_frame_usage = offset + 1;
        }
    }

    /// Reserves `size` slots on the stack and returns the starting offset.
    /// Does not associate the space with a `ValueId`.
    pub const fn reserve_stack(&mut self, size: usize) -> i32 {
        let offset = self.current_frame_usage;
        self.current_frame_usage += size as i32;
        offset
    }

    /// Gets the fp-relative offset for a `ValueId`.
    pub fn get_offset(&self, value_id: ValueId) -> CodegenResult<i32> {
        self.value_offsets.get(&value_id).copied().ok_or_else(|| {
            CodegenError::LayoutError(format!("No offset found for value {value_id:?}"))
        })
    }

    /// Gets the current frame usage (the number of words used by local variables).
    pub const fn current_frame_usage(&self) -> i32 {
        self.current_frame_usage
    }

    /// Gets the number of return values for the function.
    pub const fn num_return_values(&self) -> usize {
        self.num_return_values
    }

    /// Gets all allocated value offsets (for debugging).
    pub const fn all_offsets(&self) -> &FxHashMap<ValueId, i32> {
        &self.value_offsets
    }
}
