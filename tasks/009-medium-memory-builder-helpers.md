# Task: Extract Memory Access Patterns into Builder Helpers

## Priority

MEDIUM

## Why

The lowering code contains numerous instances of the "compute address then
load/store" pattern across multiple files, leading to code duplication and
inconsistency. This pattern appears in:

1. **Member access operations** - Computing field offsets and loading/storing
   struct fields
2. **Index access operations** - Computing array element addresses and accessing
   elements
3. **Tuple operations** - Computing tuple element offsets for both access and
   destructuring
4. **Destructuring patterns** - Extracting tuple elements in let statements and
   assignments

This duplication leads to:

- **Code bloat** - The same 4-6 line sequences appear dozens of times
- **Inconsistent error handling** - Some places use comments, others don't
- **Maintenance burden** - Changes to address calculation logic must be repeated
  everywhere
- **Type safety issues** - Manual pointer type management is error-prone

## What

Extract common memory access patterns into reusable builder helper methods that
encapsulate the "address calculation + load/store" operations. The patterns
identified are:

### Pattern 1: Field Access (Load)

```rust
// Current pattern (repeated ~8 times):
let field_addr = self.state.mir_function.new_typed_value_id(MirType::pointer(field_type.clone()));
self.instr().add_instruction(
    Instruction::get_element_ptr(field_addr, object_addr, field_offset)
        .with_comment(format!("Get address of field '{}'", field_name))
);
let loaded_value = self.state.mir_function.new_typed_value_id(field_type.clone());
self.instr().load_with_comment(field_type, loaded_value, Value::operand(field_addr), comment);
```

### Pattern 2: Field Access (Store)

```rust
// Current pattern (repeated ~6 times):
let field_addr = self.state.mir_function.new_typed_value_id(MirType::pointer(field_type));
self.instr().add_instruction(
    Instruction::get_element_ptr(field_addr, base_addr, offset)
        .with_comment(format!("Get address of field '{}'", field_name))
);
self.instr().store(Value::operand(field_addr), value, field_type);
```

### Pattern 3: Tuple Element Access (Load)

```rust
// Current pattern (repeated ~5 times):
let elem_ptr = self.state.mir_function.new_typed_value_id(MirType::pointer(elem_type.clone()));
self.instr().add_instruction(
    Instruction::get_element_ptr(elem_ptr, tuple_addr, Value::integer(index as i32))
        .with_comment(format!("Get address of tuple element {}", index))
);
let elem_value = self.state.mir_function.new_typed_value_id(elem_type.clone());
self.instr().load_with_comment(elem_type, elem_value, Value::operand(elem_ptr), comment);
```

### Pattern 4: Address-only Access (LValue)

```rust
// Current pattern (repeated ~4 times):
let addr = self.state.mir_function.new_typed_value_id(MirType::pointer(target_type));
self.instr().add_instruction(
    Instruction::get_element_ptr(addr, base_addr, offset)
        .with_comment(comment)
);
```

## How

### Implementation Plan

#### 1. Add Helper Methods to MirBuilder

Add these methods to
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/builder.rs`:

```rust
impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Load a field from a struct/object
    /// Returns the ValueId of the loaded value
    pub fn load_field(
        &mut self,
        base_addr: Value,
        offset: Value,
        field_type: MirType,
        field_name: &str,
    ) -> ValueId {
        let field_addr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(field_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(field_addr, base_addr, offset)
                .with_comment(format!("Get address of field '{}'", field_name))
        );

        let loaded_value = self.state.mir_function
            .new_typed_value_id(field_type.clone());
        self.instr().load_with_comment(
            field_type,
            loaded_value,
            Value::operand(field_addr),
            format!("Load field '{}'", field_name)
        );
        loaded_value
    }

    /// Store a value to a struct field
    pub fn store_field(
        &mut self,
        base_addr: Value,
        offset: Value,
        value: Value,
        field_type: MirType,
        field_name: &str,
    ) {
        let field_addr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(field_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(field_addr, base_addr, offset)
                .with_comment(format!("Get address of field '{}'", field_name))
        );
        self.instr().store(Value::operand(field_addr), value, field_type);
    }

    /// Load a tuple element by index
    /// Returns the ValueId of the loaded value
    pub fn load_tuple_element(
        &mut self,
        tuple_addr: Value,
        index: usize,
        elem_type: MirType,
    ) -> ValueId {
        let elem_ptr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(elem_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(
                elem_ptr,
                tuple_addr,
                Value::integer(index as i32)
            ).with_comment(format!("Get address of tuple element {}", index))
        );

        let elem_value = self.state.mir_function
            .new_typed_value_id(elem_type.clone());
        self.instr().load_with_comment(
            elem_type,
            elem_value,
            Value::operand(elem_ptr),
            format!("Load tuple element {}", index)
        );
        elem_value
    }

    /// Store a value to a tuple element
    pub fn store_tuple_element(
        &mut self,
        tuple_addr: Value,
        index: usize,
        value: Value,
        elem_type: MirType,
    ) {
        let elem_ptr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(elem_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(
                elem_ptr,
                tuple_addr,
                Value::integer(index as i32)
            ).with_comment(format!("Get address of tuple element {}", index))
        );
        self.instr().store(Value::operand(elem_ptr), value, elem_type);
    }

    /// Get the address of a field/element (for lvalue expressions)
    /// Returns the ValueId of the address
    pub fn get_element_address(
        &mut self,
        base_addr: Value,
        offset: Value,
        target_type: MirType,
        comment: &str,
    ) -> ValueId {
        let addr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(target_type));
        self.instr().add_instruction(
            Instruction::get_element_ptr(addr, base_addr, offset)
                .with_comment(comment.to_string())
        );
        addr
    }
}
```

#### 2. Migration Strategy

**Phase 1**: Update `expr.rs` patterns

- Replace member access patterns in `lower_member_access()` and
  `lower_lvalue_expression()`
- Replace index access patterns in `lower_index_access()`
- Replace tuple patterns in `lower_tuple_index()` and tuple literal handling

**Phase 2**: Update `stmt.rs` patterns

- Replace destructuring patterns in `lower_pattern()`
- Replace return statement tuple handling patterns

**Phase 3**: Update `utils.rs` patterns

- Replace composite type copying patterns in `copy_composite_type()`

**Phase 4**: Clean up existing helpers

- Remove or refactor `get_element_ptr_auto()` if no longer needed
- Update `load_auto()` and `store_value()` if they can be simplified

#### 3. Specific Replacements

In `expr.rs`, line 452-518 (member access):

```rust
// Replace this:
let field_addr = self.state.mir_function.new_typed_value_id(MirType::pointer(field_type.clone()));
self.instr().add_instruction(
    Instruction::get_element_ptr(field_addr, object_addr, field_offset)
        .with_comment(format!("Get address of field '{}'", field.value()))
);
let loaded_value = self.state.mir_function.new_typed_value_id(field_type.clone());
self.instr().load_with_comment(field_type, loaded_value, Value::operand(field_addr), comment);

// With this:
let loaded_value = self.load_field(object_addr, field_offset, field_type, field.value());
```

Similar patterns exist in 15+ other locations across the three files.

## Testing

### Unit Tests

Add tests to
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/tests/builder_helpers.rs`:

```rust
#[test]
fn test_load_field_helper() {
    // Test that load_field generates correct instruction sequence
}

#[test]
fn test_store_field_helper() {
    // Test that store_field generates correct instruction sequence
}

#[test]
fn test_tuple_element_helpers() {
    // Test tuple load/store helpers
}

#[test]
fn test_get_element_address_helper() {
    // Test address-only helper for lvalues
}
```

### Integration Tests

- Verify existing semantic tests still pass after migration
- Add snapshot tests to ensure generated MIR is identical or improved
- Test error cases (invalid field names, out-of-bounds indices)

### Performance Tests

- Verify no regression in compilation time
- Measure instruction generation efficiency

## Impact

### Positive Impact

- **Reduced Code Size**: Remove ~200-300 lines of duplicated code
- **Better Maintainability**: Single location for address calculation logic
- **Improved Consistency**: Uniform error messages and comment format
- **Type Safety**: Centralized pointer type management
- **Easier Debugging**: Clearer call stack with named helper methods

### Risk Assessment

- **Low Risk**: Changes are purely internal refactoring
- **No Breaking Changes**: External API remains unchanged
- **Backward Compatible**: Generated MIR should be identical

### Code Quality Metrics

- Lines of code reduction: ~15-20%
- Cyclomatic complexity reduction in affected methods
- Improved code review efficiency for memory access patterns
- Better test coverage through focused helper tests

### Future Benefits

- Easier to add optimizations (e.g., strength reduction for constant offsets)
- Simpler to add debugging/tracing for memory operations
- Foundation for more sophisticated address calculation optimizations
