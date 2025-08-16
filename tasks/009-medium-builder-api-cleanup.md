# Task: Builder API Cleanup - Deprecate Memory-Based Helpers

## Priority

MEDIUM

## Dependencies

Task 002 (SSA destruction parallel copy) - needs new aggregate lowering

## Why

The current MIR builder API contains memory-based helper methods (`load_field`,
`store_field`, `load_tuple_element`, `store_tuple_element`) that encourage the
old memory-first approach. With the ongoing migration to value-based aggregate
operations, these helpers:

1. **Promote anti-patterns** - They encourage developers to think in terms of
   memory operations even when value-based operations would be more appropriate
2. **Create inconsistency** - New code using these helpers will generate
   memory-heavy MIR while new aggregate instructions produce clean value-based
   MIR
3. **Hinder optimization** - Memory-based operations require expensive SROA and
   mem2reg passes to convert back to values
4. **Complicate migration** - Having both old and new APIs available makes it
   unclear which approach to use
5. **Increase maintenance burden** - Supporting both memory-based and
   value-based APIs doubles the testing and maintenance surface

The goal is to guide all new development toward the new aggregate-first approach
by deprecating the memory-based helpers and providing clear migration paths.

## What

Deprecate the following memory-based builder helper methods in favor of new
value-based aggregate operations:

### Methods to Deprecate

1. **`MirBuilder::load_field()`**
   - Current: Generates `get_element_ptr` + `load` sequence
   - Replace with: Direct use of `ExtractField` instruction

2. **`MirBuilder::store_field()`**
   - Current: Generates `get_element_ptr` + `store` sequence
   - Replace with: `InsertField` instruction for value-based field updates

3. **`MirBuilder::load_tuple_element()`**
   - Current: Generates `get_element_ptr` + `load` sequence
   - Replace with: Direct use of `ExtractTuple` instruction

4. **`MirBuilder::store_tuple_element()`**
   - Current: Generates `get_element_ptr` + `store` sequence
   - Replace with: Value-based tuple reconstruction with `MakeTuple`

5. **Related memory-pattern helpers** that encourage address-first thinking:
   - `get_element_address()` for aggregate members
   - Helper methods that automatically create stack allocations for aggregates

### Deprecation Strategy

- **Phase 1**: Mark as deprecated with clear documentation
- **Phase 2**: Update all internal usage sites to use new aggregate instructions
- **Phase 3**: Add compiler warnings for deprecated method usage
- **Phase 4**: Remove deprecated methods entirely (future task)

## How

### 1. Mark Helpers as Deprecated

Add deprecation warnings and documentation to
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/builder.rs`:

````rust
impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Load a field from a struct/object
    ///
    /// # Deprecated
    ///
    /// This method is deprecated in favor of value-based aggregate operations.
    /// Use `ExtractField` instruction directly instead:
    ///
    /// ```rust
    /// // Old approach (deprecated):
    /// let value = builder.load_field(addr, offset, field_type, "field_name");
    ///
    /// // New approach (preferred):
    /// let value_id = self.state.mir_function.new_typed_value_id(field_type);
    /// let extract = Instruction::extract_struct_field(
    ///     value_id,
    ///     struct_value,
    ///     "field_name".to_string(),
    ///     field_type
    /// );
    /// self.instr().add_instruction(extract);
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Use ExtractField instruction for value-based field access instead"
    )]
    pub fn load_field(
        &mut self,
        base_addr: Value,
        offset: Value,
        field_type: MirType,
        field_name: &str,
    ) -> ValueId {
        // Implementation remains for backward compatibility
        // but logs deprecation warning
        eprintln!("WARNING: load_field() is deprecated. Use ExtractField instruction instead.");

        let field_addr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(field_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(field_addr, base_addr, offset)
                .with_comment(format!("DEPRECATED: Get address of field '{}'", field_name))
        );

        let loaded_value = self.state.mir_function
            .new_typed_value_id(field_type.clone());
        self.instr().load_with_comment(
            field_type,
            loaded_value,
            Value::operand(field_addr),
            format!("DEPRECATED: Load field '{}'", field_name)
        );
        loaded_value
    }

    /// Store a value to a struct field
    ///
    /// # Deprecated
    ///
    /// This method is deprecated in favor of value-based aggregate operations.
    /// Use `InsertField` instruction instead:
    ///
    /// ```rust
    /// // Old approach (deprecated):
    /// builder.store_field(addr, offset, value, field_type, "field_name");
    ///
    /// // New approach (preferred):
    /// let new_struct_id = self.state.mir_function.new_typed_value_id(struct_type);
    /// let insert = Instruction::insert_struct_field(
    ///     new_struct_id,
    ///     old_struct_value,
    ///     "field_name".to_string(),
    ///     new_value
    /// );
    /// self.instr().add_instruction(insert);
    /// // Rebind variable to new struct value
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Use InsertField instruction for value-based field updates instead"
    )]
    pub fn store_field(
        &mut self,
        base_addr: Value,
        offset: Value,
        value: Value,
        field_type: MirType,
        field_name: &str,
    ) {
        eprintln!("WARNING: store_field() is deprecated. Use InsertField instruction instead.");

        let field_addr = self.state.mir_function
            .new_typed_value_id(MirType::pointer(field_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(field_addr, base_addr, offset)
                .with_comment(format!("DEPRECATED: Get address of field '{}'", field_name))
        );
        self.instr().store(Value::operand(field_addr), value, field_type);
    }

    // Similar deprecation for load_tuple_element, store_tuple_element, etc.
}
````

### 2. Update Internal Usage Sites

Systematically replace all internal usage of deprecated helpers in:

**`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs`:**

- `lower_member_access()` - Replace `load_field()` with `ExtractField`
- `lower_tuple_index()` - Replace `load_tuple_element()` with `ExtractTuple`
- `lower_lvalue_expression()` - Update to avoid deprecated address helpers

**`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/stmt.rs`:**

- `lower_assignment_statement()` - Replace `store_field()` with `InsertField` +
  rebinding
- `lower_pattern()` - Replace tuple destructuring helpers with `ExtractTuple`

**`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/utils.rs`:**

- Update any utility functions that use deprecated helpers

### 3. Documentation Updates

**Add migration guide to
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/README.md`:**

````markdown
## Migration Guide: Memory-Based to Value-Based Aggregates

### Deprecated Patterns

The following memory-based patterns are deprecated:

```rust
// DEPRECATED: Memory-based field access
let value = builder.load_field(addr, offset, field_type, "field_name");

// DEPRECATED: Memory-based field update
builder.store_field(addr, offset, new_value, field_type, "field_name");

// DEPRECATED: Memory-based tuple access
let elem = builder.load_tuple_element(tuple_addr, index, elem_type);
```
````

### Preferred Patterns

Use value-based aggregate operations instead:

```rust
// PREFERRED: Value-based field access
let value_id = self.state.mir_function.new_typed_value_id(field_type);
self.instr().add_instruction(
    Instruction::extract_struct_field(value_id, struct_value, "field_name", field_type)
);

// PREFERRED: Value-based field update
let new_struct_id = self.state.mir_function.new_typed_value_id(struct_type);
self.instr().add_instruction(
    Instruction::insert_struct_field(new_struct_id, old_struct, "field_name", new_value)
);
// Rebind variable to new_struct_id

// PREFERRED: Value-based tuple access
let elem_id = self.state.mir_function.new_typed_value_id(elem_type);
self.instr().add_instruction(
    Instruction::extract_tuple_element(elem_id, tuple_value, index, elem_type)
);
```

````

### 4. Migration Guidance

**Create helper utilities for common migration patterns:**

```rust
impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Helper for migrating from store_field to value-based field updates
    /// This creates the InsertField instruction and returns the new struct value
    pub fn update_struct_field(
        &mut self,
        struct_value: Value,
        field_name: &str,
        new_value: Value,
        struct_type: MirType,
        field_type: MirType,
    ) -> ValueId {
        let new_struct_id = self.state.mir_function.new_typed_value_id(struct_type);
        self.instr().add_instruction(
            Instruction::insert_struct_field(
                new_struct_id,
                struct_value,
                field_name.to_string(),
                new_value
            ).with_comment(format!("Update field '{}'", field_name))
        );
        new_struct_id
    }

    /// Helper for migrating from load_field to value-based field access
    pub fn extract_struct_field(
        &mut self,
        struct_value: Value,
        field_name: &str,
        field_type: MirType,
    ) -> ValueId {
        let field_id = self.state.mir_function.new_typed_value_id(field_type.clone());
        self.instr().add_instruction(
            Instruction::extract_struct_field(
                field_id,
                struct_value,
                field_name.to_string(),
                field_type
            ).with_comment(format!("Extract field '{}'", field_name))
        );
        field_id
    }
}
````

### 5. Testing Strategy

**Unit Tests for Deprecation Warnings:**

```rust
#[test]
fn test_deprecated_load_field_shows_warning() {
    // Verify deprecation warnings are shown when using old API
}

#[test]
fn test_migration_produces_equivalent_mir() {
    // Ensure migrated code produces functionally equivalent MIR
}
```

**Integration Tests:**

- Verify all internal usage has been migrated
- Ensure no performance regression
- Validate that new value-based approach produces cleaner MIR

**Snapshot Tests:**

- Compare MIR output before and after migration
- Ensure optimization passes work correctly with new instructions

### 6. Rollout Plan

**Week 1-2:** Mark methods as deprecated with warnings **Week 3-4:** Update all
internal usage sites  
**Week 5:** Update documentation and add migration helpers **Week 6:** Testing
and validation **Week 7+:** Monitor for external usage and provide migration
support

## Testing

### Automated Testing

1. **Deprecation Warning Tests** - Verify warnings appear when using old API
2. **Migration Correctness Tests** - Ensure old and new approaches produce
   equivalent behavior
3. **Performance Tests** - Verify no regression in compilation time
4. **Integration Tests** - Check all compiler phases work with updated lowering

### Manual Testing

1. **Code Review** - Ensure all internal usage has been migrated
2. **Documentation Review** - Verify migration guide is clear and complete
3. **External API Audit** - Check if any external code depends on deprecated
   methods

## Impact

### Positive Impact

- **Cleaner Codebase** - Forces migration to better value-based patterns
- **Improved Performance** - Reduces need for expensive memory-to-SSA conversion
  passes
- **Better Developer Experience** - Clear guidance on preferred patterns
- **Simplified Optimization** - Value-based operations are easier to optimize
- **Reduced Complexity** - Single approach reduces cognitive load

### Risk Assessment

- **Low Breaking Risk** - Deprecation warnings give time for migration
- **Medium Adoption Risk** - Developers might continue using deprecated methods
- **Low Performance Risk** - New approach should be faster, not slower

### Mitigation Strategies

- **Gradual Migration** - Deprecation warnings before removal
- **Clear Documentation** - Comprehensive migration guide
- **Helper Utilities** - Make migration easier with convenience methods
- **Testing Coverage** - Ensure equivalent behavior during transition

## Success Criteria

1. **All deprecated methods marked** with clear deprecation warnings
2. **Zero internal usage** of deprecated memory-based helpers
3. **Comprehensive documentation** for migration patterns
4. **No performance regression** in compilation benchmarks
5. **Equivalent MIR output** for migrated code (when functionally appropriate)
6. **Clean test suite** with all tests passing after migration

This deprecation strategy provides a clear path away from memory-heavy patterns
toward the new value-based aggregate approach, supporting the overall MIR
refactoring goals while maintaining backward compatibility during the transition
period.
