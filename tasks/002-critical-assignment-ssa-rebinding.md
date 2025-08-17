# Task 002: Convert Assignments to SSA Rebinding [CRITICAL]

## Priority: CRITICAL - Required for value-based aggregates

## Summary

Transform assignment statements from memory-based stores to SSA value rebinding.
This includes simple assignments, field updates, and tuple element updates.

## Current State

- ❌ All assignments use memory `store` operations
- ❌ Field assignments don't use `InsertField` instruction
- ❌ Tuple element assignments don't use `InsertTuple` instruction
- ❌ Pattern destructuring uses memory loads

## Affected Code Locations

### Primary Target: `crates/compiler/mir/src/lowering/stmt.rs`

#### 1. `lower_assignment_statement` (lines 272-283)

**Current Implementation:**

```rust
// Gets memory address
let lhs_addr = self.lower_lvalue_expression(lhs)?;
// Stores to memory
self.instr().add_instruction(Instruction::store(lhs_addr, rhs_value));
```

**Required Change:**

```rust
match lhs {
    Expression::Variable(var) => {
        // SSA rebinding for simple variables
        let new_value = self.lower_expression(rhs)?;
        self.bind_variable(var.definition_id, new_value);
    }
    Expression::MemberAccess { target, member } => {
        // Use InsertField for struct member updates
        let struct_value = self.lower_expression(target)?;
        let field_value = self.lower_expression(rhs)?;
        let updated = self.insert_struct_field(struct_value, member, field_value);
        // Rebind the root variable
    }
    Expression::Index { target, index } if is_tuple => {
        // Use InsertTuple for tuple element updates
        let tuple_value = self.lower_expression(target)?;
        let element_value = self.lower_expression(rhs)?;
        let updated = self.insert_tuple(tuple_value, index, element_value);
        // Rebind the root variable
    }
    // Arrays continue using memory path
    _ => { /* existing memory-based code */ }
}
```

#### 2. `lower_pattern` (lines 685-695)

**Current Implementation:**

```rust
// Uses deprecated load_tuple_element
let element_value = self.load_tuple_element(tuple_addr, index, element_type);
```

**Required Change:**

```rust
// Use ExtractTupleElement on SSA value
let element_value = self.extract_tuple_element(tuple_value, index);
```

## Implementation Steps

### Step 1: Enhance Variable Binding System

Update `crates/compiler/mir/src/lowering/utils.rs`:

- Extend `bind_variable` to handle SSA rebinding
- Track variable versions for SSA form
- Support rebinding with new values

### Step 2: Implement Assignment Transformation

1. Detect assignment target type (variable, field, tuple element, array)
2. Route to appropriate handling:
   - Variables: Direct SSA rebinding
   - Fields: `InsertField` + rebind root
   - Tuples: `InsertTuple` + rebind root
   - Arrays: Keep memory path

### Step 3: Update Pattern Matching

- Convert tuple destructuring to use `ExtractTupleElement`
- Convert struct destructuring to use `ExtractStructField`
- Remove dependency on memory addresses for patterns

### Step 4: Helper Method Updates

Create new helper methods in `LoweringContext`:

```rust
fn rebind_aggregate_member(
    &mut self,
    base_expr: &Expression,
    updated_value: ValueId,
) -> Result<()>;

fn extract_lvalue_root(
    &self,
    expr: &Expression,
) -> Option<MirDefinitionId>;
```

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_simple_assignment_ssa_rebinding() {
    // x = 5;
    // x = 10;  // Should create new SSA value, not store
}

#[test]
fn test_field_assignment_uses_insert() {
    // point.x = 42;  // Should use InsertField
}

#[test]
fn test_tuple_element_assignment() {
    // tuple.0 = value;  // Should use InsertTuple
}

#[test]
fn test_pattern_destructuring_ssa() {
    // let (a, b) = tuple;  // Should use ExtractTupleElement
}
```

### Integration Tests

- Verify SSA form correctness
- Test control flow with assignments
- Ensure no memory operations for simple aggregates
- Test nested aggregate updates

## Verification Checklist

- [ ] Simple assignments create new SSA values
- [ ] Field updates use `InsertField` instruction
- [ ] Tuple updates use `InsertTuple` instruction
- [ ] Pattern matching uses Extract operations
- [ ] No `store` instructions for non-array aggregates
- [ ] Tests pass with new implementation

## Dependencies

- Depends on Task 001 (Variable-SSA Pass) for Phi node handling
- Requires existing Insert/Extract instructions ✅

## Edge Cases to Handle

1. Nested field access: `a.b.c = value`
2. Mixed tuple/struct: `data.field.0 = value`
3. Assignment in loops (requires Phi nodes)
4. Assignment in conditionals (requires Phi nodes)

## Performance Impact

- Eliminates memory operations for aggregates
- Reduces load/store traffic
- Enables better optimization opportunities
- May increase register pressure (beneficial for M31 architecture)

## Success Criteria

1. No memory operations for tuple/struct assignments
2. All tests pass with SSA rebinding
3. Correct SSA form verified by validation pass
4. Performance improvement in benchmarks
