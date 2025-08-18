# Fix Mem2Reg GEP Tracking for Field Promotion

## Problem Analysis

### Current State

The `mem2reg_ssa.rs` pass currently has a **critical bug** in GEP
(GetElementPtr) tracking for field promotion. The `PromotableAllocation` struct
has a `gep_values` field that is designed to track GEP instructions derived from
allocations:

```rust
struct PromotableAllocation {
    alloc_id: ValueId,
    ty: MirType,
    store_blocks: FxHashSet<BasicBlockId>,
    gep_values: FxHashMap<ValueId, i32>, // Maps GEP result to constant offset
}
```

However, the `gep_values` map is **never populated** in the current
implementation. This leads to several issues:

1. **Conservative GEP Handling**: Lines 132-146 in
   `identify_promotable_allocations()` mark any allocation with GEP usage as
   escaping, preventing promotion entirely.
2. **Unused GEP Tracking**: The rename phase (lines 394-408 and 429-444) has
   code to handle GEP-derived addresses but `gep_values` is always empty.
3. **Missing Field-Level Promotion**: Multi-field types (structs, tuples,
   multi-slot types like U32) cannot be promoted because per-field value
   tracking is incomplete.

### Identified Problems

1. **GEP Value Map Never Populated**: In `identify_promotable_allocations()`,
   when processing `GetElementPtr` instructions, the code only marks allocations
   as escaping but never tracks the GEP results with their offsets.

2. **Conservative Fallback**: The current implementation conservatively disables
   promotion for any allocation that has GEP usage (lines 137-138):

   ```rust
   if allocations.contains_key(base_id) {
       escaping.insert(*base_id);
   }
   ```

3. **Inconsistent GEP Handling**: The rename phase expects `gep_values` to be
   populated but it never is, so GEP-based stores/loads are never optimized.

4. **Limited Type Support**: Only scalar types (Felt, Bool, Pointer, U32 when
   accessed as whole) are supported, but U32 with GEP access should also be
   promotable.

## Step-by-Step Implementation Plan

### Step 1: Populate `gep_values` Map During Allocation Identification

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa.rs`  
**Location**: Lines 132-148 in `identify_promotable_allocations()`

**Current code**:

```rust
InstructionKind::GetElementPtr { base, .. } => {
    // CONSERVATIVE FIX: Any GEP use disqualifies the allocation
    // Proper per-location phi insertion is needed to handle GEPs correctly
    if let Value::Operand(base_id) = base {
        // Mark the base allocation as escaping if it's tracked
        if allocations.contains_key(base_id) {
            escaping.insert(*base_id);
        }
        // Also check for chained GEPs (GEP of GEP)
        for alloc in allocations.values() {
            if alloc.gep_values.contains_key(base_id) {
                escaping.insert(alloc.alloc_id);
            }
        }
    }
}
```

**Replace with**:

```rust
InstructionKind::GetElementPtr { dest, base, offset } => {
    if let Value::Operand(base_id) = base {
        // Check if this is a GEP from a tracked allocation
        if let Some(alloc) = allocations.get_mut(base_id) {
            // Extract constant offset if possible
            if let Value::Literal(Literal::Integer(offset_val)) = offset {
                // Track this GEP result with its offset
                alloc.gep_values.insert(*dest, *offset_val);
            } else {
                // Non-constant offset - mark as escaping for now
                // TODO: Support variable offsets in the future
                escaping.insert(*base_id);
            }
        } else {
            // Check for chained GEPs (GEP of GEP)
            for alloc in allocations.values_mut() {
                if let Some(&base_offset) = alloc.gep_values.get(base_id) {
                    // This is a chained GEP
                    if let Value::Literal(Literal::Integer(offset_val)) = offset {
                        // Calculate combined offset
                        let combined_offset = base_offset + offset_val;
                        alloc.gep_values.insert(*dest, combined_offset);
                    } else {
                        // Non-constant chained offset - mark as escaping
                        escaping.insert(alloc.alloc_id);
                    }
                    break;
                }
            }
        }
    }
}
```

### Step 2: Enable Multi-Field Type Promotion

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa.rs`  
**Location**: Lines 111-129 in `identify_promotable_allocations()`

**Current code**:

```rust
if matches!(
    ty,
    MirType::Felt | MirType::Bool | MirType::Pointer(_) | MirType::U32
) {
    // Scalar types can be safely promoted when accessed as whole values
    allocations.insert(
        *dest,
        PromotableAllocation {
            alloc_id: *dest,
            ty: ty.clone(),
            store_blocks: FxHashSet::default(),
            gep_values: FxHashMap::default(),
        },
    );
} else {
    // Other multi-slot types not yet supported
    // TODO: Implement SROA or per-slot phi insertion for full support
    escaping.insert(*dest);
}
```

**Replace with**:

```rust
// Allow promotion of types that can benefit from field-level tracking
match ty {
    // Single-slot types that can be promoted as whole values
    MirType::Felt | MirType::Bool | MirType::Pointer(_) => {
        allocations.insert(
            *dest,
            PromotableAllocation {
                alloc_id: *dest,
                ty: ty.clone(),
                store_blocks: FxHashSet::default(),
                gep_values: FxHashMap::default(),
            },
        );
    }
    // Multi-slot types that can be promoted with per-field tracking
    MirType::U32 | MirType::Tuple(_) | MirType::Struct { .. } => {
        allocations.insert(
            *dest,
            PromotableAllocation {
                alloc_id: *dest,
                ty: ty.clone(),
                store_blocks: FxHashSet::default(),
                gep_values: FxHashMap::default(),
            },
        );
    }
    // Types not yet supported for promotion
    MirType::Array { .. } | MirType::Unknown | MirType::Unit => {
        escaping.insert(*dest);
    }
}
```

### Step 3: Fix Phi Node Insertion for Per-Field Values

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa.rs`  
**Location**: Lines 250-297 in `insert_phi_nodes()`

**Current implementation** only creates phi nodes for offset 0. We need to
create phi nodes for all accessed offsets.

**Add after line 294**:

```rust
// For multi-field allocations, we need phi nodes for each accessed offset
for &offset in alloc.gep_values.values() {
    if offset != 0 {
        // Create separate phi nodes for non-zero offsets
        let mut offset_phi_blocks = FxHashSet::default();
        let mut offset_worklist = alloc.store_blocks.clone();
        let mut offset_processed = FxHashSet::default();

        // Find phi locations for this offset (same algorithm as base allocation)
        while let Some(block) = offset_worklist.iter().next().cloned() {
            offset_worklist.remove(&block);
            if offset_processed.insert(block) {
                if let Some(frontier) = dom_frontiers.get(&block) {
                    for &frontier_block in frontier {
                        if offset_phi_blocks.insert(frontier_block) {
                            offset_worklist.insert(frontier_block);
                        }
                    }
                }
            }
        }

        // Create Phi nodes for this offset
        for &phi_block_id in &offset_phi_blocks {
            // Use the element type, not the allocation type
            let element_ty = match &alloc.ty {
                MirType::U32 => MirType::felt(), // U32 elements are felt
                MirType::Tuple(elements) => {
                    elements.get(offset as usize).cloned().unwrap_or(MirType::felt())
                }
                MirType::Struct { fields, .. } => {
                    fields.get(offset as usize).map(|(_, ty)| ty.clone()).unwrap_or(MirType::felt())
                }
                _ => MirType::felt(), // Default to felt for other cases
            };

            let phi_dest = function.new_typed_value_id(element_ty.clone());

            // Store in a per-offset phi map (we'll need to modify the function signature)
            // For now, use a different key format to distinguish offsets
            let offset_key = format!("{}_{}", alloc.alloc_id.as_raw(), offset);
            // This requires refactoring the phi_locations structure

            // Insert empty Phi at the beginning of the block
            let phi_instr = Instruction::phi(phi_dest, element_ty, Vec::new());
            let block = &mut function.basic_blocks[phi_block_id];
            block.instructions.insert(0, phi_instr);

            self.stats.phi_nodes_inserted += 1;
        }
    }
}
```

### Step 4: Update Value Stack Initialization

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa.rs`  
**Location**: Lines 319-328 in `rename_variables()`

**Current code**:

```rust
// Initialize value stacks for each allocation and offset
// Maps (alloc_id, offset) -> stack of values
let mut value_stacks: FxHashMap<(ValueId, i32), Vec<Value>> = FxHashMap::default();
for alloc in promotable {
    // Initialize stacks for all known offsets (from GEPs)
    value_stacks.insert((alloc.alloc_id, 0), Vec::new());
    for &offset in alloc.gep_values.values() {
        value_stacks.insert((alloc.alloc_id, offset), Vec::new());
    }
}
```

This code is correct but needs the GEP values to be populated first (from Step
1).

### Step 5: Handle Composite Type Stores

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa.rs**   **Location**: Lines 157-189 in `identify_promotable_allocations()`

**Current logic** marks allocations as escaping if they have composite stores.
This needs refinement:

**Replace lines 169-172**:

```rust
if matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }) {
    // Mark as escaping - cannot be promoted with composite stores
    escaping.insert(*addr_id);
}
```

**With**:

```rust
if matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }) {
    // Composite stores to multi-field types are problematic
    // They store entire structures, but we track individual fields
    // For now, mark as escaping. Future enhancement could decompose
    // composite stores into individual field stores.
    escaping.insert(*addr_id);
}
```

## Test Cases to Verify the Fix

### Test Case 1: U32 with GEP Promotion

```rust
#[test]
fn test_u32_gep_promotion_fixed() {
    let mut function = MirFunction::new("test_u32_gep_fixed");
    let entry = function.entry_block;

    // %0 = framealloc u32
    let u32_alloc = function.new_typed_value_id(MirType::pointer(MirType::u32()));

    // %1 = getelementptr %0, 0
    let low_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // %2 = getelementptr %0, 1
    let high_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // Store to both parts
    function.basic_blocks[entry].instructions.extend([
        Instruction::frame_alloc(u32_alloc, MirType::u32()),
        Instruction::get_element_ptr(low_ptr, Value::operand(u32_alloc), Value::integer(0)),
        Instruction::get_element_ptr(high_ptr, Value::operand(u32_alloc), Value::integer(1)),
        Instruction::store(Value::operand(low_ptr), Value::integer(100), MirType::felt()),
        Instruction::store(Value::operand(high_ptr), Value::integer(200), MirType::felt()),
    ]);

    // Load from both parts
    let low_loaded = function.new_typed_value_id(MirType::felt());
    let high_loaded = function.new_typed_value_id(MirType::felt());

    function.basic_blocks[entry].instructions.extend([
        Instruction::load(low_loaded, MirType::felt(), Value::operand(low_ptr)),
        Instruction::load(high_loaded, MirType::felt(), Value::operand(high_ptr)),
    ]);

    function.basic_blocks[entry].terminator = Terminator::Return {
        values: vec![Value::operand(low_loaded), Value::operand(high_loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // Should be promoted with per-field tracking
    assert!(changed, "U32 with GEP should now be promotable");
    assert_eq!(pass.stats.allocations_promoted, 1);
    assert_eq!(pass.stats.stores_eliminated, 2);
    assert_eq!(pass.stats.loads_eliminated, 2);
}
```

### Test Case 2: Struct Field Promotion

```rust
#[test]
fn test_struct_field_promotion() {
    let mut function = MirFunction::new("test_struct_promotion");
    let entry = function.entry_block;

    let struct_ty = MirType::struct_type(
        "Point".to_string(),
        vec![("x".to_string(), MirType::felt()), ("y".to_string(), MirType::felt())],
    );

    let struct_alloc = function.new_typed_value_id(MirType::pointer(struct_ty.clone()));
    let x_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let y_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    function.basic_blocks[entry].instructions.extend([
        Instruction::frame_alloc(struct_alloc, struct_ty),
        Instruction::get_element_ptr(x_ptr, Value::operand(struct_alloc), Value::integer(0)),
        Instruction::get_element_ptr(y_ptr, Value::operand(struct_alloc), Value::integer(1)),
        Instruction::store(Value::operand(x_ptr), Value::integer(10), MirType::felt()),
        Instruction::store(Value::operand(y_ptr), Value::integer(20), MirType::felt()),
    ]);

    let x_loaded = function.new_typed_value_id(MirType::felt());
    let y_loaded = function.new_typed_value_id(MirType::felt());

    function.basic_blocks[entry].instructions.extend([
        Instruction::load(x_loaded, MirType::felt(), Value::operand(x_ptr)),
        Instruction::load(y_loaded, MirType::felt(), Value::operand(y_ptr)),
    ]);

    function.basic_blocks[entry].terminator = Terminator::Return {
        values: vec![Value::operand(x_loaded), Value::operand(y_loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // Should be promoted with field-level tracking
    assert!(changed, "Struct with field access should be promotable");
    assert_eq!(pass.stats.allocations_promoted, 1);
}
```

### Test Case 3: Chained GEP Support

```rust
#[test]
fn test_chained_gep_promotion() {
    let mut function = MirFunction::new("test_chained_gep");
    let entry = function.entry_block;

    let u32_alloc = function.new_typed_value_id(MirType::pointer(MirType::u32()));
    let base_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let offset_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    function.basic_blocks[entry].instructions.extend([
        Instruction::frame_alloc(u32_alloc, MirType::u32()),
        // %1 = getelementptr %0, 0  (base_ptr points to offset 0)
        Instruction::get_element_ptr(base_ptr, Value::operand(u32_alloc), Value::integer(0)),
        // %2 = getelementptr %1, 1  (offset_ptr points to offset 0+1=1)
        Instruction::get_element_ptr(offset_ptr, Value::operand(base_ptr), Value::integer(1)),
        Instruction::store(Value::operand(offset_ptr), Value::integer(42), MirType::felt()),
    ]);

    let loaded = function.new_typed_value_id(MirType::felt());
    function.basic_blocks[entry].instructions.push(
        Instruction::load(loaded, MirType::felt(), Value::operand(offset_ptr))
    );

    function.basic_blocks[entry].terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };

    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    assert!(changed, "Chained GEP should be handled correctly");
}
```

## Implementation Notes

1. **Gradual Rollout**: Implement Step 1 first and test with existing cases to
   ensure no regressions.

2. **Error Handling**: Non-constant offsets should continue to mark allocations
   as escaping until variable offset support is added.

3. **Type Safety**: Ensure phi node types match the element types being
   accessed, not the allocation types.

4. **Performance**: The current approach creates separate phi nodes for each
   offset. Future optimization could use SROA (Scalar Replacement of Aggregates)
   for better performance.

5. **Backward Compatibility**: Existing working cases (scalar promotion without
   GEP) should continue to work unchanged.

## Files to Modify

1. **Primary**:
   `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa.rs`
2. **Tests**:
   `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mem2reg_ssa_tests.rs`

## Expected Results

After implementing this fix:

1. **U32 Types**: Can be promoted even when accessed via GEP to individual limbs
2. **Struct Types**: Can be promoted when accessed via field GEPs
3. **Tuple Types**: Can be promoted when accessed via element GEPs
4. **Performance**: Significant improvement for code that uses struct/tuple
   fields
5. **Correctness**: Proper SSA form with per-field phi nodes

The fix transforms the mem2reg pass from a basic scalar-only optimization to a
full aggregate-aware optimization that can handle field-level promotion
correctly.
