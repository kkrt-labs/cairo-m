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
//! A function with `M` argument slots that returns `K` return value slots has the following layout:
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

/// Represents the memory layout of a value in the stack frame.
#[derive(Debug, Clone)]
pub enum ValueLayout {
    /// A single-slot value (felt, bool, ptr) at a specific offset.
    Slot { offset: i32 },
    /// A multi-slot value (like u32) with a base offset and size.
    MultiSlot { offset: i32, size: usize },
    /// A compile-time constant that doesn't need a stack slot.
    Constant(cairo_m_compiler_mir::Literal),
    /// A value that was optimized out.
    OptimizedOut,
}

/// Maps every ValueId in a function to its fp-relative memory offset.
#[derive(Debug, Clone)]
pub struct FunctionLayout {
    /// Maps ValueId to its memory layout.
    pub value_layouts: FxHashMap<ValueId, ValueLayout>,
    /// The total frame size needed for this function.
    pub frame_size: usize,
    /// Number of parameters this function takes.
    pub num_parameters: usize,
    /// Number of values this function returns.
    num_return_values: usize,
    /// Total number of slots required for return values (accounting for multi-slot types).
    num_return_slots: usize,
}

impl FunctionLayout {
    /// Creates a new layout for a function, allocating slots for its parameters.
    pub fn new(function: &MirFunction) -> CodegenResult<Self> {
        let mut layout = Self {
            value_layouts: FxHashMap::default(),
            frame_size: 0,
            num_parameters: function.parameters.len(),
            num_return_values: function.return_values.len(),
            num_return_slots: 0, // Will be calculated in allocate_parameters_with_sizes
        };

        // Phase 1: Calculate parameter layout with proper multi-slot support
        layout.allocate_parameters_with_sizes(function)?;

        // Phase 2: Allocate locals and temporaries
        layout.allocate_locals_and_temporaries(function)?;

        Ok(layout)
    }

    /// Allocates memory slots for function parameters with proper size handling.
    fn allocate_parameters_with_sizes(&mut self, function: &MirFunction) -> CodegenResult<()> {
        // Calculate total slots needed for parameters and return values
        let mut m_slots = 0;
        for &param_id in &function.parameters {
            let ty = function.value_types.get(&param_id).ok_or_else(|| {
                CodegenError::LayoutError(format!("No type found for parameter {param_id:?}"))
            })?;
            m_slots += ty.size_units();
        }

        let mut k_slots = 0;
        for &return_id in &function.return_values {
            // Try to get the type, but if it's not available, assume single-slot
            // This can happen when the MIR doesn't fully populate value_types
            let ty = function.value_types.get(&return_id).ok_or_else(|| {
                CodegenError::LayoutError(format!("No type found for return value {return_id:?}"))
            })?;
            let size = ty.size_units();
            k_slots += size;
        }

        // Store the calculated k_slots for later use
        self.num_return_slots = k_slots;

        // Now allocate parameters with correct offsets
        let mut cumulative_param_size = 0;
        for &param_id in &function.parameters {
            let ty = function.value_types.get(&param_id).ok_or_else(|| {
                CodegenError::LayoutError(format!("No type found for parameter {param_id:?}"))
            })?;
            let size = ty.size_units();

            // Calculate the offset using the formula from Issue 2
            let offset = -(m_slots as i32) - (k_slots as i32) - 2 + cumulative_param_size as i32;

            if size == 1 {
                self.value_layouts
                    .insert(param_id, ValueLayout::Slot { offset });
            } else {
                self.value_layouts
                    .insert(param_id, ValueLayout::MultiSlot { offset, size });
            }

            cumulative_param_size += size;
        }

        Ok(())
    }

    /// Allocates all locals and temporaries by walking through the function's basic blocks.
    fn allocate_locals_and_temporaries(&mut self, function: &MirFunction) -> CodegenResult<()> {
        use cairo_m_compiler_mir::{InstructionKind, Literal, Value};

        let mut current_offset = 0;

        // Walk through all basic blocks and instructions
        for block in function.basic_blocks.iter() {
            for instruction in &block.instructions {
                // Handle special memory instructions
                match &instruction.kind {
                    InstructionKind::Call { dests, .. } => {
                        // Allocate space for call return values
                        for dest_id in dests {
                            if self.value_layouts.contains_key(dest_id) {
                                continue;
                            }

                            // Get the type and size for this return value
                            let ty = function.value_types.get(dest_id).ok_or_else(|| {
                                CodegenError::LayoutError(format!(
                                    "No type found for call return value {dest_id:?}"
                                ))
                            })?;
                            let size = ty.size_units();

                            // Allocate space for the return value
                            if size == 1 {
                                self.value_layouts.insert(
                                    *dest_id,
                                    ValueLayout::Slot {
                                        offset: current_offset as i32,
                                    },
                                );
                            } else {
                                self.value_layouts.insert(
                                    *dest_id,
                                    ValueLayout::MultiSlot {
                                        offset: current_offset as i32,
                                        size,
                                    },
                                );
                            }

                            current_offset += size;
                        }
                    }
                    InstructionKind::StackAlloc { dest, size } => {
                        // Skip if already allocated
                        if self.value_layouts.contains_key(dest) {
                            continue;
                        }

                        // Allocate a block of memory
                        let offset = current_offset as i32;
                        self.value_layouts.insert(
                            *dest,
                            ValueLayout::MultiSlot {
                                offset,
                                size: *size,
                            },
                        );
                        current_offset += size;
                    }
                    InstructionKind::GetElementPtr { dest, base, offset } => {
                        // Skip if already allocated
                        if self.value_layouts.contains_key(dest) {
                            continue;
                        }

                        // Look up the base layout
                        let base_offset = match base {
                            Value::Operand(base_id) => match self.value_layouts.get(base_id) {
                                Some(ValueLayout::Slot { offset }) => *offset,
                                Some(ValueLayout::MultiSlot { offset, .. }) => *offset,
                                _ => {
                                    return Err(CodegenError::LayoutError(format!(
                                            "Base value {base_id:?} for getelementptr has no memory layout"
                                        )));
                                }
                            },
                            _ => {
                                return Err(CodegenError::LayoutError(format!(
                                    "getelementptr base must be an operand, got {base:?}"
                                )));
                            }
                        };

                        // Evaluate the offset
                        let offset_value = match offset {
                            Value::Literal(Literal::Integer(n)) => *n,
                            _ => {
                                // For now, we only support literal offsets
                                return Err(CodegenError::LayoutError(format!(
                                    "getelementptr offset must be a literal integer, got {offset:?}"
                                )));
                            }
                        };

                        // Calculate the final offset
                        let final_offset = base_offset + offset_value;

                        // Store this as a pointer to the calculated offset
                        self.value_layouts.insert(
                            *dest,
                            ValueLayout::Slot {
                                offset: final_offset,
                            },
                        );
                    }
                    _ => {
                        // For all other instructions, process destinations normally
                        for dest_id in instruction.destinations() {
                            // Skip if already allocated (e.g., parameters)
                            if self.value_layouts.contains_key(&dest_id) {
                                continue;
                            }

                            // Get the type and size for this value
                            let ty = function.value_types.get(&dest_id).ok_or_else(|| {
                                CodegenError::LayoutError(format!(
                                    "No type found for value {dest_id:?}"
                                ))
                            })?;
                            let size = ty.size_units();

                            // Create appropriate layout based on size
                            if size == 1 {
                                self.value_layouts.insert(
                                    dest_id,
                                    ValueLayout::Slot {
                                        offset: current_offset as i32,
                                    },
                                );
                            } else {
                                self.value_layouts.insert(
                                    dest_id,
                                    ValueLayout::MultiSlot {
                                        offset: current_offset as i32,
                                        size,
                                    },
                                );
                            }

                            current_offset += size;
                        }
                    }
                }
            }
        }

        // Set the final frame size
        self.frame_size = current_offset;

        Ok(())
    }

    /// Allocates a new local variable at the next available positive offset from `fp`.
    pub fn allocate_local(&mut self, value_id: ValueId, size: usize) -> CodegenResult<i32> {
        // If this value is already allocated, return its offset.
        if let Some(layout) = self.value_layouts.get(&value_id) {
            return match layout {
                ValueLayout::Slot { offset } | ValueLayout::MultiSlot { offset, .. } => Ok(*offset),
                _ => Err(CodegenError::LayoutError(format!(
                    "Cannot get offset for non-memory value {value_id:?}"
                ))),
            };
        }

        let offset = self.frame_size as i32;
        if size == 1 {
            self.value_layouts
                .insert(value_id, ValueLayout::Slot { offset });
        } else {
            self.value_layouts
                .insert(value_id, ValueLayout::MultiSlot { offset, size });
        }
        self.frame_size += size;

        Ok(offset)
    }

    /// Manually maps a `ValueId` to a specific offset. Used by the caller to map
    /// return value destinations.
    pub fn map_value(&mut self, value_id: ValueId, offset: i32) {
        // For now, assume mapped values are single-slot
        self.value_layouts
            .insert(value_id, ValueLayout::Slot { offset });
        // Update frame_size if this offset extends beyond it
        if offset >= self.frame_size as i32 {
            self.frame_size = (offset + 1) as usize;
        }
    }

    /// Reserves `size` slots on the stack and returns the starting offset.
    /// Does not associate the space with a `ValueId`.
    pub const fn reserve_stack(&mut self, size: usize) -> i32 {
        let offset = self.frame_size as i32;
        self.frame_size += size;
        offset
    }

    /// Gets the fp-relative offset for a `ValueId`.
    pub fn get_offset(&self, value_id: ValueId) -> CodegenResult<i32> {
        match self.value_layouts.get(&value_id) {
            Some(ValueLayout::Slot { offset }) | Some(ValueLayout::MultiSlot { offset, .. }) => {
                Ok(*offset)
            }
            Some(ValueLayout::Constant(_)) | Some(ValueLayout::OptimizedOut) => Err(
                CodegenError::LayoutError(format!("Value {value_id:?} has no memory offset")),
            ),
            None => Err(CodegenError::LayoutError(format!(
                "No layout found for value {value_id:?}"
            ))),
        }
    }

    /// Gets the current frame usage (the number of words used by local variables).
    pub const fn current_frame_usage(&self) -> i32 {
        self.frame_size as i32
    }

    /// Gets the number of return values for the function.
    pub const fn num_return_values(&self) -> usize {
        self.num_return_values
    }

    /// Gets the total number of return slots for the function (accounting for multi-slot types).
    pub const fn num_return_slots(&self) -> usize {
        self.num_return_slots
    }

    /// Gets all allocated value layouts (for debugging).
    pub const fn all_layouts(&self) -> &FxHashMap<ValueId, ValueLayout> {
        &self.value_layouts
    }

    /// Gets the size of a value in slots.
    pub fn get_value_size(&self, value_id: ValueId) -> usize {
        match self.value_layouts.get(&value_id) {
            Some(ValueLayout::Slot { .. }) => 1,
            Some(ValueLayout::MultiSlot { size, .. }) => *size,
            _ => 1, // Default to single slot
        }
    }

    /// Gets the value layout for a specific ValueId.
    pub fn get_layout(&self, value_id: ValueId) -> CodegenResult<&ValueLayout> {
        self.value_layouts.get(&value_id).ok_or_else(|| {
            CodegenError::LayoutError(format!("No layout found for value {value_id:?}"))
        })
    }

    /// Gets the current top offset (highest allocated offset in the frame).
    /// This is the last offset that was allocated, or -1 if nothing is allocated yet.
    pub const fn current_top_offset(&self) -> i32 {
        if self.frame_size == 0 {
            -1
        } else {
            self.frame_size as i32 - 1
        }
    }

    /// Checks if a value with the given size is stored contiguously starting at the expected offset.
    /// For single-slot values, this just checks if the value is at the expected offset.
    /// For multi-slot values, this checks that all slots are contiguous.
    pub fn is_contiguous(
        &self,
        value_id: ValueId,
        expected_offset: i32,
        expected_size: usize,
    ) -> bool {
        match self.value_layouts.get(&value_id) {
            Some(ValueLayout::Slot { offset }) => expected_size == 1 && *offset == expected_offset,
            Some(ValueLayout::MultiSlot { offset, size }) => {
                *size == expected_size && *offset == expected_offset
            }
            _ => false,
        }
    }
}

#[cfg(test)]
impl FunctionLayout {
    /// Creates a new empty layout for testing.
    pub fn new_for_test() -> Self {
        Self {
            value_layouts: FxHashMap::default(),
            frame_size: 0,
            num_parameters: 0,
            num_return_values: 0,
            num_return_slots: 0,
        }
    }

    /// Allocates a value with the given size for testing.
    pub fn allocate_value(&mut self, value_id: ValueId, size: usize) -> CodegenResult<i32> {
        self.allocate_local(value_id, size)
    }
}
