# MIR Builder API Migration Guide

## Overview

The MIR builder API has completed its transition from memory-based aggregate
operations to value-based operations. This guide documents the current
value-based patterns that should be used throughout the codebase.

## Deprecated Methods

The following memory-based helper methods are deprecated and will be removed in
a future version:

- `load_field()` - Use `extract_struct_field()` instead
- `store_field()` - Use `insert_struct_field()` instead
- `load_tuple_element()` - Use `extract_tuple_element()` instead
- `store_tuple_element()` - Use `insert_tuple()` instead
- `get_element_address()` - Use value-based operations when possible

## Migration Examples

### Field Access

**Old (Deprecated):**

```rust
// Load field from memory
let field_value = builder.load_field(
    struct_addr,
    offset,
    field_type,
    "field_name"
);
```

**New (Preferred):**

```rust
// Extract field from value
let field_value = builder.extract_struct_field(
    struct_value,
    "field_name",
    field_type
);
```

### Field Update

**Old (Deprecated):**

```rust
// Store to field in memory
builder.store_field(
    struct_addr,
    offset,
    new_value,
    field_type,
    "field_name"
);
```

**New (Preferred):**

```rust
// Create new struct with updated field
let new_struct = builder.insert_struct_field(
    old_struct,
    "field_name",
    new_value,
    struct_type
);
// Rebind variable to new_struct
```

### Tuple Element Access

**Old (Deprecated):**

```rust
// Load tuple element from memory
let elem = builder.load_tuple_element(
    tuple_addr,
    index,
    elem_type
);
```

**New (Preferred):**

```rust
// Extract element from tuple value
let elem = builder.extract_tuple_element(
    tuple_value,
    index,
    elem_type
);
```

### Tuple Element Update

**Old (Deprecated):**

```rust
// Store to tuple element in memory
builder.store_tuple_element(
    tuple_addr,
    index,
    new_value,
    elem_type
);
```

**New (Preferred):**

```rust
// Create new tuple with updated element
let new_tuple = builder.insert_tuple(
    old_tuple,
    index,
    new_value,
    tuple_type
);
// Rebind variable to new_tuple
```

## When Memory Operations Are Still Needed

Memory-based operations are still appropriate for:

1. **Arrays** - Arrays intentionally remain on the memory path
2. **External Memory** - Interfacing with external memory locations
3. **AddressOf Operations** - When explicit addresses are required

Example:

```rust
// Arrays still use memory operations
let elem_ptr = builder.get_element_ptr(array_addr, index);
let elem_value = builder.load(elem_ptr, elem_type);
```

## Benefits of Value-Based Operations

1. **Cleaner MIR** - Fewer instructions, easier to read
2. **Better Optimization** - Value-based operations optimize more easily
3. **Reduced Compilation Time** - No need for expensive SROA/mem2reg passes
4. **Simpler Semantics** - Values are immutable, reducing complexity

## Migration Strategy

1. **Phase 1** - Update new code to use value-based operations
2. **Phase 2** - Gradually migrate existing code when touched
3. **Phase 3** - Systematic migration of remaining usage
4. **Phase 4** - Remove deprecated methods (future release)

## Helper Functions

The builder provides these value-based aggregate operations:

```rust
// Tuple operations
make_tuple(elements, tuple_type) -> ValueId
extract_tuple_element(tuple, index, elem_type) -> ValueId
insert_tuple(tuple, index, new_value, tuple_type) -> ValueId

// Struct operations
make_struct(fields, struct_type) -> ValueId
extract_struct_field(struct, field_name, field_type) -> ValueId
insert_struct_field(struct, field_name, new_value, struct_type) -> ValueId
```

## Questions?

If you have questions about migration or encounter issues, please:

1. Check the test suite for examples
2. Look at recently migrated code for patterns
3. Ask in the development channel
