# MIR Aggregate Migration Guide

This guide helps contributors migrate from memory-based aggregate patterns to
the new value-based aggregate system in Cairo-M MIR.

## Quick Reference

| Operation            | Old (Memory-Based)          | New (Value-Based)              |
| -------------------- | --------------------------- | ------------------------------ |
| Create tuple         | `framealloc` + `store`      | `make_tuple`                   |
| Create struct        | `framealloc` + `store`      | `make_struct`                  |
| Access field         | `get_element_ptr` + `load`  | `extract_field`                |
| Access tuple element | `get_element_ptr` + `load`  | `extract_tuple`                |
| Update field         | `get_element_ptr` + `store` | `insert_field` + SSA rebinding |

## Identifying Legacy Patterns

### Pattern 1: Memory-Based Tuple Creation

**Look for:**

```rust
// In lowering code
let alloca = self.builder.alloca(tuple_type);
for (i, elem) in elements.iter().enumerate() {
    let ptr = self.builder.get_element_ptr(alloca, i);
    self.builder.store(ptr, elem);
}
let result = self.builder.load(alloca);
```

**Replace with:**

```rust
let result = self.builder.make_tuple(elements);
```

### Pattern 2: Memory-Based Struct Creation

**Look for:**

```rust
let alloca = self.builder.alloca(struct_type);
for (field_name, value) in fields {
    let offset = struct_type.field_offset(&field_name);
    let ptr = self.builder.get_element_ptr(alloca, offset);
    self.builder.store(ptr, value);
}
let result = self.builder.load(alloca);
```

**Replace with:**

```rust
let result = self.builder.make_struct(struct_name, fields);
```

### Pattern 3: Field Access Through Memory

**Look for:**

```rust
let addr = self.builder.address_of(struct_value);
let field_ptr = self.builder.get_element_ptr(addr, field_offset);
let value = self.builder.load(field_ptr);
```

**Replace with:**

```rust
let value = self.builder.extract_field(struct_value, field_name);
```

## Implementing New Features

### When to Use Value-Based Aggregates

**Always use value-based for:**

- Tuple literals: `(1, 2, 3)`
- Struct literals: `Point { x: 1, y: 2 }`
- Field access: `point.x`
- Pattern matching on aggregates
- Function returns of aggregates

**Continue using memory-based for:**

- Arrays: `[1, 2, 3, 4, 5]`
- Explicit address operations: `&value`
- Foreign function interfaces requiring memory layout
- Large aggregates (>10 fields) that might benefit from memory

### Example: Lowering a Struct Literal

```rust
// In ir_generation.rs or similar

fn lower_struct_literal(&mut self, struct_expr: &StructExpr) -> Value {
    let struct_name = struct_expr.name.clone();
    let struct_type = self.get_struct_type(&struct_name);

    // Collect field values
    let mut fields = Vec::new();
    for field in &struct_expr.fields {
        let value = self.lower_expression(&field.value);
        fields.push((field.name.clone(), value));
    }

    // Use value-based construction
    let result_id = self.function.new_value_id();
    self.builder.make_struct(result_id, struct_name, fields);
    Value::operand(result_id)
}
```

### Example: Lowering Field Access

```rust
fn lower_field_access(&mut self, base: &Expr, field: &str) -> Value {
    let base_value = self.lower_expression(base);

    // Use value-based extraction
    let result_id = self.function.new_value_id();
    self.builder.extract_field(result_id, base_value, field.to_string());
    Value::operand(result_id)
}
```

### Example: Handling Assignment

```rust
fn lower_field_assignment(&mut self, base: &Expr, field: &str, value: &Expr) -> Value {
    let base_value = self.lower_expression(base);
    let new_value = self.lower_expression(value);

    // Create new struct with updated field
    let result_id = self.function.new_value_id();
    self.builder.insert_field(result_id, base_value, field.to_string(), new_value);

    // Handle SSA rebinding (variable update)
    self.update_variable_binding(base, Value::operand(result_id));
    Value::operand(result_id)
}
```

## Common Pitfalls

### Pitfall 1: Mixing Memory and Value Operations

**Wrong:**

```rust
let tuple = self.builder.make_tuple(vec![val1, val2]);
let ptr = self.builder.get_element_ptr(tuple, 0); // Error: tuple is a value, not a pointer
```

**Right:**

```rust
let tuple = self.builder.make_tuple(vec![val1, val2]);
let elem = self.builder.extract_tuple(tuple, 0);
```

### Pitfall 2: Forgetting SSA Rebinding

**Wrong:**

```rust
// Trying to mutate in place
self.builder.insert_field(struct_val, "x", new_x); // Creates new value!
// struct_val still has old value
```

**Right:**

```rust
let new_struct = self.builder.insert_field(struct_val, "x", new_x);
self.update_variable_binding(var_name, new_struct);
```

### Pitfall 3: Using Memory for Small Aggregates

**Avoid:**

```rust
// Don't use memory for simple tuples/structs
let pair_alloca = self.builder.alloca(pair_type);
// ... memory operations ...
```

**Prefer:**

```rust
let pair = self.builder.make_tuple(vec![first, second]);
```

## Testing Your Migration

### 1. Unit Tests for New Instructions

```rust
#[test]
fn test_struct_value_operations() {
    let mut function = MirFunction::new("test");
    let builder = MirBuilder::new(&mut function);

    // Test value-based struct creation
    let s = builder.make_struct("Point", vec![
        ("x", Value::integer(10)),
        ("y", Value::integer(20)),
    ]);

    // Test field extraction
    let x = builder.extract_field(s, "x");
    assert_eq!(x, Value::integer(10));

    // Test field update
    let s2 = builder.insert_field(s, "x", Value::integer(30));
    let new_x = builder.extract_field(s2, "x");
    assert_eq!(new_x, Value::integer(30));
}
```

### 2. Snapshot Tests

Create snapshot tests to verify MIR output:

```rust
#[test]
fn test_aggregate_lowering_snapshot() {
    let source = r#"
        struct Point { x: felt, y: felt }
        fn main() -> Point {
            Point { x: 1, y: 2 }
        }
    "#;

    let mir = compile_to_mir(source);
    insta::assert_snapshot!(mir.to_string());
}
```

Expected snapshot should show:

```mir
fn main() -> Point {
bb0:
    %0 = make_struct Point { x: 1, y: 2 }
    return %0
}
```

### 3. Performance Validation

```rust
#[bench]
fn bench_aggregate_heavy_compilation(b: &mut Bencher) {
    let source = generate_aggregate_heavy_code();
    b.iter(|| {
        compile_with_value_aggregates(&source)
    });
}
```

## Gradual Migration Strategy

### Phase 1: Add New Instructions

1. Implement `make_tuple`, `extract_tuple`, etc.
2. Add builder methods
3. Write comprehensive tests

### Phase 2: Update Lowering

1. Start with simple cases (literals)
2. Move to field access
3. Handle assignments and control flow

### Phase 3: Conditional Usage

1. Add feature flags to enable/disable
2. Run A/B tests comparing approaches
3. Gradually enable for more code patterns

### Phase 4: Remove Old Code

1. Once stable, remove memory-based aggregate lowering
2. Delete SROA/Mem2Reg passes
3. Update all documentation

## Debugging Tips

### MIR Inspection

Use verbose output to see generated MIR:

```bash
cargo run -- -i program.cm -v
```

Look for:

- `make_tuple`/`make_struct` for creation
- `extract_*` for access
- No `framealloc` for simple aggregates

### Common Error Messages

**"Cannot take address of value aggregate"**

- You're trying to use `&` on a value-based aggregate
- Solution: Let the compiler materialize to memory when needed

**"Type mismatch: expected pointer, got value"**

- Mixing memory and value operations
- Solution: Use appropriate value-based instruction

**"Undefined value after insert"**

- Forgot to update variable binding after insert operation
- Solution: Ensure SSA rebinding is handled

## Best Practices

1. **Think in Values**: Aggregates are values, not memory locations
2. **Immutable by Default**: Updates create new values
3. **Let SSA Handle Dataflow**: Don't try to track mutations manually
4. **Profile Before Optimizing**: Measure impact of value vs memory
5. **Keep Arrays on Memory Path**: Don't try to make arrays value-based
6. **Document Decisions**: Comment when choosing memory over values

## Getting Help

- Check existing value-based implementations in `mir/src/lowering/`
- Look at test cases in `mir/tests/aggregate_*.rs`
- Review the design document: `docs/mir_aggregate_first.md`
- Ask in team chat with #mir-aggregates tag

## Checklist for Contributors

When implementing aggregate-related features:

- [ ] Use value-based instructions for tuples/structs
- [ ] Keep arrays on memory path
- [ ] Handle SSA rebinding for updates
- [ ] Add comprehensive tests
- [ ] Update relevant documentation
- [ ] Verify no performance regression
- [ ] Check snapshot tests still pass
- [ ] Review generated MIR for expected patterns
