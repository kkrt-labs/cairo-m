# Optimize InsertField and InsertTuple Operations with In-Place Updates

## Problem Analysis

### Current State

The `LowerAggregatesPass` in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs`
currently implements a **copy-on-write strategy** for `InsertField` and
`InsertTuple` operations. This approach is overly conservative and generates
inefficient code:

#### Current Copy-on-Write Implementation

```rust
InstructionKind::InsertField { dest, struct_val, field_name, new_value, struct_ty } => {
    // Always allocate a new struct
    let new_alloca = function.new_typed_value_id(MirType::pointer(struct_ty.clone()));

    // Copy ALL fields from old struct to new struct
    if let MirType::Struct { fields, .. } = struct_ty {
        for (fname, ftype) in fields {
            if fname == field_name { continue; }
            // Load from old, store to new - expensive!
        }
    }

    // Store the updated field value
    // ... more code to store new field
}
```

#### Performance Problems

1. **Always Allocates**: Creates a new allocation even when the original
   aggregate is never used again
2. **Copies All Fields**: Generates load/store pairs for every unchanged field
   in structs and tuples
3. **Memory Overhead**: Creates temporary allocations that consume frame space
4. **Instruction Bloat**: A single `InsertField` can generate 10+ instructions
   for a 5-field struct

### Use Count Analysis Available

The MIR framework provides `function.get_value_use_counts()` which returns a
`FxHashMap<ValueId, usize>` mapping each value to its usage count. This is
perfect for detecting single-use aggregates.

#### Usage Pattern Analysis

```rust
pub fn get_value_use_counts(&self) -> FxHashMap<ValueId, usize> {
    let mut counts = FxHashMap::default();
    for (_id, block) in self.basic_blocks() {
        for instruction in &block.instructions {
            for used_value in instruction.used_values() {
                *counts.entry(used_value).or_default() += 1;
            }
        }
        // Also count terminator usage...
    }
    counts
}
```

#### InsertField/InsertTuple Used Values

From `instruction.rs`, both operations track their input aggregate:

```rust
InstructionKind::InsertField { struct_val, new_value, .. } => {
    if let Value::Operand(id) = struct_val { used.insert(*id); }
    if let Value::Operand(id) = new_value { used.insert(*id); }
}

InstructionKind::InsertTuple { tuple_val, new_value, .. } => {
    if let Value::Operand(id) = tuple_val { used.insert(*id); }
    if let Value::Operand(id) = new_value { used.insert(*id); }
}
```

## Conditions for Safe In-Place Updates

### Primary Condition: Single-Use Aggregates

An aggregate can be updated in-place if its **use count equals 1**, meaning:

- Only the `InsertField`/`InsertTuple` instruction uses the aggregate
- No other instructions read from the original aggregate after the update
- The original aggregate's memory can be safely reused

### Safety Validation Requirements

1. **Use Count Check**:
   `use_counts.get(&aggregate_id).copied().unwrap_or(0) == 1`
2. **Memory Allocation Exists**: The aggregate must have been lowered and have
   an entry in `aggregate_allocas`
3. **Valid Field/Index**: The field name or tuple index must be valid for the
   aggregate type
4. **Type Compatibility**: The new value type must match the expected
   field/element type

### Edge Cases to Handle

1. **Zero Use Count**: Aggregate created but never used → should be eliminated
   by dead code elimination
2. **Multiple Use Count**: Aggregate used by other instructions → must use
   copy-on-write
3. **Chained Operations**: Multiple inserts on the same aggregate → only the
   first can be in-place if it becomes single-use
4. **Complex Control Flow**: Aggregate used across basic blocks → requires
   careful analysis

## Step-by-Step Implementation Plan

### Step 1: Modify `lower_instruction` Function Signature

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs`  
**Location**:
Lines 278-283

**Current signature**:

```rust
fn lower_instruction(
    &mut self,
    instruction: &Instruction,
    function: &mut MirFunction,
) -> Vec<Instruction>
```

**Updated signature**:

```rust
fn lower_instruction(
    &mut self,
    instruction: &Instruction,
    function: &mut MirFunction,
    use_counts: &FxHashMap<ValueId, usize>,
) -> Vec<Instruction>
```

### Step 2: Compute Use Counts in Pass Entry Point

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs`  
**Location**:
Lines 568-605 in `impl MirPass for LowerAggregatesPass`

**Current code**:

```rust
impl MirPass for LowerAggregatesPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Clone instructions to avoid borrow issues
        let blocks_instructions: Vec<Vec<Instruction>> = function
            .basic_blocks
            .iter()
            .map(|b| b.instructions.clone())
            .collect();

        // Process each block's instructions
        let mut blocks_new_instructions = Vec::new();
        for block_instructions in blocks_instructions {
            let mut new_instructions = Vec::new();

            for instruction in block_instructions {
                let lowered = self.lower_instruction(&instruction, function);
                // ... rest of processing
```

**Updated code**:

```rust
impl MirPass for LowerAggregatesPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Compute use counts before processing instructions
        let use_counts = function.get_value_use_counts();

        // Clone instructions to avoid borrow issues
        let blocks_instructions: Vec<Vec<Instruction>> = function
            .basic_blocks
            .iter()
            .map(|b| b.instructions.clone())
            .collect();

        // Process each block's instructions
        let mut blocks_new_instructions = Vec::new();
        for block_instructions in blocks_instructions {
            let mut new_instructions = Vec::new();

            for instruction in block_instructions {
                let lowered = self.lower_instruction(&instruction, function, &use_counts);
                if lowered.len() != 1 || !lowered[0].kind.eq(&instruction.kind) {
                    modified = true;
                }
                new_instructions.extend(lowered);
            }

            blocks_new_instructions.push(new_instructions);
        }

        // Apply changes...
```

### Step 3: Add In-Place Update Helper Methods

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs**   **Location**: Add after line 276 (before `lower_instruction`)

```rust
/// Create in-place field update for single-use struct
fn create_in_place_field_update(
    &mut self,
    dest: ValueId,
    struct_val: &Value,
    field_name: &str,
    new_value: Value,
    struct_ty: &MirType,
    function: &mut MirFunction,
) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    if let Some(struct_val_id) = Self::extract_value_id(struct_val) {
        if let Some(&existing_alloca) = self.aggregate_allocas.get(&struct_val_id) {
            // Reuse existing allocation - no new framealloc needed
            let layout = DataLayout::new();

            // Get field offset and type
            if let Some(offset) = layout.field_offset(struct_ty, field_name) {
                let field_type = if let MirType::Struct { fields, .. } = struct_ty {
                    fields.iter()
                        .find(|(name, _)| name == field_name)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(MirType::Unknown)
                } else {
                    MirType::Unknown
                };

                // Update the specific field in-place
                let field_ptr = function.new_typed_value_id(MirType::pointer(field_type.clone()));
                instructions.push(
                    Instruction::get_element_ptr(
                        field_ptr,
                        Value::operand(existing_alloca),
                        Value::integer(offset as i32),
                    )
                    .with_comment(format!("Get address of field '{}' for in-place update", field_name))
                );
                instructions.push(
                    Instruction::store(Value::operand(field_ptr), new_value, field_type)
                        .with_comment(format!("In-place update field '{}'", field_name))
                );

                // Reuse existing alloca for destination
                self.aggregate_allocas.insert(dest, existing_alloca);
                instructions.push(
                    Instruction::assign(
                        dest,
                        Value::operand(existing_alloca),
                        MirType::pointer(struct_ty.clone()),
                    )
                    .with_comment("Reuse existing struct alloca for in-place update".to_string())
                );
            }
        }
    }

    instructions
}

/// Create in-place tuple element update for single-use tuple
fn create_in_place_tuple_update(
    &mut self,
    dest: ValueId,
    tuple_val: &Value,
    index: usize,
    new_value: Value,
    tuple_ty: &MirType,
    function: &mut MirFunction,
) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    if let Some(tuple_val_id) = Self::extract_value_id(tuple_val) {
        if let Some(&existing_alloca) = self.aggregate_allocas.get(&tuple_val_id) {
            // Reuse existing allocation
            let layout = DataLayout::new();
            let offset = layout.tuple_offset(tuple_ty, index).unwrap_or(index) as i32;

            // Get element type
            let element_type = if let MirType::Tuple(types) = tuple_ty {
                types.get(index).cloned().unwrap_or(MirType::Unknown)
            } else {
                MirType::Unknown
            };

            // Update the specific element in-place
            let elem_ptr = function.new_typed_value_id(MirType::pointer(element_type.clone()));
            instructions.push(
                Instruction::get_element_ptr(
                    elem_ptr,
                    Value::operand(existing_alloca),
                    Value::integer(offset),
                )
                .with_comment(format!("Get address of tuple element {} for in-place update", index))
            );
            instructions.push(
                Instruction::store(Value::operand(elem_ptr), new_value, element_type)
                    .with_comment(format!("In-place update tuple element {}", index))
            );

            // Reuse existing alloca for destination
            self.aggregate_allocas.insert(dest, existing_alloca);
            instructions.push(
                Instruction::assign(
                    dest,
                    Value::operand(existing_alloca),
                    MirType::pointer(tuple_ty.clone()),
                )
                .with_comment("Reuse existing tuple alloca for in-place update".to_string())
            );
        }
    }

    instructions
}

/// Check if an aggregate can be updated in-place
fn can_update_in_place(
    &self,
    aggregate_val: &Value,
    use_counts: &FxHashMap<ValueId, usize>,
) -> bool {
    if let Some(aggregate_id) = Self::extract_value_id(aggregate_val) {
        // Check if aggregate is single-use and has existing allocation
        let use_count = use_counts.get(&aggregate_id).copied().unwrap_or(0);
        let has_allocation = self.aggregate_allocas.contains_key(&aggregate_id);

        use_count == 1 && has_allocation
    } else {
        false
    }
}
```

### Step 4: Update InsertField Handling with Optimization

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs**   **Location**: Lines 330-446 (InsertField case in `lower_instruction`)

**Replace the entire InsertField case**:

```rust
InstructionKind::InsertField {
    dest,
    struct_val,
    field_name,
    new_value,
    struct_ty,
} => {
    // Check if we can optimize with in-place update
    if self.can_update_in_place(struct_val, use_counts) {
        // Single-use aggregate - update in-place
        self.create_in_place_field_update(
            *dest, struct_val, field_name, *new_value, struct_ty, function
        )
    } else {
        // Multiple-use aggregate - fall back to copy-on-write
        // (Keep existing copy-on-write implementation)
        let mut instructions = Vec::new();

        // Allocate new struct
        let new_alloca = function.new_typed_value_id(MirType::pointer(struct_ty.clone()));
        instructions.push(
            Instruction::frame_alloc(new_alloca, struct_ty.clone())
                .with_comment("Alloca for copied struct (multi-use case)".to_string()),
        );

        // Copy old struct contents if we have an existing alloca
        if let Some(value_id) = Self::extract_value_id(struct_val) {
            if let Some(&old_alloca) = self.aggregate_allocas.get(&value_id) {
                // Copy each field from old to new (existing logic)
                if let MirType::Struct { fields, .. } = struct_ty {
                    let layout = DataLayout::new();
                    for (fname, ftype) in fields {
                        if fname == field_name {
                            continue; // Skip the field we're updating
                        }
                        let offset = layout.field_offset(struct_ty, fname).unwrap_or(0) as i32;

                        // Load from old
                        let old_field_ptr = function.new_typed_value_id(MirType::pointer(ftype.clone()));
                        instructions.push(
                            Instruction::get_element_ptr(
                                old_field_ptr,
                                Value::operand(old_alloca),
                                Value::integer(offset),
                            )
                            .with_comment(format!("Get old field '{}' for copy", fname)),
                        );
                        let field_value = function.new_typed_value_id(ftype.clone());
                        instructions.push(
                            Instruction::load(
                                field_value,
                                ftype.clone(),
                                Value::operand(old_field_ptr),
                            )
                            .with_comment(format!("Load old field '{}' for copy", fname)),
                        );

                        // Store to new
                        let new_field_ptr = function.new_typed_value_id(MirType::pointer(ftype.clone()));
                        instructions.push(
                            Instruction::get_element_ptr(
                                new_field_ptr,
                                Value::operand(new_alloca),
                                Value::integer(offset),
                            )
                            .with_comment(format!("Get new field '{}' location for copy", fname)),
                        );
                        instructions.push(
                            Instruction::store(
                                Value::operand(new_field_ptr),
                                Value::operand(field_value),
                                ftype.clone(),
                            )
                            .with_comment(format!("Copy field '{}'", fname)),
                        );
                    }
                }
            }
        }

        // Now store the updated field value
        let layout = DataLayout::new();
        if let Some(offset) = layout.field_offset(struct_ty, field_name) {
            let field_type = if let MirType::Struct { fields, .. } = struct_ty {
                fields.iter()
                    .find(|(name, _)| name == field_name)
                    .map(|(_, ty)| ty.clone())
                    .unwrap_or(MirType::Unknown)
            } else {
                MirType::Unknown
            };

            let field_ptr = function.new_typed_value_id(MirType::pointer(field_type.clone()));
            instructions.push(
                Instruction::get_element_ptr(
                    field_ptr,
                    Value::operand(new_alloca),
                    Value::integer(offset as i32),
                )
                .with_comment(format!("Get address of field '{}' for update", field_name)),
            );
            instructions.push(
                Instruction::store(Value::operand(field_ptr), *new_value, field_type)
                    .with_comment(format!("Update field '{}'", field_name)),
            );
        }

        self.aggregate_allocas.insert(*dest, new_alloca);
        instructions.push(
            Instruction::assign(
                *dest,
                Value::operand(new_alloca),
                MirType::pointer(struct_ty.clone()),
            )
            .with_comment("Alias copied struct to alloca".to_string()),
        );

        instructions
    }
}
```

### Step 5: Update InsertTuple Handling with Optimization

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs**   **Location**: Lines 448-560 (InsertTuple case in `lower_instruction`)

**Replace the entire InsertTuple case**:

```rust
InstructionKind::InsertTuple {
    dest,
    tuple_val,
    index,
    new_value,
    tuple_ty,
} => {
    // Check if we can optimize with in-place update
    if self.can_update_in_place(tuple_val, use_counts) {
        // Single-use aggregate - update in-place
        self.create_in_place_tuple_update(
            *dest, tuple_val, *index, *new_value, tuple_ty, function
        )
    } else {
        // Multiple-use aggregate - fall back to copy-on-write
        // (Keep existing copy-on-write implementation)
        let mut instructions = Vec::new();

        // Allocate new tuple
        let new_alloca = function.new_typed_value_id(MirType::pointer(tuple_ty.clone()));
        instructions.push(
            Instruction::frame_alloc(new_alloca, tuple_ty.clone())
                .with_comment("Alloca for copied tuple (multi-use case)".to_string()),
        );

        // Copy old tuple contents if we have an existing alloca
        if let Some(value_id) = Self::extract_value_id(tuple_val) {
            if let Some(&old_alloca) = self.aggregate_allocas.get(&value_id) {
                // Copy each element from old to new (existing logic)
                if let MirType::Tuple(types) = tuple_ty {
                    let layout = DataLayout::new();
                    for (i, elem_type) in types.iter().enumerate() {
                        if i == *index {
                            continue; // Skip the element we're updating
                        }
                        let offset = layout.tuple_offset(tuple_ty, i).unwrap_or(i) as i32;

                        // Load from old
                        let old_elem_ptr = function.new_typed_value_id(MirType::pointer(elem_type.clone()));
                        instructions.push(
                            Instruction::get_element_ptr(
                                old_elem_ptr,
                                Value::operand(old_alloca),
                                Value::integer(offset),
                            )
                            .with_comment(format!("Get old element {} for copy", i)),
                        );
                        let elem_value = function.new_typed_value_id(elem_type.clone());
                        instructions.push(
                            Instruction::load(
                                elem_value,
                                elem_type.clone(),
                                Value::operand(old_elem_ptr),
                            )
                            .with_comment(format!("Load old element {} for copy", i)),
                        );

                        // Store to new
                        let new_elem_ptr = function.new_typed_value_id(MirType::pointer(elem_type.clone()));
                        instructions.push(
                            Instruction::get_element_ptr(
                                new_elem_ptr,
                                Value::operand(new_alloca),
                                Value::integer(offset),
                            )
                            .with_comment(format!("Get new element {} location for copy", i)),
                        );
                        instructions.push(
                            Instruction::store(
                                Value::operand(new_elem_ptr),
                                Value::operand(elem_value),
                                elem_type.clone(),
                            )
                            .with_comment(format!("Copy element {}", i)),
                        );
                    }
                }
            }
        }

        // Now store the updated element value
        let element_type = if let MirType::Tuple(types) = tuple_ty {
            types.get(*index).cloned().unwrap_or(MirType::Unknown)
        } else {
            MirType::Unknown
        };

        let layout = DataLayout::new();
        let offset = layout.tuple_offset(tuple_ty, *index).unwrap_or(*index) as i32;

        let elem_ptr = function.new_typed_value_id(MirType::pointer(element_type.clone()));
        instructions.push(
            Instruction::get_element_ptr(
                elem_ptr,
                Value::operand(new_alloca),
                Value::integer(offset),
            )
            .with_comment(format!(
                "Get address (offset {}) of tuple element {} for update",
                offset, index
            )),
        );
        instructions.push(
            Instruction::store(Value::operand(elem_ptr), *new_value, element_type)
                .with_comment(format!("Update tuple element {}", index)),
        );

        self.aggregate_allocas.insert(*dest, new_alloca);
        instructions.push(
            Instruction::assign(
                *dest,
                Value::operand(new_alloca),
                MirType::pointer(tuple_ty.clone()),
            )
            .with_comment("Alias copied tuple to alloca".to_string()),
        );

        instructions
    }
}
```

### Step 6: Update All `lower_instruction` Calls

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs**   **Location**: Line 585 in the `run`
method

**Change**:

```rust
let lowered = self.lower_instruction(&instruction, function);
```

**To**:

```rust
let lowered = self.lower_instruction(&instruction, function, &use_counts);
```

## Test Cases for Verification

### Test Case 1: Single-Use Struct Field Update (Should Be Optimized)

```rust
#[test]
fn test_single_use_struct_field_optimization() {
    let mut function = MirFunction::new("test_single_use_field");
    let mut pass = LowerAggregatesPass::new();

    let struct_ty = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    // Create original struct
    let orig_struct = function.new_typed_value_id(struct_ty.clone());
    // Update field (single use of orig_struct)
    let updated_struct = function.new_typed_value_id(struct_ty.clone());

    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Original struct creation
    block.instructions.push(Instruction::make_struct(
        orig_struct,
        vec![
            ("x".to_string(), Value::integer(10)),
            ("y".to_string(), Value::integer(20)),
        ],
        struct_ty.clone(),
    ));

    // Single field update - should be optimized
    block.instructions.push(Instruction::insert_field(
        updated_struct,
        Value::operand(orig_struct),
        "x".to_string(),
        Value::integer(30),
        struct_ty.clone(),
    ));

    let initial_instruction_count = block.instructions.len();
    let modified = pass.run(&mut function);

    assert!(modified, "Pass should modify the function");

    let final_block = &function.basic_blocks[block_id];

    // Should have fewer instructions due to in-place update
    // Original: MakeStruct + InsertField → framealloc + 2*store + 1*gep + 1*store + assign
    // Optimized: MakeStruct + InsertField → framealloc + 2*store + 1*gep + 1*store + assign
    // (but no copy operations for unchanged fields)

    let frame_alloc_count = final_block.instructions.iter()
        .filter(|i| matches!(i.kind, InstructionKind::FrameAlloc { .. }))
        .count();

    // Should only have one allocation (reused), not two
    assert_eq!(frame_alloc_count, 1, "Should only have one allocation for in-place update");

    // Look for the optimization comment
    let has_in_place_comment = final_block.instructions.iter()
        .any(|i| i.comment.as_ref().map_or(false, |c| c.contains("in-place update")));

    assert!(has_in_place_comment, "Should have in-place update comment");
}
```

### Test Case 2: Multi-Use Struct Field Update (Should Use Copy-on-Write)

```rust
#[test]
fn test_multi_use_struct_field_copy_on_write() {
    let mut function = MirFunction::new("test_multi_use_field");
    let mut pass = LowerAggregatesPass::new();

    let struct_ty = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let orig_struct = function.new_typed_value_id(struct_ty.clone());
    let updated_struct = function.new_typed_value_id(struct_ty.clone());
    let extracted = function.new_typed_value_id(MirType::felt());

    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Original struct creation
    block.instructions.push(Instruction::make_struct(
        orig_struct,
        vec![
            ("x".to_string(), Value::integer(10)),
            ("y".to_string(), Value::integer(20)),
        ],
        struct_ty.clone(),
    ));

    // Use original struct (making it multi-use)
    block.instructions.push(Instruction::extract_struct_field(
        extracted,
        Value::operand(orig_struct),
        "x".to_string(),
        MirType::felt(),
    ));

    // Field update on multi-use struct - should use copy-on-write
    block.instructions.push(Instruction::insert_field(
        updated_struct,
        Value::operand(orig_struct),
        "x".to_string(),
        Value::integer(30),
        struct_ty.clone(),
    ));

    let modified = pass.run(&mut function);
    assert!(modified, "Pass should modify the function");

    let final_block = &function.basic_blocks[block_id];

    // Should have two allocations (original + copy)
    let frame_alloc_count = final_block.instructions.iter()
        .filter(|i| matches!(i.kind, InstructionKind::FrameAlloc { .. }))
        .count();

    assert_eq!(frame_alloc_count, 2, "Should have two allocations for copy-on-write");

    // Should have copy operations for unchanged fields
    let copy_comment_count = final_block.instructions.iter()
        .filter(|i| i.comment.as_ref().map_or(false, |c| c.contains("Copy field")))
        .count();

    assert_eq!(copy_comment_count, 1, "Should copy the unchanged field 'y'");
}
```

### Test Case 3: Single-Use Tuple Element Update

```rust
#[test]
fn test_single_use_tuple_element_optimization() {
    let mut function = MirFunction::new("test_single_use_tuple");
    let mut pass = LowerAggregatesPass::new();

    let tuple_ty = MirType::Tuple(vec![MirType::felt(), MirType::felt(), MirType::felt()]);

    let orig_tuple = function.new_typed_value_id(tuple_ty.clone());
    let updated_tuple = function.new_typed_value_id(tuple_ty.clone());

    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Create original tuple
    block.instructions.push(Instruction::make_tuple(
        orig_tuple,
        vec![Value::integer(1), Value::integer(2), Value::integer(3)],
    ));

    // Single element update - should be optimized
    block.instructions.push(Instruction::insert_tuple(
        updated_tuple,
        Value::operand(orig_tuple),
        1,
        Value::integer(42),
        tuple_ty.clone(),
    ));

    let modified = pass.run(&mut function);
    assert!(modified);

    let final_block = &function.basic_blocks[block_id];

    // Should only have one allocation
    let frame_alloc_count = final_block.instructions.iter()
        .filter(|i| matches!(i.kind, InstructionKind::FrameAlloc { .. }))
        .count();

    assert_eq!(frame_alloc_count, 1, "Should reuse allocation for in-place tuple update");

    // Should have in-place update comment
    let has_in_place_comment = final_block.instructions.iter()
        .any(|i| i.comment.as_ref().map_or(false, |c| c.contains("in-place update")));

    assert!(has_in_place_comment, "Should have in-place update comment");
}
```

### Test Case 4: Chain of Insert Operations

```rust
#[test]
fn test_insert_chain_optimization() {
    let mut function = MirFunction::new("test_insert_chain");
    let mut pass = LowerAggregatesPass::new();

    let struct_ty = MirType::Struct {
        name: "Point3D".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
            ("z".to_string(), MirType::felt()),
        ],
    };

    let struct1 = function.new_typed_value_id(struct_ty.clone());
    let struct2 = function.new_typed_value_id(struct_ty.clone());
    let struct3 = function.new_typed_value_id(struct_ty.clone());

    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Create original struct
    block.instructions.push(Instruction::make_struct(
        struct1,
        vec![
            ("x".to_string(), Value::integer(1)),
            ("y".to_string(), Value::integer(2)),
            ("z".to_string(), Value::integer(3)),
        ],
        struct_ty.clone(),
    ));

    // Chain of updates - each should be optimized if single-use
    block.instructions.push(Instruction::insert_field(
        struct2,
        Value::operand(struct1),
        "x".to_string(),
        Value::integer(10),
        struct_ty.clone(),
    ));

    block.instructions.push(Instruction::insert_field(
        struct3,
        Value::operand(struct2),
        "y".to_string(),
        Value::integer(20),
        struct_ty.clone(),
    ));

    let modified = pass.run(&mut function);
    assert!(modified);

    let final_block = &function.basic_blocks[block_id];

    // Each intermediate result should be optimized (single-use chain)
    let frame_alloc_count = final_block.instructions.iter()
        .filter(|i| matches!(i.kind, InstructionKind::FrameAlloc { .. }))
        .count();

    // Should only have the original allocation, reused throughout the chain
    assert_eq!(frame_alloc_count, 1, "Should reuse allocation through the entire chain");
}
```

## Edge Cases and Safety Considerations

### Edge Case 1: Zero Use Count

```rust
// Aggregate created but never used
let unused_struct = function.new_typed_value_id(struct_ty);
block.instructions.push(Instruction::make_struct(unused_struct, fields, struct_ty));
// No InsertField using unused_struct

// Expected: Dead code elimination should remove this,
// but if it reaches LowerAggregates, use_count will be 0
// Action: Treat as copy-on-write (conservative)
```

### Edge Case 2: Use Count Analysis Timing

```rust
// The use count is computed BEFORE lowering instructions
// If an instruction is lowered and creates new uses, they won't be counted
// This is safe because:
// 1. We only optimize based on the original MIR structure
// 2. New memory instructions don't affect the aggregate use semantics
```

### Edge Case 3: Cross-Block Usage

```rust
// Block 1: Creates struct
let s1 = make_struct(...)

// Block 2: Uses struct in condition
if extract_field(s1, "flag") { ... }

// Block 3: Updates struct
let s2 = insert_field(s1, "value", new_val)

// Expected: s1 has use_count = 2, so copy-on-write is used
// This is correct - we can't do in-place updates across blocks safely
```

### Edge Case 4: Aliasing through Assignment

```rust
let s1 = make_struct(...)
let s1_alias = assign(s1)  // s1_alias = s1
let s2 = insert_field(s1_alias, "field", value)

// Expected: s1 has use_count = 1 (assign), s1_alias has use_count = 1 (insert_field)
// Both can potentially be optimized, but:
// - s1_alias points to same memory as s1
// - Need to ensure aggregate_allocas tracks both correctly
```

### Safety Requirement 1: Allocation Tracking Consistency

The `aggregate_allocas` map must correctly track which ValueIds point to which
memory allocations. When doing in-place updates:

1. The source aggregate must have an existing allocation
2. The destination aggregate must reuse the same allocation
3. The mapping must be updated correctly

### Safety Requirement 2: Type Compatibility

```rust
// Field type must match new value type
let field_type = get_field_type(struct_ty, field_name);
assert!(is_compatible(field_type, new_value_type));

// Tuple element type must match
let element_type = get_element_type(tuple_ty, index);
assert!(is_compatible(element_type, new_value_type));
```

### Safety Requirement 3: Memory Layout Consistency

```rust
// Offsets must be computed consistently
let layout = DataLayout::new();
let offset = layout.field_offset(struct_ty, field_name);
// Must use the same DataLayout instance and calculation method
// throughout the compilation pipeline
```

## Performance Impact Analysis

### Best Case Scenarios

1. **Single Field Updates**: 1 store instead of N loads + N stores (where N =
   field count)
2. **Chained Updates**: Reuses allocation across multiple operations
3. **Large Structures**: Dramatic reduction in memory traffic and instruction
   count

### Example Optimization: 5-Field Struct Update

**Before Optimization** (copy-on-write):

```
%new_alloc = framealloc Point5D
%old_f1_ptr = getelementptr %old_alloc, 0
%old_f1_val = load felt, %old_f1_ptr
%new_f1_ptr = getelementptr %new_alloc, 0
store %new_f1_ptr, %old_f1_val, felt
%old_f2_ptr = getelementptr %old_alloc, 1
%old_f2_val = load felt, %old_f2_ptr
%new_f2_ptr = getelementptr %new_alloc, 1
store %new_f2_ptr, %old_f2_val, felt
// ... repeat for f3, f4 (unchanged fields)
%new_f5_ptr = getelementptr %new_alloc, 4
store %new_f5_ptr, %new_value, felt
// Total: 1 framealloc + 8 gep + 4 load + 5 store = 18 instructions
```

**After Optimization** (in-place update):

```
%field_ptr = getelementptr %existing_alloc, 4
store %field_ptr, %new_value, felt
%dest = assign %existing_alloc
// Total: 1 gep + 1 store + 1 assign = 3 instructions
```

**Improvement**: 83% reduction in instruction count (18 → 3)

### Memory Usage Impact

- **Reduced Frame Allocations**: Reuses existing memory instead of allocating
  new
- **Lower Peak Memory**: Avoids temporary copies during aggregate updates
- **Better Cache Locality**: Updates in-place preserve memory layout

## Implementation Timeline

### Phase 1: Core Infrastructure (Steps 1-3)

- Modify function signatures
- Add use count computation
- Implement helper methods
- **Deliverable**: Infrastructure ready, no behavioral changes yet

### Phase 2: InsertField Optimization (Step 4)

- Implement optimized InsertField handling
- Test with single-field updates
- **Deliverable**: Struct field updates optimized

### Phase 3: InsertTuple Optimization (Step 5)

- Implement optimized InsertTuple handling
- Test with tuple element updates
- **Deliverable**: Tuple element updates optimized

### Phase 4: Integration and Testing (Step 6 + Tests)

- Update all call sites
- Comprehensive test suite
- Performance benchmarking
- **Deliverable**: Full optimization active with verification

## Files to Modify

### Primary Implementation

1. **`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates.rs`**
   - Add helper methods (300+ lines)
   - Modify `lower_instruction` signature and logic (200+ lines)
   - Update `run` method to compute use counts (10+ lines)

### Test Files

2. **`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/lower_aggregates_tests.rs`**
   (if exists)
   - Add optimization test cases (200+ lines)
3. **Create new test file if needed**:
   **`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/insert_optimization_tests.rs`**

### Imports to Add

```rust
use rustc_hash::FxHashMap;  // For use_counts parameter (likely already imported)
```

## Expected Results

After implementing this optimization:

1. **Performance Improvements**:
   - 50-85% reduction in instructions for single-use aggregate updates
   - Reduced memory allocations and frame usage
   - Better instruction cache utilization

2. **Correctness Maintained**:
   - Multi-use aggregates still use safe copy-on-write
   - All edge cases handled conservatively
   - No semantic changes to program behavior

3. **Code Quality**:
   - Clear separation between optimization and safety
   - Comprehensive test coverage
   - Detailed comments explaining optimization decisions

4. **Backward Compatibility**:
   - Existing working cases continue to work unchanged
   - Pass can be disabled/enabled without affecting correctness
   - Gradual adoption possible

The optimization transforms the LowerAggregates pass from a simple but
inefficient copy-on-write strategy to an intelligent optimization that provides
significant performance benefits while maintaining full correctness guarantees.
