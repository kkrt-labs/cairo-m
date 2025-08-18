# Task 03: Add Insert+Extract Folding Rules to ConstFold Pass

## Overview

This task implements new constant folding rules for Insert+Extract combinations
in the MIR ConstFold pass. These optimizations will fold patterns like:

- `ExtractStructField(InsertField(s, f, v), g)` → direct field access or the
  inserted value
- `ExtractTupleElement(InsertTuple(t, i, v), j)` → direct element access or the
  inserted value

## Analysis of Existing Folding Patterns

### Current Implementation Structure

The `ConstFoldPass` in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/const_fold.rs`
implements folding through:

1. **Single-pass analysis**: Builds a `value_defs` map of all definitions in the
   current block
2. **Pattern matching**: Iterates through instructions looking for foldable
   patterns
3. **Replacement generation**: Creates new instructions to replace optimizable
   patterns
4. **Dead code elimination**: Removes unused aggregate creation instructions

### Existing Folding Patterns

#### Extract+Make Patterns (Lines 47-110)

- **ExtractTupleElement(MakeTuple(...))**: Directly returns the element at the
  given index
- **ExtractStructField(MakeStruct(...))**: Directly returns the field value

#### Insert+Make Patterns (Lines 112-180)

- **InsertField(MakeStruct(...))**: Creates new MakeStruct with updated field
- **InsertTuple(MakeTuple(...))**: Creates new MakeTuple with updated element

### Key Design Principles

1. **Value-based operations**: All aggregate operations work on values, not
   memory addresses
2. **Single-block scope**: Folding only considers definitions within the same
   basic block
3. **Type preservation**: Folded operations maintain the same types as original
   operations
4. **Comment generation**: Folded instructions include descriptive comments for
   debugging

## New Folding Rules Specification

### Rule 1: ExtractStructField(InsertField(s, f, v), g)

**Pattern**:
`dest = extract_field(insert_field(struct_val, field_f, new_val), field_g)`

**Folding Logic**:

- If `field_f == field_g`: Replace with `dest = new_val` (direct assignment)
- If `field_f != field_g`: Replace with
  `dest = extract_field(struct_val, field_g)` (bypass the insert)

**Requirements**:

- Both operations must be in the same basic block
- The InsertField result must only be used by this ExtractStructField (to enable
  dead code elimination)
- Field names must be compared as strings
- Type safety: `new_val` type must match the expected field type when
  `field_f == field_g`

### Rule 2: ExtractTupleElement(InsertTuple(t, i, v), j)

**Pattern**:
`dest = extract_tuple(insert_tuple(tuple_val, index_i, new_val), index_j)`

**Folding Logic**:

- If `index_i == index_j`: Replace with `dest = new_val` (direct assignment)
- If `index_i != index_j`: Replace with
  `dest = extract_tuple(tuple_val, index_j)` (bypass the insert)

**Requirements**:

- Both operations must be in the same basic block
- The InsertTuple result must only be used by this ExtractTupleElement (to
  enable dead code elimination)
- Index bounds must be validated
- Type safety: `new_val` type must match the expected element type when
  `index_i == index_j`

## Implementation Plan

### Location

File:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/const_fold.rs`
Function: `fold_block()` (lines 26-200)

### Code Changes

#### 1. Add New Pattern Matching (Insert after line 180, before the `_ => {}` case)

```rust
// Fold ExtractStructField(InsertField(s, f, v), g)
InstructionKind::ExtractStructField {
    dest,
    struct_val,
    field_name: extract_field,
    field_ty,
} => {
    if let Value::Operand(struct_id) = struct_val {
        if let Some(insert_def) = value_defs.get(struct_id) {
            if let InstructionKind::InsertField {
                struct_val: original_struct,
                field_name: insert_field,
                new_value: inserted_value,
                ..
            } = &insert_def.kind
            {
                if insert_field == extract_field {
                    // Same field: extract the inserted value directly
                    let replacement = Instruction::assign(
                        *dest,
                        *inserted_value,
                        field_ty.clone(),
                    )
                    .with_comment(format!(
                        "Folded ExtractField(InsertField({}, \"{}\"), \"{}\")",
                        struct_id.index(),
                        insert_field,
                        extract_field
                    ));
                    replacements.push((idx, replacement));
                    changed = true;
                } else {
                    // Different field: extract from original struct
                    let replacement = Instruction::extract_struct_field(
                        *dest,
                        *original_struct,
                        extract_field.clone(),
                        field_ty.clone(),
                    )
                    .with_comment(format!(
                        "Folded ExtractField(InsertField({}, \"{}\"), \"{}\")",
                        struct_id.index(),
                        insert_field,
                        extract_field
                    ));
                    replacements.push((idx, replacement));
                    changed = true;
                }
            }
        }
    }
}

// Fold ExtractTupleElement(InsertTuple(t, i, v), j)
InstructionKind::ExtractTupleElement {
    dest,
    tuple,
    index: extract_index,
    element_ty,
} => {
    if let Value::Operand(tuple_id) = tuple {
        if let Some(insert_def) = value_defs.get(tuple_id) {
            if let InstructionKind::InsertTuple {
                tuple_val: original_tuple,
                index: insert_index,
                new_value: inserted_value,
                ..
            } = &insert_def.kind
            {
                if insert_index == extract_index {
                    // Same index: extract the inserted value directly
                    let replacement = Instruction::assign(
                        *dest,
                        *inserted_value,
                        element_ty.clone(),
                    )
                    .with_comment(format!(
                        "Folded ExtractTuple(InsertTuple({}, {}), {})",
                        tuple_id.index(),
                        insert_index,
                        extract_index
                    ));
                    replacements.push((idx, replacement));
                    changed = true;
                } else {
                    // Different index: extract from original tuple
                    let replacement = Instruction::extract_tuple_element(
                        *dest,
                        *original_tuple,
                        *extract_index,
                        element_ty.clone(),
                    )
                    .with_comment(format!(
                        "Folded ExtractTuple(InsertTuple({}, {}), {})",
                        tuple_id.index(),
                        insert_index,
                        extract_index
                    ));
                    replacements.push((idx, replacement));
                    changed = true;
                }
            }
        }
    }
}
```

#### 2. Handle Existing Extract Patterns with Insert Check

The existing `ExtractTupleElement` and `ExtractStructField` patterns (lines
47-110) need to be updated to handle the new Insert+Extract folding. The code
above should **replace** the existing extract patterns to handle both Make and
Insert cases.

### Edge Cases and Safety Considerations

1. **Type Safety**: The folded operations must preserve type correctness
   - When folding same-field/index accesses, verify that `new_value` type
     matches expected type
   - When folding different-field/index accesses, preserve original extract type

2. **Bounds Checking**: For tuple operations, ensure indices are within bounds
   - InsertTuple validation should already ensure `insert_index` is valid
   - ExtractTupleElement validation should already ensure `extract_index` is
     valid

3. **Single Use Validation**: While not strictly required for correctness, these
   optimizations are most beneficial when the intermediate Insert result has
   only one use (allowing dead code elimination)

4. **Control Flow**: Only fold within the same basic block to avoid invalidating
   the value_defs map

5. **Memory Semantics**: All operations are value-based, so no memory aliasing
   concerns

## Test Cases

### Test 1: Same Field/Index Access

```rust
#[test]
fn test_insert_extract_same_field_folding() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.entry_block;

    // Create: s1 = MakeStruct{x: a, y: b}; s2 = InsertField(s1, "x", c); result = ExtractField(s2, "x")
    // Should fold to: result = c

    let a = function.new_typed_value_id(MirType::felt());
    let b = function.new_typed_value_id(MirType::felt());
    let c = function.new_typed_value_id(MirType::felt());
    let s1 = function.new_typed_value_id(struct_type.clone());
    let s2 = function.new_typed_value_id(struct_type.clone());
    let result = function.new_typed_value_id(MirType::felt());

    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let block = function.get_basic_block_mut(entry).unwrap();

    // s1 = MakeStruct{x: a, y: b}
    block.instructions.push(Instruction::make_struct(
        s1,
        vec![
            ("x".to_string(), Value::operand(a)),
            ("y".to_string(), Value::operand(b)),
        ],
        struct_type.clone(),
    ));

    // s2 = InsertField(s1, "x", c)
    block.instructions.push(Instruction::insert_field(
        s2,
        Value::operand(s1),
        "x".to_string(),
        Value::operand(c),
        struct_type,
    ));

    // result = ExtractField(s2, "x")
    block.instructions.push(Instruction::extract_struct_field(
        result,
        Value::operand(s2),
        "x".to_string(),
        MirType::felt(),
    ));

    // Run optimization
    let pass = ConstFoldPass::new();
    let changed = pass.fold_block(&mut function, entry);
    assert!(changed);

    // Verify last instruction is assign: result = c
    let block = function.get_basic_block(entry).unwrap();
    let last_instr = &block.instructions[2];
    match &last_instr.kind {
        InstructionKind::Assign { source, dest, .. } => {
            assert_eq!(*dest, result);
            assert_eq!(*source, Value::operand(c));
        }
        _ => panic!("Expected Assign instruction after folding"),
    }
}
```

### Test 2: Different Field/Index Access

```rust
#[test]
fn test_insert_extract_different_field_folding() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.entry_block;

    // Create: s1 = MakeStruct{x: a, y: b}; s2 = InsertField(s1, "x", c); result = ExtractField(s2, "y")
    // Should fold to: result = ExtractField(s1, "y") -> result = b

    // Similar setup as above, but extract "y" instead of "x"
    // The folding should bypass the insert and go directly to original struct
}
```

### Test 3: Tuple Operations

```rust
#[test]
fn test_insert_extract_tuple_same_index() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.entry_block;

    // Create: t1 = MakeTuple(a, b, c); t2 = InsertTuple(t1, 1, d); result = ExtractTuple(t2, 1)
    // Should fold to: result = d

    // Similar structure to struct test but with tuples and indices
}

#[test]
fn test_insert_extract_tuple_different_index() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.entry_block;

    // Create: t1 = MakeTuple(a, b, c); t2 = InsertTuple(t1, 1, d); result = ExtractTuple(t2, 2)
    // Should fold to: result = ExtractTuple(t1, 2) -> result = c
}
```

### Test 4: Complex Chaining

```rust
#[test]
fn test_chained_insert_extract_folding() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.entry_block;

    // Create: s1 = MakeStruct{x: a, y: b}; s2 = InsertField(s1, "x", c); s3 = InsertField(s2, "y", d); result = ExtractField(s3, "x")
    // Should fold through multiple passes to: result = c

    // This tests that multiple optimization passes can handle chained operations
}
```

## Performance Impact Assessment

### Positive Impacts

1. **Instruction Reduction**: Eliminates redundant Insert+Extract pairs
2. **Memory Pressure**: Reduces intermediate values that may need to be spilled
3. **Register Allocation**: Fewer temporary values reduce register pressure
4. **Code Size**: Smaller generated CASM code
5. **Runtime Performance**: Direct value access instead of aggregate operations

### Potential Concerns

1. **Compilation Time**: Additional pattern matching adds minimal overhead to
   the pass
2. **Code Complexity**: The patterns are straightforward and follow existing
   conventions
3. **Debug Information**: Generated comments help preserve optimization
   information

### Optimization Opportunities Enabled

1. **Dead Code Elimination**: Unused Insert instructions can be removed after
   folding
2. **Copy Propagation**: Direct value assignments enable further optimizations
3. **Constant Propagation**: Folding with literal values enables constant
   propagation
4. **Loop Optimizations**: Reduced aggregate manipulation in loops

## Implementation Risks and Mitigations

### Risks

1. **Type Mismatch**: Incorrect type handling could break code generation
   - **Mitigation**: Preserve original type annotations and add debug assertions

2. **Control Flow Issues**: Cross-block optimizations could be problematic
   - **Mitigation**: Only operate within single basic blocks (current design)

3. **Use-After-Free Logic**: Incorrect dead code elimination
   - **Mitigation**: Use existing `get_value_use_counts()` infrastructure

### Testing Strategy

1. **Unit Tests**: Comprehensive test cases for all patterns and edge cases
2. **Snapshot Tests**: Verify MIR output matches expected optimized form
3. **Integration Tests**: Test with real Cairo-M programs to ensure correctness
4. **Regression Tests**: Ensure existing folding patterns continue to work

## Conclusion

This optimization adds significant value by eliminating common Insert+Extract
patterns that arise from functional updates in aggregate-first MIR. The
implementation follows existing patterns in the codebase and maintains type
safety and correctness. The performance benefits, particularly for
aggregate-heavy code, justify the additional complexity.

The implementation is low-risk because:

- It follows established patterns in the existing ConstFold pass
- It operates within single basic blocks only
- It preserves type information and generates helpful debug comments
- The optimization is purely additive and doesn't modify existing folding logic

The test suite will ensure correctness and catch any regressions, while the
performance impact should be universally positive for programs that use
aggregate types extensively.
