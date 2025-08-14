# Task: Fix Silent Type Fallback in Struct Literal Lowering

## Priority

CRITICAL

## Why

This issue represents a critical type safety vulnerability in the MIR generation
phase. When lowering struct literals, the compiler currently silently falls back
to `felt` type if a struct field is not found in the type information. This
silent fallback can:

1. **Hide bugs**: Typos in field names or struct definition mismatches will not
   be caught
2. **Generate incorrect code**: Fields will be stored with wrong types, leading
   to runtime errors or incorrect behavior
3. **Violate type safety**: The compiler's type system promises are broken when
   incorrect types are used silently
4. **Mask structural problems**: Issues in semantic analysis or type propagation
   may go undetected

## What

In `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs`,
the `lower_struct_literal` function has a silent type fallback issue on lines
753-756:

```rust
// Store the field value
let field_type = struct_type
    .field_type(field_name.value())
    .unwrap_or(&MirType::felt())  // ⚠️ CRITICAL ISSUE: Silent fallback to felt
    .clone();
```

The `field_type()` method returns `Option<&MirType>`, and when `None` is
returned (field not found), the code silently falls back to `MirType::felt()`.
This means:

- If a field doesn't exist in the struct definition, it will be treated as
  `felt` type
- No error is generated, hiding the underlying issue
- The struct layout and type information become inconsistent

## How

### Current problematic code (lines 753-756):

```rust
// Store the field value
let field_type = struct_type
    .field_type(field_name.value())
    .unwrap_or(&MirType::felt())  // ⚠️ Silent fallback hides bugs
    .clone();
```

### Fixed code with proper error handling:

```rust
// Store the field value
let field_type = struct_type
    .field_type(field_name.value())
    .ok_or_else(|| {
        format!(
            "Internal Compiler Error: Field '{}' not found on struct type '{:?}'. This indicates an issue with type information propagation from semantic analysis to MIR lowering.",
            field_name.value(),
            struct_type
        )
    })?
    .clone();
```

### Location:

- **File**:
  `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs`
- **Function**: `lower_struct_literal`
- **Lines**: 753-756
- **Method**: `MirBuilder::lower_struct_literal`

### Root cause analysis:

The issue occurs because earlier in the function (lines 712-721), there's
already proper error handling for field offset calculation, but the field type
retrieval doesn't follow the same pattern. The code should be consistent in its
error handling approach.

## Testing

### Test cases to verify the fix:

1. **Invalid field name test**: Create a struct literal with a field that
   doesn't exist in the struct definition

   ```rust
   // This should generate a proper error, not silently use felt type
   struct Point { x: u32, y: u32 }
   let p = Point { x: 1, z: 2 }; // 'z' field doesn't exist
   ```

2. **Type propagation test**: Ensure that when semantic analysis correctly
   identifies struct fields, MIR lowering uses the exact field types

   ```rust
   struct Mixed { a: u32, b: felt, c: bool }
   let m = Mixed { a: 42_u32, b: 123, c: true };
   ```

3. **Regression test**: Verify that valid struct literals continue to work
   correctly after the fix

4. **Error message test**: Verify that the error message provides helpful
   diagnostic information

### Testing approach:

- Add unit tests in `mir/src/lowering/expr.rs`
- Add integration tests that compile invalid struct literals and verify error
  generation
- Use snapshot testing to ensure consistent error messages

## Impact

### Safety improvements:

1. **Type safety**: Ensures that all field types are correctly validated and
   used
2. **Early error detection**: Catches field name mismatches and type propagation
   issues during compilation
3. **Consistent error handling**: Aligns with the existing error handling
   pattern used for field offset calculation
4. **Better diagnostics**: Provides clear error messages when type information
   is inconsistent

### Risk mitigation:

- The fix converts a silent bug into a proper compilation error
- Catches issues that could lead to runtime memory corruption or incorrect
  behavior
- Prevents cascading failures from incorrect type information

### Development benefits:

- Makes debugging type-related issues much easier
- Ensures that semantic analysis and MIR lowering are properly synchronized
- Maintains the integrity of the compiler's type system guarantees
