# Task 007: High-Priority Aggregate Validation

## Why

As part of the MIR refactoring towards aggregate-first design (Issue #10 from
MIR_REPORT.md), new first-class aggregate instructions will be introduced
(`MakeTuple`, `ExtractTupleElement`, `MakeStruct`, `ExtractStructField`,
`InsertField`). These value-oriented operations require comprehensive validation
to ensure MIR correctness and catch errors early in the compilation pipeline.

The current validation infrastructure primarily focuses on memory-based
operations (load/store pointer validation, GEP usage). With the introduction of
aggregate-as-values operations, we need validation that:

1. **Prevents out-of-bounds access**: Tuple index operations must stay within
   tuple arity bounds
2. **Ensures field existence**: Struct field operations must reference valid
   field names
3. **Maintains type safety**: Aggregate operations must respect the type system
4. **Provides helpful diagnostics**: Clear error messages when validation fails

Without proper validation, malformed aggregate operations could propagate
through the compilation pipeline, leading to runtime errors or incorrect code
generation.

## What

This task extends the existing `Validation` pass in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs` to include
comprehensive checks for the new aggregate instructions:

### Validation Checks Required

**1. Tuple Operations**

- `ExtractTupleElement`: Verify index < tuple arity when statically determinable
- `MakeTuple`: Ensure all element types are provided and valid
- Type consistency: Extract operation result type matches tuple element type

**2. Struct Operations**

- `ExtractStructField`: Verify field name exists in struct definition
- `MakeStruct`: Ensure all required fields are provided, no duplicate fields
- `InsertField`: Verify field name exists and type matches
- Type consistency: Field operations respect struct field types

**3. Type-Aware Validation**

- Use `function.get_value_type_or_unknown()` for type lookup
- When type information is unavailable, emit warnings rather than errors
- Graceful degradation matching existing validation patterns

**4. Error Reporting**

- Follow existing `RUST_LOG`-gated error reporting pattern
- Descriptive messages including block ID, instruction index, and context
- Differentiate between errors (definite violations) and warnings (unknown
  types)

### Integration Points

- Extend `validate_aggregate_operations()` method in the `Validation` impl
- Add validation calls to the main `run()` method
- Maintain compatibility with both SSA and post-SSA validation modes
- Ensure aggregate validation works alongside existing pointer/memory validation

## How

### 1. Extend validation.rs Structure

Add new validation method to the existing `Validation` impl:

```rust
impl Validation {
    /// Validate aggregate operations (tuples and structs)
    fn validate_aggregate_operations(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    InstructionKind::ExtractTupleElement { tuple, index, .. } => {
                        self.validate_tuple_extract(function, block_id, instr_idx, tuple, *index);
                    }
                    InstructionKind::ExtractStructField { struct_val, field_name, .. } => {
                        self.validate_struct_field_extract(function, block_id, instr_idx, struct_val, field_name);
                    }
                    InstructionKind::MakeStruct { fields, struct_ty, .. } => {
                        self.validate_struct_construction(function, block_id, instr_idx, fields, struct_ty);
                    }
                    InstructionKind::InsertField { struct_val, field_name, .. } => {
                        self.validate_struct_field_insert(function, block_id, instr_idx, struct_val, field_name);
                    }
                    _ => {} // Other instructions handled by existing validation
                }
            }
        }
    }
}
```

### 2. Add Type-Aware Validation Logic

Implement specific validation methods following the existing pattern:

```rust
fn validate_tuple_extract(&self, function: &MirFunction, block_id: BasicBlockId,
                         instr_idx: usize, tuple: &Value, index: usize) {
    if let Value::Operand(tuple_id) = tuple {
        if let Some(tuple_type) = function.get_value_type(*tuple_id) {
            if let MirType::Tuple(elements) = tuple_type {
                if index >= elements.len() {
                    if std::env::var("RUST_LOG").is_ok() {
                        eprintln!(
                            "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                             Tuple index {index} out of bounds for tuple with {} elements",
                            elements.len()
                        );
                    }
                }
            } else if !matches!(tuple_type, MirType::Unknown) {
                // Non-tuple type used in tuple operation
                if std::env::var("RUST_LOG").is_ok() {
                    eprintln!(
                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                         Extract tuple element from non-tuple type {tuple_type:?}"
                    );
                }
            }
        } else if std::env::var("RUST_LOG").is_ok() {
            eprintln!(
                "[WARN] Block {block_id:?}, instruction {instr_idx}: \
                 Cannot determine type for tuple extract validation"
            );
        }
    }
}

fn validate_struct_field_extract(&self, function: &MirFunction, block_id: BasicBlockId,
                                instr_idx: usize, struct_val: &Value, field_name: &str) {
    if let Value::Operand(struct_id) = struct_val {
        if let Some(struct_type) = function.get_value_type(*struct_id) {
            if let MirType::Struct { fields, .. } = struct_type {
                if !fields.iter().any(|(name, _)| name == field_name) {
                    if std::env::var("RUST_LOG").is_ok() {
                        eprintln!(
                            "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                             Field '{field_name}' does not exist in struct"
                        );
                    }
                }
            } else if !matches!(struct_type, MirType::Unknown) {
                if std::env::var("RUST_LOG").is_ok() {
                    eprintln!(
                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                         Extract field from non-struct type {struct_type:?}"
                    );
                }
            }
        }
    }
}
```

### 3. Error Message Improvements

Enhance error messages to provide actionable feedback:

- Include context about expected vs actual types
- Show available fields/indices when reporting missing ones
- Use consistent formatting with existing validation messages
- Maintain the `[ERROR]`/`[WARN]` prefixing convention

### 4. Integration with Existing Validation

Add the new validation call to the main `run()` method:

```rust
impl MirPass for Validation {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // ... existing validation calls ...
        self.validate_aggregate_operations(function);
        false // Validation doesn't modify the function
    }
}
```

### 5. Testing Integration

Create unit tests following the existing pattern in `validation_tests.rs`:

```rust
#[test]
fn test_tuple_bounds_validation() {
    let mut function = MirFunction::new("test_tuple_bounds".to_string());
    // Create instruction with out-of-bounds tuple access
    // Verify validation detects the error
}

#[test]
fn test_struct_field_validation() {
    let mut function = MirFunction::new("test_struct_field".to_string());
    // Create instruction accessing non-existent field
    // Verify validation detects the error
}
```

### Implementation Steps

1. **Phase 1**: Add basic aggregate validation structure without new instruction
   kinds
2. **Phase 2**: Wait for Task 001 completion (aggregate instructions
   implementation)
3. **Phase 3**: Implement validation logic for the specific instruction kinds
4. **Phase 4**: Add comprehensive test coverage
5. **Phase 5**: Integrate with pipeline and verify with existing tests

**Priority**: HIGH  
**Dependencies**: Task 001 (Critical - Pre-optimization Dead Store
Elimination)  
**Estimated Effort**: 2-3 days after Task 001 completion

This validation infrastructure will ensure the new aggregate-first MIR maintains
correctness while providing developers with clear diagnostic information when
issues arise.
