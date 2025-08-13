# PR2: Typed GEP & SSA Aggregates

## Objective

Unify on typed GEP, lower aggregates to SSA instead of memory, and fix empty
tuple representation.

## Tasks

### Task 1: Standardize on typed GEP

**Current State:**

- Untyped `GetElementPtr`: Used in all MIR lowering (expr.rs)
- Typed `GetElementPtrTyped`: Only used in tests and SROA pass
- SROA treats untyped GEP as escape, blocking optimization

**Files to modify:**

#### Phase 1: Update MIR Lowering to Generate Typed GEPs

- [ ] `crates/compiler/mir/src/lowering/expr.rs`
  - Lines ~420-430: Field access - convert to typed GEP
  - Lines ~450-460: Array/index access - keep untyped for now
  - Lines ~520-530: Tuple indexing - convert to typed GEP
  - Lines ~680-690: Struct construction - convert to typed GEP

**Implementation for field access:**

```rust
// Replace untyped:
Instruction::get_element_ptr(dest, object_addr, field_offset)
// With typed:
Instruction::get_element_ptr_typed(
    dest,
    object_addr,
    vec![AccessPath::Field(field_name.clone())],
    object_type
)
```

#### Phase 2: Update Mem2Reg Pass

- [ ] `crates/compiler/mir/src/passes/mem2reg_ssa.rs`
  - Add handling for `GetElementPtrTyped` alongside untyped
  - Track typed paths in addition to offsets

#### Phase 3: Remove Untyped GEP

- [ ] After all uses migrated, remove `GetElementPtr` from `InstructionKind`
- [ ] Update validation pass warnings (lines 337-354 in passes.rs)

### Task 2: Lower aggregates to SSA

**Current Memory-Heavy Approach:**

- Struct/tuple literals use `frame_alloc` + stores
- Member/tuple access uses GEP + load
- SSA instructions (`BuildStruct`, `ExtractValue`) defined but unused

**Files to modify:**

#### Struct Literal Lowering

- [ ] `crates/compiler/mir/src/lowering/expr.rs` lines 684-763

**Replace current approach:**

```rust
// OLD: Memory-based
let struct_addr = /* ... */;
self.instr().add_instruction(
    Instruction::frame_alloc(struct_addr, struct_type.clone())
);
// Store each field...

// NEW: SSA-based
let struct_val = self.state.mir_function.new_typed_value_id(struct_type.clone());
let fields: Vec<(String, Value)> = /* collect field values */;
self.instr().add_instruction(
    Instruction::build_struct(struct_val, struct_type, fields)
);
return Ok(Value::Operand(struct_val));
```

#### Tuple Literal Lowering

- [ ] `crates/compiler/mir/src/lowering/expr.rs` lines 765-845

**For non-empty tuples:**

```rust
let tuple_val = self.state.mir_function.new_typed_value_id(tuple_type.clone());
let elements: Vec<Value> = /* collect element values */;
self.instr().add_instruction(
    Instruction::build_tuple(tuple_val, elements, tuple_type)
);
return Ok(Value::Operand(tuple_val));
```

#### Member Access

- [ ] `crates/compiler/mir/src/lowering/expr.rs` lines 446-519

**When base is SSA value:**

```rust
// Check if base is SSA aggregate
if !base_type.is_pointer() {
    let extracted_val = self.state.mir_function.new_typed_value_id(field_type);
    self.instr().add_instruction(
        Instruction::extract_value(
            extracted_val,
            base_value,
            vec![AccessPath::Field(field_name.clone())],
            field_type
        )
    );
    return Ok(Value::Operand(extracted_val));
}
// Otherwise use current address+load approach
```

#### Tuple Index Access

- [ ] `crates/compiler/mir/src/lowering/expr.rs` lines 847-915

**Similar pattern - check for SSA tuple first**

#### Codegen Support

- [ ] `crates/compiler/codegen/src/generator.rs` lines 545-596
  - Implement `BuildStruct`/`BuildTuple` codegen
  - Implement `ExtractValue` codegen
  - Currently these panic with "not yet implemented"

### Task 3: Fix empty tuple representation

**Current Bug:** Empty tuples return `Value::integer(0)` (line 774)

- [ ] `crates/compiler/mir/src/lowering/expr.rs` line 774

**Fix:**

```rust
// OLD:
if elements.is_empty() {
    return Ok(Value::integer(0)); // WRONG - type confusion
}

// NEW:
if elements.is_empty() {
    // Option 1: Add Value::unit()
    return Ok(Value::unit());

    // Option 2: Use empty BuildTuple
    let unit_val = self.state.mir_function.new_typed_value_id(MirType::Unit);
    self.instr().add_instruction(
        Instruction::build_tuple(unit_val, vec![], MirType::Unit)
    );
    return Ok(Value::Operand(unit_val));
}
```

- [ ] Add `MirType::Unit` if not present in `crates/compiler/mir/src/types.rs`
- [ ] Add `Value::unit()` helper method

### Task 4: Deduplicate helper functions

**Duplicates Found:**

- Type resolution: `get_expr_type()` vs `get_expression_type()` (both unused!)
- Callee resolution: `resolve_callee_expression()` vs `resolve_function()`

**Actions:**

#### Remove Unused Type Functions

- [ ] Delete `get_expression_type()` from `utils.rs` lines 24-28
- [ ] Keep `get_expr_type()` in `builder.rs` (has caching)

#### Consolidate Callee Resolution

- [ ] `crates/compiler/mir/src/lowering/stmt.rs` line 368
  - Replace `self.resolve_function(callee)` with
    `self.resolve_callee_expression(callee)`
- [ ] Delete `resolve_function()` from `utils.rs` lines 222-293
- [ ] Enhance error messages in remaining `resolve_callee_expression()`

## Implementation Order

1. Fix empty tuple representation (quick fix)
2. Deduplicate helper functions (cleanup)
3. Migrate to typed GEP for struct/tuple access
4. Implement SSA aggregate lowering with feature flag
5. Add codegen support for SSA aggregates
6. Enable SSA aggregates by default after testing

## Testing Strategy

- Add tests for empty tuple handling
- Test typed GEP with nested structs/tuples
- Verify SROA optimization with typed GEPs
- Test SSA aggregate operations end-to-end
- Performance testing for register pressure
