# Task: Fix SROA and Lowering Typed GEP Mismatch

## Priority

CRITICAL

## Why

The SROA (Scalar Replacement of Aggregates) optimization pass is completely
ineffective due to a critical mismatch between what it expects and what the
lowering phase produces:

1. **Complete Optimization Failure**: SROA expects `GetElementPtrTyped`
   instructions with constant field paths, but lowering only emits untyped
   `GetElementPtr` instructions with numeric offsets
2. **Performance Impact**: Aggregate allocations (structs/tuples) are never
   broken down into scalar allocations, preventing mem2reg from promoting them
   to SSA form
3. **Missed Optimization Opportunities**: The entire SROA → mem2reg optimization
   chain is broken, leaving aggregate operations unoptimized
4. **False Expectation Setting**: SROA exists and appears functional but never
   actually triggers on real programs

### Evidence of the Mismatch

**SROA Expects (from
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/sroa.rs:161-163`):**

```rust
InstructionKind::GetElementPtrTyped {
    dest, base, path, ..
} => {
    // ... processes typed GEPs with FieldPath
}
```

**Lowering Produces (from
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs:169`):**

```rust
Instruction::get_element_ptr(dest, object_addr, field_offset)
```

This fundamental mismatch means SROA's `identify_alloca_candidates()` function
finds aggregate allocations but never finds any `GetElementPtrTyped`
instructions that access their fields, so no allocations are ever split.

## What

Fix the mismatch between SROA expectations and lowering output by implementing
one of two architectural approaches:

### Option A: Update Lowering to Emit Typed GEPs (Recommended)

Change the lowering phase to emit `GetElementPtrTyped` instructions that include
semantic field path information, preserving the original design intent.

### Option B: Update SROA to Handle Untyped GEPs

Modify SROA to work with untyped `GetElementPtr` instructions by reconstructing
field paths from numeric offsets using type layout information.

## How

### Option A: Update Lowering to Emit Typed GEPs (Recommended)

This preserves the original SROA design and provides better semantic information
for optimizations.

#### Changes Required:

**1. Update Member Access in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs`**

Replace untyped GEP calls (lines 169, 498, 748) with typed versions:

```rust
// BEFORE (line 169):
self.instr().add_instruction(
    Instruction::get_element_ptr(dest, object_addr, field_offset)
        .with_comment(format!("Get address of field '{}'", field.value())),
);

// AFTER:
let field_path = vec![AccessPath::Field(field.value().clone())];
self.instr().add_instruction(
    Instruction::get_element_ptr_typed(dest, object_addr, field_path, object_mir_type)
        .with_comment(format!("Get address of field '{}'", field.value())),
);
```

**2. Update Tuple Index Access**

Replace untyped tuple access (lines 247, 827, 895):

```rust
// BEFORE (line 895):
Instruction::get_element_ptr(element_addr, tuple_addr, Value::integer(offset as i32))

// AFTER:
let tuple_path = vec![AccessPath::TupleIndex(index)];
Instruction::get_element_ptr_typed(element_addr, tuple_addr, tuple_path, tuple_mir_type)
```

**3. Add InstrBuilder Method**

Add to
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/builder/instr_builder.rs`:

```rust
/// Add a typed get_element_ptr instruction
pub fn get_element_ptr_typed(
    &mut self,
    dest: ValueId,
    base: Value,
    path: FieldPath,
    base_type: MirType
) -> &mut Self {
    let instr = Instruction::get_element_ptr_typed(dest, base, path, base_type);
    self.add_instruction(instr);
    self
}
```

**4. Import Required Types**

Add to lowering files:

```rust
use crate::{AccessPath, FieldPath};
```

#### Files to Modify:

- `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs` (8
  locations)
- `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/stmt.rs` (2
  locations)
- `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/builder/instr_builder.rs`
  (add method)

### Option B: Update SROA to Handle Untyped GEPs (Alternative)

Modify SROA to reconstruct field paths from untyped GEP offsets using DataLayout
information.

**Changes in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/sroa.rs`:**

```rust
// Add to identify_alloca_candidates() after line 184:
InstructionKind::GetElementPtr { dest, base, offset } => {
    if let Value::Operand(base_id) = base {
        if let Some(candidate) = candidates.get_mut(base_id) {
            // Reconstruct field path from numeric offset
            if let Value::Literal(Literal::Integer(offset_val)) = offset {
                if let Some(path) = self.reconstruct_field_path(
                    &candidate.aggregate_type,
                    *offset_val as usize
                ) {
                    candidate.typed_geps.insert(*dest, path);
                    candidate.use_blocks.insert(block_id);
                }
            }
        }
    }
}

// Add helper method:
fn reconstruct_field_path(&self, ty: &MirType, offset: usize) -> Option<FieldPath> {
    let layout = DataLayout::new();
    match ty {
        MirType::Struct { fields, .. } => {
            for (field_name, field_type) in fields {
                if let Some(field_offset) = layout.field_offset(ty, field_name) {
                    if field_offset == offset {
                        return Some(vec![AccessPath::Field(field_name.clone())]);
                    }
                }
            }
        }
        MirType::Tuple(types) => {
            for (index, _) in types.iter().enumerate() {
                if let Some(tuple_offset) = layout.tuple_offset(ty, index) {
                    if tuple_offset == offset {
                        return Some(vec![AccessPath::TupleIndex(index)]);
                    }
                }
            }
        }
        _ => {}
    }
    None
}
```

### Recommendation: Option A

Option A is recommended because:

1. **Preserves Design Intent**: The typed GEP infrastructure already exists and
   was designed for SROA
2. **Better Semantic Information**: Field paths provide richer information than
   numeric offsets
3. **Cleaner Architecture**: Separates concerns between lowering (semantic) and
   optimization (mechanical)
4. **Future-Proof**: Enables advanced optimizations that need field-level
   information
5. **Smaller Change Surface**: Leverages existing instruction types rather than
   adding reconstruction logic

## Testing

### Verification Steps

1. **Compilation**: `cargo build -p cairo-m-compiler-mir`
2. **SROA Tests**: `cargo test -p cairo-m-compiler-mir sroa`
3. **Integration**: `cargo test --test mdtest_snapshots`
4. **Snapshot Review**: `cargo insta review`

### Expected Behavior Changes

After fixing the mismatch, SROA should:

- Successfully identify aggregate allocations with typed field accesses
- Split structs/tuples into per-field scalar allocations
- Enable mem2reg to promote field accesses to SSA form
- Show "allocas_split" > 0 in SROA statistics

### Test Cases to Verify

Create test functions with:

```rust
// Should trigger SROA splitting:
fn test_struct_fields() {
    let s = Struct { x: 10, y: 20 };
    let a = s.x;  // Should become direct SSA value
    let b = s.y;  // Should become direct SSA value
}

fn test_tuple_elements() {
    let t = (1, 2, 3);
    let first = t.0;   // Should become direct SSA value
    let second = t.1;  // Should become direct SSA value
}
```

### Manual Verification

1. **Debug Output**: Enable SROA pass debugging to see allocation splitting
2. **MIR Inspection**: Verify that struct/tuple allocations are replaced with
   scalar allocations
3. **Performance**: Measure optimization impact on aggregate-heavy code

## Impact

### Performance Improvements Expected

- **Memory Operations**: 30-50% reduction in load/store instructions for
  aggregate types
- **Register Promotion**: Field accesses become direct SSA values instead of
  memory operations
- **Optimization Chain**: Enables subsequent passes (constant propagation, dead
  code elimination) to work on scalar values
- **Code Quality**: Generated CASM will have significantly fewer memory
  operations

### Optimization Pipeline Restoration

This fix restores the intended optimization flow:

```
Source → Lowering → SROA (split aggregates) → mem2reg (promote to SSA) → other passes
```

Currently broken at the SROA step, preventing the entire aggregate optimization
chain.

### Risk Assessment

- **Low Risk**: Option A leverages existing, tested instruction types
- **High Impact**: Fixes a fundamental optimization failure
- **Testable**: Changes are observable through MIR inspection and performance
  measurements
- **Reversible**: Can revert to untyped GEPs if issues arise

This is a critical architectural fix that will significantly improve the
optimizer's effectiveness on aggregate types, which are common in real Cairo-M
programs.
