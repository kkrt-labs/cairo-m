# Task 003: Synthesize Tuples for Multi-Return Function Calls [CRITICAL]

## Priority: CRITICAL - Blocks value-based design

## Summary

Fix function call expression handling to synthesize tuple values using
`MakeTuple` instead of spilling multiple return values to memory. This is
essential for maintaining value-based semantics throughout the IR.

## Current State

- ❌ Multi-return functions spill to stack via `frame_alloc`
- ❌ Uses `get_element_ptr` + `store` for each return value
- ❌ Returns memory address instead of tuple value
- ❌ Creates unnecessary memory traffic

## Affected Code Location

### Primary Target: `crates/compiler/mir/src/lowering/expr.rs`

#### `lower_function_call_expr` (lines 365-394)

**Current Implementation (WRONG):**

```rust
// Allocates memory for tuple
self.instr().add_instruction(
    Instruction::frame_alloc(tuple_addr, tuple_type.clone())
        .with_comment("Allocate space for tuple return value".to_string()),
);

// Stores each return value to memory
for (i, ret_val) in ret_vals.iter().enumerate() {
    let element_ptr = self.get_element_ptr(tuple_addr, vec![i as u32], element_type);
    self.instr().add_instruction(Instruction::store(element_ptr, *ret_val));
}

// Returns memory address
Ok(Value::Aggregate(AggregatePath::Address(tuple_addr)))
```

**Required Implementation:**

```rust
// When function returns multiple values
if ret_vals.len() > 1 {
    // Synthesize tuple value using MakeTuple
    let tuple_value = self.make_tuple(ret_vals.clone());

    // Return as SSA value, not memory address
    Ok(Value::operand(tuple_value))
} else if ret_vals.len() == 1 {
    // Single return value - use directly
    Ok(Value::operand(ret_vals[0]))
} else {
    // Void return
    Ok(Value::void())
}
```

## Implementation Steps

### Step 1: Remove Memory Allocation

- Delete `frame_alloc` instruction generation
- Remove `get_element_ptr` calculations
- Eliminate `store` operations for return values

### Step 2: Implement Tuple Synthesis

```rust
// In lower_function_call_expr
match call_result {
    CallResult::MultipleReturns(values) => {
        // Create tuple from multiple return values
        let tuple_id = self.make_tuple(values);
        Ok(Value::operand(tuple_id))
    }
    CallResult::SingleReturn(value) => {
        Ok(Value::operand(value))
    }
    CallResult::Void => {
        Ok(Value::void())
    }
}
```

### Step 3: Update Return Value Handling

Ensure the synthesized tuple can be:

- Destructured via `ExtractTupleElement`
- Passed to other functions
- Assigned to variables
- Used in expressions

### Step 4: Fix Related Code

Update any code that expects function calls to return memory addresses:

- Pattern matching on function results
- Assignment of function results
- Using function results in expressions

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_multi_return_creates_tuple_value() {
    // fn returns_pair() -> (i32, i32) { (1, 2) }
    // let result = returns_pair();
    // Should create MakeTuple, not memory allocation
}

#[test]
fn test_tuple_result_extraction() {
    // let (a, b) = returns_pair();
    // Should use ExtractTupleElement on SSA value
}

#[test]
fn test_nested_function_calls() {
    // let result = outer(inner());
    // Should pass tuple values, not addresses
}
```

### Integration Tests

- Test with various return value counts (0, 1, 2, 5+)
- Test tuple results in different contexts
- Verify no memory operations for tuple returns
- Test with optimization passes enabled

## Edge Cases

1. **Void functions** - Should return `Value::void()`
2. **Single return** - Should return value directly, no tuple
3. **Many returns** (5+) - Should still use `MakeTuple`
4. **Nested calls** - `f(g())` where g returns tuple
5. **Tuple element access** - `f().0` should work

## Verification Checklist

- [ ] No `frame_alloc` for function return values
- [ ] No `store` operations for tuple returns
- [ ] `MakeTuple` instruction generated for multi-returns
- [ ] Return values are SSA values, not addresses
- [ ] Pattern matching works with tuple returns
- [ ] All tests pass with new implementation

## Performance Impact

- Eliminates memory allocation for return values
- Reduces memory traffic significantly
- Enables better optimization opportunities
- Allows return value elimination when unused

## Dependencies

- Requires `MakeTuple` instruction support ✅
- Requires `ExtractTupleElement` for destructuring ✅

## Code to Remove

After implementation, these should be deleted:

```rust
// Remove this entire block from lower_function_call_expr
let tuple_addr = self.new_temp_local();
let tuple_type = self.semantic_db.mir_type(self.file, &return_type);
self.instr().add_instruction(
    Instruction::frame_alloc(tuple_addr, tuple_type.clone())
        .with_comment("Allocate space for tuple return value".to_string()),
);
// ... all the store operations ...
```

## Success Criteria

1. Function calls returning tuples use `MakeTuple`
2. No memory operations for tuple returns
3. Tuple values can be destructured correctly
4. Performance improvement in function-heavy code
5. Validation pass confirms SSA form correctness
