# AddressOf Operation Guidelines

## Overview

The AddressOf instruction is reserved for operations that require explicit
memory addresses, particularly for arrays and other memory-bound types. This
document outlines when and how AddressOf should be used.

## When to Use AddressOf

### Arrays (Future Implementation)

- Array element address calculation: `&arr[i]`
- Array slicing operations
- Passing arrays to functions that expect pointers

### Current Restrictions

- Tuples and structs should NOT use AddressOf for normal operations
- These types use value-based operations (MakeTuple, ExtractTuple, etc.)

## Implementation Pattern

```rust
// For future array implementation
match base_type {
    MirType::Array { .. } => {
        // Use AddressOf for array element references
        let elem_addr = self.calculate_element_address(base, index);
        self.instr().address_of(dest, elem_addr)
    }
    MirType::Tuple(_) | MirType::Struct { .. } => {
        // Use value-based operations, no AddressOf needed
        self.lower_value_aggregate_access(base, index)
    }
    _ => // handle other cases
}
```

## Memory Path vs Value Path

| Type   | Operations                                          | Path   |
| ------ | --------------------------------------------------- | ------ |
| Array  | framealloc, get_element_ptr, load, store, AddressOf | Memory |
| Tuple  | MakeTuple, ExtractTupleElement, InsertTuple         | Value  |
| Struct | MakeStruct, ExtractStructField, InsertField         | Value  |
