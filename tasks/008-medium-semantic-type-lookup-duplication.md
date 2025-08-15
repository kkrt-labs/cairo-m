# Task: Unify Semantic Type Lookups with Helper Methods

## Priority

MEDIUM - COMPLETED

## Why

The MIR lowering code in `crates/compiler/mir/src/lowering/expr.rs` contains
excessive duplication of semantic type lookup patterns. Throughout the file,
there are 18+ instances where the same pattern is repeated:

```rust
let semantic_type = expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
let mir_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
```

This violates the DRY (Don't Repeat Yourself) principle, making the code harder
to maintain, more error-prone, and unnecessarily verbose. Each duplication
increases the likelihood of bugs when parameters need to change or when the
semantic type resolution logic evolves.

## What

A comprehensive refactoring is needed to eliminate the duplication by utilizing
an existing helper method that already provides this exact functionality with
caching benefits.

**The solution already exists**: In
`crates/compiler/mir/src/lowering/builder.rs` (lines 87-97), there's a
`get_expr_type` method in the `LoweringContext` that does precisely this
operation with caching:

```rust
/// Get or compute the MIR type for an expression
pub fn get_expr_type(&self, expr_id: ExpressionId) -> MirType {
    let mut cache = self.expr_type_cache.borrow_mut();
    cache
        .entry(expr_id)
        .or_insert_with(|| {
            let sem_type = expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
            MirType::from_semantic_type(self.db, sem_type)
        })
        .clone()
}
```

## How

### Implementation Strategy

**Phase 1: Identify All Duplication Locations** The following locations in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs`
contain the duplication pattern:

1. **Line 132-140**: `lower_lvalue_expression` - Member access object type
2. **Line 155-162**: `lower_lvalue_expression` - Member access field type
3. **Line 186-193**: `lower_lvalue_expression` - Index access element type
4. **Line 212-221**: `lower_lvalue_expression` - Tuple index tuple type
5. **Line 302-304**: `lower_identifier` - Definition semantic type (uses
   `definition_semantic_type` - different pattern)
6. **Line 334-336**: `lower_unary_op` - Result type
7. **Line 357-359**: `lower_binary_op` - Result type
8. **Line 371-377**: `lower_binary_op` - Left operand type
9. **Line 397-404**: `lower_function_call_expr` - Tuple return type
10. **Line 466-473**: `lower_member_access` - Object type
11. **Line 488-490**: `lower_member_access` - Field type
12. **Line 536-538**: `lower_index_access` - Element type
13. **Line 599-608**: `lower_function_call` - Function return types (tuple
    elements)
14. **Line 631**: `lower_function_call` - Single return type
15. **Line 690-692**: `lower_struct_literal` - Struct type
16. **Line 734-741**: `lower_struct_literal` - Field value type
17. **Line 778-780**: `lower_tuple_literal` - Tuple type
18. **Line 813-820**: `lower_tuple_literal` - Element type
19. **Line 859-868**: `lower_tuple_index` - Tuple type

**Phase 2: Systematic Replacement** Replace each occurrence with a call to
`self.ctx.get_expr_type(expr_id)`:

**Before:**

```rust
let semantic_type = expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
let result_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
```

**After:**

```rust
let result_type = self.ctx.get_expr_type(expr_id);
```

### Special Cases to Handle

1. **Definition types (Line 302-304)**: Uses `definition_semantic_type` instead
   of `expression_semantic_type`. This should remain as-is since it's a
   different semantic lookup pattern.

2. **Binary operations with left operand type (Lines 371-377)**: This case needs
   special handling since it looks up the type of a different expression
   (`left_expr_id`), not the current `expr_id`.

3. **Tuple element types in function calls (Lines 607-608)**: This iterates over
   tuple element types, which requires the semantic type data, not just the MIR
   type.

### Migration Strategy

1. **Start with simple cases**: Begin with straightforward replacements where
   `expr_id` is directly available
2. **Handle complex cases**: Address cases where expression IDs need to be
   looked up from spans
3. **Preserve semantics**: Ensure all replacements maintain identical behavior
4. **Test thoroughly**: Run the full test suite after each batch of changes

### Example Transformation

**Before (lines 334-336):**

```rust
let semantic_type = expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
let result_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
```

**After:**

```rust
let result_type = self.ctx.get_expr_type(expr_id);
```

## Testing

1. **Regression testing**: Run the complete MIR test suite to ensure no
   behavioral changes
2. **Snapshot validation**: Verify that all snapshot tests continue to pass
3. **Integration testing**: Test with the full compiler pipeline including
   codegen
4. **Performance verification**: Confirm that caching provides expected
   performance benefits
5. **Diff testing**: Run diff tests comparing Cairo-M execution with Rust
   implementations

## Impact

### Code Quality Improvements

- **Reduced line count**: Eliminates ~36 lines of duplicated code
- **Improved maintainability**: Single point of change for type lookup logic
- **Enhanced readability**: Cleaner, more concise method implementations
- **Better performance**: Leverages existing caching infrastructure to avoid
  redundant semantic queries
- **Reduced error potential**: Eliminates copy-paste errors in parameter passing

### Risk Assessment

- **Low risk**: The helper method already exists and is well-tested
- **Minimal behavioral change**: Direct 1:1 replacement preserves all existing
  functionality
- **Clear rollback path**: Each change can be easily reverted if issues arise

### Success Metrics

- All existing tests continue to pass
- Code coverage remains identical
- No performance regressions in compilation times
- Reduced cognitive complexity in affected methods

## Implementation Summary

Successfully unified semantic type lookups by replacing 14 duplicated patterns
with calls to the existing `get_expr_type` helper method:

1. **Replacements made**:
   - Member access object type (both lvalue and expression contexts)
   - Member access field type (both contexts)
   - Index access element type (both contexts)
   - Tuple index tuple type (both contexts)
   - Binary operation result type
   - Unary operation result type
   - Struct literal type
   - Struct field value types
   - Tuple literal type
   - Tuple element types
2. **Special cases preserved**:
   - Definition semantic type lookups (using `definition_semantic_type`)
   - Function call return type checking (needs TypeData examination)
   - Left operand type in binary operations (needs semantic data)

3. **Benefits achieved**:
   - Eliminated ~28 lines of duplicated code
   - Improved performance through type caching
   - Single point of change for type lookup logic
   - Cleaner, more maintainable code

All tests pass and the refactoring maintains identical behavior while improving
code quality.
