# Task 008: Array Memory Path Preservation

**Priority**: MEDIUM  
**Dependencies**: Task 002 (aggregate lowering implementation)

## Why

Arrays need to stay on the memory path for now while aggregates (tuples and
structs) move to value-based operations. This scoping decision allows us to land
the aggregate-first MIR refactoring without "boiling the ocean" by tackling
array semantics simultaneously.

Key reasons for keeping arrays on memory path:

1. **Memory Semantics Complexity**: Arrays involve more complex memory semantics
   than simple aggregates, including bounds checking, potential dynamic sizing,
   and element addressing patterns that require careful handling.

2. **Address-of Operations**: Arrays frequently need to provide element
   addresses via `AddressOf` operations, which naturally require memory-based
   representation.

3. **Existing Test Coverage**: The current memory-based array implementation has
   extensive test coverage that we want to preserve during the aggregate
   transition.

4. **Incremental Migration**: By scoping arrays out of the initial aggregate
   refactoring, we can validate the new aggregate system thoroughly before
   tackling array complexities.

## What

Implement clear scoping to ensure arrays and explicit `AddressOf` operations
continue using the existing memory-based lowering path while the new aggregate
instructions (`MakeTuple`, `ExtractTuple`, `MakeStruct`, `ExtractField`) are
applied only to struct/tuple values.

This task involves:

1. **Guarding New Aggregate Code**: Add type checks to ensure new aggregate
   instructions only apply to struct/tuple types
2. **Preserving Array Lowering**: Maintain existing `get_element_ptr` +
   `load`/`store` patterns for array operations
3. **Address-of Handling**: Ensure `AddressOf` expressions continue using
   memory-based lowering
4. **Testing**: Verify all existing array functionality remains unchanged

## How

### Implementation Steps

#### 1. Guard New Aggregate Code

**File**: `crates/compiler/mir/src/lowering/expr.rs`

Add type guards in the lowering functions to ensure new aggregate operations
only apply to appropriate types:

```rust
// In lower_member_access
match base_type {
    MirType::Struct(_) => {
        // Use new ExtractField instruction
        let dest = self.value_id_gen.next();
        self.builder.extract_field(dest, base_value, field_name, field_type);
        Value::Operand(dest)
    },
    MirType::Array(_) => {
        // Keep existing memory-based path
        self.lower_lvalue_expression(base)
            .and_then(|addr| self.load_field(addr, field_name))
    },
    _ => // handle other types
}

// In lower_tuple_index
match tuple_type {
    MirType::Tuple(_) => {
        // Use new ExtractTuple instruction
        let dest = self.value_id_gen.next();
        self.builder.extract_tuple_element(dest, tuple_value, index, element_type);
        Value::Operand(dest)
    },
    MirType::Array(_) => {
        // Keep existing get_element_ptr + load path
        self.lower_lvalue_expression(tuple)
            .and_then(|addr| self.load_array_element(addr, index))
    },
    _ => // handle other types
}
```

#### 2. Maintain Array Lowering Paths

**File**: `crates/compiler/mir/src/lowering/expr.rs`

Ensure array operations continue using the current memory-based approach:

```rust
// In lower_array_literal (if exists)
fn lower_array_literal(&mut self, elements: &[Expression]) -> Result<Value, LoweringError> {
    // Continue using frame_alloc + store pattern for arrays
    let array_type = self.get_expression_type(array_expr)?;
    let alloc_addr = self.builder.frame_alloc(array_type);

    for (index, element) in elements.iter().enumerate() {
        let element_value = self.lower_expression(element)?;
        let element_addr = self.builder.get_element_ptr(alloc_addr, index);
        self.builder.store(element_addr, element_value);
    }

    Value::Address(alloc_addr)
}

// In lower_array_index
fn lower_array_index(&mut self, array: &Expression, index: &Expression) -> Result<Value, LoweringError> {
    // Keep using get_element_ptr + load for arrays
    let array_addr = self.lower_lvalue_expression(array)?;
    let index_value = self.lower_expression(index)?;
    let element_addr = self.builder.get_element_ptr(array_addr, index_value);
    self.builder.load(element_addr)
}
```

#### 3. Testing Existing Array Functionality

**File**: `crates/compiler/mir/tests/array_tests.rs` (if not exists, create)

Add comprehensive tests to ensure arrays continue working:

```rust
#[test]
fn test_array_literal_memory_allocation() {
    let source = r#"
        fn test_arrays() -> felt {
            let arr = [1, 2, 3];
            return arr[1];
        }
    "#;

    let mir = compile_to_mir(source);

    // Verify array uses memory operations
    assert_contains_instruction(&mir, "frame_alloc");
    assert_contains_instruction(&mir, "get_element_ptr");
    assert_contains_instruction(&mir, "load");

    // Verify no aggregate instructions are used
    assert_not_contains_instruction(&mir, "make_tuple");
    assert_not_contains_instruction(&mir, "extract_tuple");
}

#[test]
fn test_array_address_of() {
    let source = r#"
        fn test_array_address() -> felt* {
            let arr = [1, 2, 3];
            return &arr[0];
        }
    "#;

    let mir = compile_to_mir(source);

    // Verify address-of uses memory path
    assert_contains_instruction(&mir, "get_element_ptr");
    // Return should be an address, not a value
    assert_return_type_is_address(&mir);
}
```

#### 4. Documentation Updates

**File**: `crates/compiler/mir/README.md` (update existing or create)

Document the scoping decision:

```markdown
## Aggregate vs Array Handling

The MIR currently uses two different approaches for composite types:

### Value-Based Aggregates (Structs/Tuples)

- Use `MakeTuple`, `ExtractTuple`, `MakeStruct`, `ExtractField` instructions
- Represented as SSA values, not memory allocations
- Optimized through value-based passes

### Memory-Based Arrays

- Continue using `frame_alloc`, `get_element_ptr`, `load`, `store` instructions
- Represented as memory addresses
- Support for `AddressOf` operations on elements
- Preserved for compatibility and address semantics

This scoping allows incremental migration while maintaining correctness.
```

#### 5. Validation Updates

**File**: `crates/compiler/mir/src/passes/validation.rs`

Add validation to ensure proper scoping:

```rust
fn validate_extract_operations(&self, instr: &Instruction) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match &instr.kind {
        InstructionKind::ExtractTuple { tuple, .. } => {
            if let Some(tuple_type) = self.get_value_type(tuple) {
                if matches!(tuple_type, MirType::Array(_)) {
                    errors.push(ValidationError::new(
                        "ExtractTuple used on array type - arrays should use memory operations"
                    ));
                }
            }
        },
        InstructionKind::ExtractField { struct_val, .. } => {
            if let Some(struct_type) = self.get_value_type(struct_val) {
                if matches!(struct_type, MirType::Array(_)) {
                    errors.push(ValidationError::new(
                        "ExtractField used on array type - arrays should use memory operations"
                    ));
                }
            }
        },
        _ => {}
    }

    errors
}
```

### Testing Strategy

1. **Regression Tests**: Run all existing array tests to ensure no functionality
   breaks
2. **Type Boundary Tests**: Test edge cases where arrays and structs interact
3. **Address-of Tests**: Verify `AddressOf` operations work correctly with
   arrays
4. **Performance Tests**: Ensure array operations maintain current performance
   characteristics

### Success Criteria

- [ ] All existing array tests pass unchanged
- [ ] New aggregate instructions are never generated for array types
- [ ] Array operations continue using memory-based lowering (`get_element_ptr`,
      `load`, `store`)
- [ ] `AddressOf` operations on arrays work correctly
- [ ] Validation catches any improper mixing of aggregate/array operations
- [ ] Documentation clearly explains the scoping decision
- [ ] No performance regression in array operations

### Future Work

This task explicitly leaves array-to-value migration for a future iteration.
Once the aggregate-first approach is proven stable for structs and tuples,
arrays can be evaluated for similar treatment based on:

1. Performance benefits analysis
2. Complexity of address semantics handling
3. Backend capability requirements
4. Breaking change impact assessment

By keeping arrays on the memory path initially, we minimize risk while
validating the aggregate refactoring approach.
