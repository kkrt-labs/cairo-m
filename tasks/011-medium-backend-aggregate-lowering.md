# Task 011: Backend Aggregate Lowering

**Priority:** MEDIUM  
**Dependencies:** Task 001 (requires aggregate instructions)

## Why

With the introduction of first-class aggregate instructions (`MakeTuple`,
`ExtractTuple`, `MakeStruct`, `ExtractField`) in the MIR, some backends may not
be able to directly consume these value-based aggregate operations. Different
code generation targets have varying levels of support for aggregate types:

- **Modern LLVM-style backends**: Can handle aggregates natively and benefit
  from the value-based representation
- **Register-based VMs**: May need aggregates lowered to individual scalar
  operations
- **Stack-based VMs**: May require memory-based representations for ABI
  compatibility
- **Legacy codegen**: May expect all aggregates to be represented as memory
  operations

Rather than forcing all backends to implement aggregate handling, we need an
optional late-stage pass that can convert aggregate instructions back to memory
operations when required. This maintains the benefits of the aggregate-first MIR
for optimization while providing backend compatibility through isolation of
target-specific concerns.

## What

Implement an optional `LowerAggregatesPass` that can convert value-based
aggregate instructions back to memory operations for backend compatibility. This
pass should:

1. **Convert aggregate values to memory**: Replace `MakeTuple`/`MakeStruct` with
   `FrameAlloc` + individual `Store` operations
2. **Convert aggregate access to loads**: Replace `ExtractTuple`/`ExtractField`
   with `GetElementPtr` + `Load` operations
3. **Handle ABI boundaries**: Ensure function parameters and return values
   follow backend-specific aggregate conventions
4. **Preserve optimization benefits**: Only run when explicitly enabled via
   backend configuration
5. **Support gradual migration**: Allow backends to opt into aggregate support
   incrementally

The pass should be **feature-flagged** and **disabled by default**, ensuring
that aggregate-capable backends continue to benefit from the optimized
representation while legacy backends can request the memory-based fallback.

## How

### 1. Create `lower_aggregates.rs` Pass

Create a new file: `crates/compiler/mir/src/passes/lower_aggregates.rs`

```rust
//! Late-stage aggregate lowering for backend compatibility

use crate::{
    passes::MirPass,
    BasicBlockId, InstructionKind, MirFunction, MirType, Value, ValueId
};
use std::collections::HashMap;

/// Pass that converts aggregate instructions to memory operations
/// for backends that cannot handle first-class aggregates
#[derive(Debug, Default)]
pub struct LowerAggregatesPass {
    /// Map from original aggregate values to their memory locations
    aggregate_allocas: HashMap<ValueId, ValueId>,
}

impl LowerAggregatesPass {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert MakeTuple/MakeStruct to memory allocation + stores
    fn lower_aggregate_creation(&mut self, function: &mut MirFunction, instruction: &InstructionKind) -> Option<Vec<InstructionKind>> {
        match instruction {
            InstructionKind::MakeTuple { dest, elements } => {
                // Replace with: %alloca = frame_alloc(tuple_type)
                // followed by: store_tuple_element(%alloca, i, element) for each element
                // Store alloca mapping for later extract operations
                Some(self.create_tuple_alloca_and_stores(*dest, elements, function))
            }
            InstructionKind::MakeStruct { dest, fields, struct_ty } => {
                // Replace with: %alloca = frame_alloc(struct_type)
                // followed by: store_field(%alloca, field_name, value) for each field
                Some(self.create_struct_alloca_and_stores(*dest, fields, struct_ty, function))
            }
            _ => None,
        }
    }

    /// Convert ExtractTuple/ExtractField to GEP + load operations
    fn lower_aggregate_access(&mut self, instruction: &InstructionKind) -> Option<Vec<InstructionKind>> {
        match instruction {
            InstructionKind::ExtractTuple { dest, tuple, index, .. } => {
                // Find the alloca for this tuple value
                if let Some(&alloca_value) = self.aggregate_allocas.get(&Self::extract_value_id(tuple)) {
                    // Replace with: %gep = get_element_ptr(%alloca, index)
                    //               %dest = load(%gep)
                    Some(self.create_tuple_gep_and_load(*dest, alloca_value, *index))
                } else {
                    None // Value not from an aggregate creation we can lower
                }
            }
            InstructionKind::ExtractField { dest, struct_val, field_name, .. } => {
                if let Some(&alloca_value) = self.aggregate_allocas.get(&Self::extract_value_id(struct_val)) {
                    // Replace with: %gep = get_element_ptr(%alloca, field_name)
                    //               %dest = load(%gep)
                    Some(self.create_struct_gep_and_load(*dest, alloca_value, field_name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // Helper methods for instruction creation...
}

impl MirPass for LowerAggregatesPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Process each basic block
        for (block_id, block) in function.basic_blocks.iter_enumerated_mut() {
            let mut new_instructions = Vec::new();

            for instruction in &block.instructions {
                // Try to lower aggregate creation first
                if let Some(replacement) = self.lower_aggregate_creation(function, &instruction.kind) {
                    new_instructions.extend(replacement.into_iter().map(|kind| Instruction { kind }));
                    modified = true;
                }
                // Then try to lower aggregate access
                else if let Some(replacement) = self.lower_aggregate_access(&instruction.kind) {
                    new_instructions.extend(replacement.into_iter().map(|kind| Instruction { kind }));
                    modified = true;
                } else {
                    new_instructions.push(instruction.clone());
                }
            }

            block.instructions = new_instructions;
        }

        modified
    }

    fn name(&self) -> &'static str {
        "LowerAggregates"
    }
}
```

### 2. Feature Flag Configuration

Update `crates/compiler/mir/src/backend.rs` to support aggregate lowering
configuration:

```rust
impl BackendConfig {
    /// Whether this backend requires aggregate lowering
    pub fn requires_aggregate_lowering(&self) -> bool {
        self.options.get("lower_aggregates")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }

    /// Enable aggregate lowering for this backend
    pub fn with_aggregate_lowering(mut self) -> Self {
        self.options.insert("lower_aggregates".to_string(), "true".to_string());
        self
    }
}
```

Add environment variable support in `crates/compiler/mir/src/pipeline.rs`:

```rust
impl PipelineConfig {
    /// Create config with environment variable overrides
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Check CAIROM_LOWER_AGGREGATES environment variable
        if let Ok(val) = std::env::var("CAIROM_LOWER_AGGREGATES") {
            if val == "1" || val.to_lowercase() == "true" {
                config.backend_config = config.backend_config.with_aggregate_lowering();
            }
        }

        config
    }
}
```

### 3. ABI Boundary Handling

Extend the pass to handle function boundaries:

```rust
impl LowerAggregatesPass {
    /// Handle aggregate function parameters
    fn lower_function_parameters(&mut self, function: &mut MirFunction) {
        // Convert aggregate parameters to pointer parameters
        // Update function signature to receive addresses instead of values
        // Insert loads at function entry to maintain SSA semantics
    }

    /// Handle aggregate return values
    fn lower_function_returns(&mut self, function: &mut MirFunction) {
        // Convert aggregate returns to out-parameters or memory copies
        // Update return terminators to work with memory representations
    }
}
```

### 4. Backend Compatibility Options

Update `PassManager::standard_pipeline()` to conditionally include aggregate
lowering:

```rust
impl PassManager {
    /// Create pipeline with backend-specific configuration
    pub fn for_backend(backend_config: &BackendConfig) -> Self {
        let mut pipeline = Self::standard_pipeline();

        // Add aggregate lowering if backend requires it
        if backend_config.requires_aggregate_lowering() {
            pipeline = pipeline.add_pass(LowerAggregatesPass::new());
        }

        pipeline
    }
}
```

Add integration in the compilation pipeline:

```rust
impl<B: Backend> CompilationPipeline<B> {
    /// Apply backend-specific transformations
    fn apply_backend_lowering(
        &mut self,
        module: &mut MirModule,
        config: &PipelineConfig
    ) -> BackendResult<()> {
        if config.backend_config.requires_aggregate_lowering() {
            let mut lowering_pass = LowerAggregatesPass::new();

            for function in module.functions_mut() {
                lowering_pass.run(function);
            }
        }

        Ok(())
    }
}
```

### 5. Integration Points

**Update backend trait to declare aggregate support:**

```rust
impl BackendInfo {
    /// Whether this backend natively supports aggregate instructions
    pub fn supports_aggregates(&self) -> bool {
        self.optional_mir_features.contains(&"first_class_aggregates".to_string())
    }
}
```

**Add pass to module exports:**

```rust
// In crates/compiler/mir/src/passes.rs
pub mod lower_aggregates;
pub use lower_aggregates::LowerAggregatesPass;

// In crates/compiler/mir/src/lib.rs
pub use passes::LowerAggregatesPass;
```

**Testing strategy:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_lowering() {
        // Test that MakeTuple + ExtractTuple becomes alloca + store + gep + load
    }

    #[test]
    fn test_struct_lowering() {
        // Test that MakeStruct + ExtractField becomes alloca + store + gep + load
    }

    #[test]
    fn test_feature_flag_disabled() {
        // Test that aggregates pass through unchanged when flag is off
    }
}
```

## Implementation Notes

1. **Gradual Migration**: This approach allows backends to migrate to aggregate
   support incrementally - they can start with aggregate lowering enabled and
   gradually add native support for specific aggregate operations.

2. **Performance Considerations**: The pass should only run when explicitly
   requested, ensuring that aggregate-capable backends don't pay the cost of
   memory conversion.

3. **ABI Compatibility**: Special attention should be paid to function call
   boundaries, where aggregate passing conventions may differ between backends.

4. **Debugging Support**: The lowered code should maintain debugging information
   and source location mappings where possible.

5. **Integration Testing**: Test the pass with both aggregate-capable and legacy
   backends to ensure compatibility and correctness.

This design isolates backend-specific aggregate handling concerns to a single,
optional pass while preserving the optimization benefits of the aggregate-first
MIR design for modern backends.
