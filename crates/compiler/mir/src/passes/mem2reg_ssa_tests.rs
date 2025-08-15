use super::*;
use crate::{BasicBlock, BasicBlockId, Instruction, MirType, Terminator, Value};

#[test]
fn test_simple_promotion() {
    let mut function = MirFunction::new("test".to_string());

    // Use the existing entry block created by MirFunction::new
    let entry_block_id = function.entry_block;

    // %0 = framealloc felt
    let alloc = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // store %0, 42
    // %1 = load %0
    let loaded = function.new_typed_value_id(MirType::felt());

    // Build instructions
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::frame_alloc(alloc, MirType::felt()));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::store(
            Value::operand(alloc),
            Value::integer(42),
            MirType::felt(),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::load(
            loaded,
            MirType::felt(),
            Value::operand(alloc),
        ));

    // return %1
    function.basic_blocks[entry_block_id].terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    assert!(changed);
    assert_eq!(pass.stats.allocations_promoted, 1);
    assert_eq!(pass.stats.stores_eliminated, 1);
    assert_eq!(pass.stats.loads_eliminated, 1);

    // Check that the allocation and memory operations were removed
    let block = &function.basic_blocks[BasicBlockId::from_raw(0)];
    assert_eq!(block.instructions.len(), 1); // Only the assign remains

    // The load should be replaced with an assign
    if let InstructionKind::Assign { source, .. } = &block.instructions[0].kind {
        assert_eq!(*source, Value::integer(42));
    } else {
        panic!("Expected assign instruction");
    }
}

#[test]
fn test_promotion_with_phi() {
    let mut function = MirFunction::new("test_phi".to_string());

    // Create an if-else pattern that requires a Phi node
    // Use existing entry block
    let entry_block_id = function.entry_block;
    let alloc = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let cond = function.new_typed_value_id(MirType::bool());

    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::frame_alloc(alloc, MirType::felt()));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::assign(
            cond,
            Value::boolean(true),
            MirType::bool(),
        ));

    // Then block - store 10
    let mut then_block = BasicBlock::new();
    then_block.instructions.push(Instruction::store(
        Value::operand(alloc),
        Value::integer(10),
        MirType::felt(),
    ));
    let then_block_id = function.basic_blocks.push(then_block);

    // Else block - store 20
    let mut else_block = BasicBlock::new();
    else_block.instructions.push(Instruction::store(
        Value::operand(alloc),
        Value::integer(20),
        MirType::felt(),
    ));
    let else_block_id = function.basic_blocks.push(else_block);

    // Merge block - load and return
    let mut merge_block = BasicBlock::new();
    let loaded = function.new_typed_value_id(MirType::felt());
    merge_block.instructions.push(Instruction::load(
        loaded,
        MirType::felt(),
        Value::operand(alloc),
    ));
    merge_block.terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };
    let merge_block_id = function.basic_blocks.push(merge_block);

    // Set up control flow
    function.basic_blocks[entry_block_id].terminator = Terminator::If {
        condition: Value::operand(cond),
        then_target: then_block_id,
        else_target: else_block_id,
    };
    function.basic_blocks[then_block_id].terminator = Terminator::Jump {
        target: merge_block_id,
    };
    function.basic_blocks[else_block_id].terminator = Terminator::Jump {
        target: merge_block_id,
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    assert!(changed);
    assert_eq!(pass.stats.allocations_promoted, 1);
    assert_eq!(pass.stats.phi_nodes_inserted, 1);
    assert_eq!(pass.stats.stores_eliminated, 2);
    assert_eq!(pass.stats.loads_eliminated, 1);

    // Check that a Phi node was inserted in the merge block
    let merge = &function.basic_blocks[BasicBlockId::from_raw(3)];
    assert!(!merge.instructions.is_empty());

    // First instruction should be a Phi node
    if let InstructionKind::Phi { sources, .. } = &merge.instructions[0].kind {
        assert_eq!(sources.len(), 2); // Two predecessors

        // Check that the Phi has the correct values
        let has_10 = sources.iter().any(|(_, v)| *v == Value::integer(10));
        let has_20 = sources.iter().any(|(_, v)| *v == Value::integer(20));
        assert!(has_10 && has_20);
    } else {
        panic!("Expected Phi instruction at beginning of merge block");
    }
}

#[test]
fn test_escaping_allocation_not_promoted() {
    let mut function = MirFunction::new("test_escape".to_string());

    // Use existing entry block
    let entry_block_id = function.entry_block;

    // %0 = framealloc felt
    let alloc = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // Call a function with the allocation address (escapes)
    let callee = crate::FunctionId::from_raw(0);

    // %1 = load %0
    let loaded = function.new_typed_value_id(MirType::felt());

    // Build instructions
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::frame_alloc(alloc, MirType::felt()));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::void_call(
            callee,
            vec![Value::operand(alloc)],
            crate::instruction::CalleeSignature {
                param_types: vec![MirType::pointer(MirType::felt())],
                return_types: vec![],
            },
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::store(
            Value::operand(alloc),
            Value::integer(42),
            MirType::felt(),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::load(
            loaded,
            MirType::felt(),
            Value::operand(alloc),
        ));

    function.basic_blocks[entry_block_id].terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // Should not be promoted because it escapes
    assert!(!changed);
    assert_eq!(pass.stats.allocations_promoted, 0);
    assert_eq!(pass.stats.stores_eliminated, 0);
    assert_eq!(pass.stats.loads_eliminated, 0);
}

#[test]
fn test_gep_promotion() {
    let mut function = MirFunction::new("test_gep".to_string());

    // Use existing entry block
    let entry_block_id = function.entry_block;

    // %0 = framealloc felt
    let alloc = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // %1 = getelementptr %0, 0
    let gep = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // %2 = load %1
    let loaded = function.new_typed_value_id(MirType::felt());

    // Build instructions
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::frame_alloc(alloc, MirType::felt()));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::get_element_ptr(
            gep,
            Value::operand(alloc),
            Value::integer(0),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::store(
            Value::operand(gep),
            Value::integer(99),
            MirType::felt(),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::load(
            loaded,
            MirType::felt(),
            Value::operand(gep),
        ));

    function.basic_blocks[entry_block_id].terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    assert!(changed);
    assert_eq!(pass.stats.allocations_promoted, 1);
    assert_eq!(pass.stats.stores_eliminated, 1);
    assert_eq!(pass.stats.loads_eliminated, 1);

    // Check that GEP was also removed
    let block = &function.basic_blocks[BasicBlockId::from_raw(0)];
    // Should only have the assign from the load replacement
    assert_eq!(block.instructions.len(), 1);
}

#[test]
fn test_u32_not_promoted() {
    // Test that u32 allocations (2 slots) are not promoted with current implementation
    let mut function = MirFunction::new("test_u32".to_string());
    let entry = function.entry_block;

    // Create blocks for a simple if-then-else that writes different halves
    let then_block = function.basic_blocks.push(crate::BasicBlock::new());
    let else_block = function.basic_blocks.push(crate::BasicBlock::new());
    let merge_block = function.basic_blocks.push(crate::BasicBlock::new());

    // Allocate a u32 (2 slots)
    let u32_alloc = function.new_typed_value_id(MirType::pointer(MirType::u32()));
    let low_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let high_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let low_val = function.new_typed_value_id(MirType::felt());
    let high_val = function.new_typed_value_id(MirType::felt());

    // Entry: allocate u32
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::frame_alloc(u32_alloc, MirType::u32()));

    // Then: write to low half
    function.basic_blocks[then_block]
        .instructions
        .push(Instruction::get_element_ptr(
            low_ptr,
            Value::operand(u32_alloc),
            Value::integer(0),
        ));
    function.basic_blocks[then_block]
        .instructions
        .push(Instruction::store(
            Value::operand(low_ptr),
            Value::integer(100),
            MirType::felt(),
        ));

    // Else: write to high half
    function.basic_blocks[else_block]
        .instructions
        .push(Instruction::get_element_ptr(
            high_ptr,
            Value::operand(u32_alloc),
            Value::integer(1),
        ));
    function.basic_blocks[else_block]
        .instructions
        .push(Instruction::store(
            Value::operand(high_ptr),
            Value::integer(200),
            MirType::felt(),
        ));

    // Merge: read both halves
    let merge_low_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let merge_high_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::get_element_ptr(
            merge_low_ptr,
            Value::operand(u32_alloc),
            Value::integer(0),
        ));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::get_element_ptr(
            merge_high_ptr,
            Value::operand(u32_alloc),
            Value::integer(1),
        ));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::load(
            low_val,
            MirType::felt(),
            Value::operand(merge_low_ptr),
        ));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::load(
            high_val,
            MirType::felt(),
            Value::operand(merge_high_ptr),
        ));

    // Set up control flow
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::boolean(true),
        then_target: then_block,
        else_target: else_block,
    };
    function.basic_blocks[then_block].terminator = Terminator::Jump {
        target: merge_block,
    };
    function.basic_blocks[else_block].terminator = Terminator::Jump {
        target: merge_block,
    };
    function.basic_blocks[merge_block].terminator = Terminator::Return {
        values: vec![Value::operand(low_val), Value::operand(high_val)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // Should NOT be promoted because u32 is multi-slot
    assert!(!changed);
    assert_eq!(pass.stats.allocations_promoted, 0);
}

#[test]
fn test_struct_not_promoted() {
    // Test that struct allocations are not promoted with current implementation
    let mut function = MirFunction::new("test_struct".to_string());
    let entry = function.entry_block;

    // Create blocks for a simple if-then-else that writes different fields
    let then_block = function.basic_blocks.push(crate::BasicBlock::new());
    let else_block = function.basic_blocks.push(crate::BasicBlock::new());
    let merge_block = function.basic_blocks.push(crate::BasicBlock::new());

    // Define a struct type with two felt fields
    let struct_ty = MirType::struct_type(
        "Point".to_string(),
        vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    );

    // Allocate the struct
    let struct_alloc = function.new_typed_value_id(MirType::pointer(struct_ty.clone()));
    let field_x_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let field_y_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let x_val = function.new_typed_value_id(MirType::felt());
    let y_val = function.new_typed_value_id(MirType::felt());

    // Entry: allocate struct
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::frame_alloc(struct_alloc, struct_ty));

    // Then: write to field x
    function.basic_blocks[then_block]
        .instructions
        .push(Instruction::get_element_ptr(
            field_x_ptr,
            Value::operand(struct_alloc),
            Value::integer(0),
        ));
    function.basic_blocks[then_block]
        .instructions
        .push(Instruction::store(
            Value::operand(field_x_ptr),
            Value::integer(10),
            MirType::felt(),
        ));

    // Else: write to field y
    function.basic_blocks[else_block]
        .instructions
        .push(Instruction::get_element_ptr(
            field_y_ptr,
            Value::operand(struct_alloc),
            Value::integer(1),
        ));
    function.basic_blocks[else_block]
        .instructions
        .push(Instruction::store(
            Value::operand(field_y_ptr),
            Value::integer(20),
            MirType::felt(),
        ));

    // Merge: read both fields
    let merge_x_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let merge_y_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::get_element_ptr(
            merge_x_ptr,
            Value::operand(struct_alloc),
            Value::integer(0),
        ));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::get_element_ptr(
            merge_y_ptr,
            Value::operand(struct_alloc),
            Value::integer(1),
        ));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::load(
            x_val,
            MirType::felt(),
            Value::operand(merge_x_ptr),
        ));
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::load(
            y_val,
            MirType::felt(),
            Value::operand(merge_y_ptr),
        ));

    // Set up control flow
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::boolean(true),
        then_target: then_block,
        else_target: else_block,
    };
    function.basic_blocks[then_block].terminator = Terminator::Jump {
        target: merge_block,
    };
    function.basic_blocks[else_block].terminator = Terminator::Jump {
        target: merge_block,
    };
    function.basic_blocks[merge_block].terminator = Terminator::Return {
        values: vec![Value::operand(x_val), Value::operand(y_val)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // Should NOT be promoted because struct is multi-slot
    assert!(!changed);
    assert_eq!(pass.stats.allocations_promoted, 0);
}

#[test]
fn test_tuple_not_promoted() {
    // Test that tuple allocations are not promoted with current implementation
    let mut function = MirFunction::new("test_tuple".to_string());
    let entry = function.entry_block;

    // Create a simple tuple type (felt, felt)
    let tuple_ty = MirType::tuple(vec![MirType::felt(), MirType::felt()]);

    // Allocate the tuple
    let tuple_alloc = function.new_typed_value_id(MirType::pointer(tuple_ty.clone()));

    // Entry: allocate tuple and store values
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::frame_alloc(tuple_alloc, tuple_ty));

    // Store to tuple elements
    let elem0_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));
    let elem1_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    function.basic_blocks[entry]
        .instructions
        .push(Instruction::get_element_ptr(
            elem0_ptr,
            Value::operand(tuple_alloc),
            Value::integer(0),
        ));
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::store(
            Value::operand(elem0_ptr),
            Value::integer(42),
            MirType::felt(),
        ));
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::get_element_ptr(
            elem1_ptr,
            Value::operand(tuple_alloc),
            Value::integer(1),
        ));
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::store(
            Value::operand(elem1_ptr),
            Value::integer(84),
            MirType::felt(),
        ));

    function.basic_blocks[entry].terminator = Terminator::Return { values: vec![] };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // Should NOT be promoted because tuple is multi-slot
    assert!(!changed);
    assert_eq!(pass.stats.allocations_promoted, 0);
}

#[test]
fn test_u32_simple_promotion() {
    // Test that U32 allocations CAN be promoted when accessed as a whole (no GEP)
    let mut function = MirFunction::new("test_u32_simple".to_string());
    let entry_block_id = function.entry_block;

    // %0 = framealloc u32
    let alloc = function.new_typed_value_id(MirType::pointer(MirType::u32()));

    // store %0, 12345u32
    // %1 = load %0
    let loaded = function.new_typed_value_id(MirType::u32());

    // Build instructions for U32 allocation, store, and load
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::frame_alloc(alloc, MirType::u32()));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::store(
            Value::operand(alloc),
            Value::integer(12345),
            MirType::u32(),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::load(
            loaded,
            MirType::u32(),
            Value::operand(alloc),
        ));

    // return %1
    function.basic_blocks[entry_block_id].terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // U32 should be promoted when accessed as a whole
    assert!(changed);
    assert_eq!(pass.stats.allocations_promoted, 1);
    assert_eq!(pass.stats.stores_eliminated, 1);
    assert_eq!(pass.stats.loads_eliminated, 1);

    // Check that the allocation and memory operations were removed
    let block = &function.basic_blocks[BasicBlockId::from_raw(0)];
    assert_eq!(block.instructions.len(), 1); // Only the assign remains

    // The load should be replaced with an assign
    if let InstructionKind::Assign { source, .. } = &block.instructions[0].kind {
        assert_eq!(*source, Value::integer(12345));
    } else {
        panic!("Expected assign instruction");
    }
}

#[test]
fn test_u32_with_gep_not_promoted() {
    // Test that U32 allocations with GEP access are NOT promoted
    let mut function = MirFunction::new("test_u32_gep".to_string());
    let entry_block_id = function.entry_block;

    // %0 = framealloc u32
    let alloc = function.new_typed_value_id(MirType::pointer(MirType::u32()));

    // %1 = getelementptr %0, 0  (access low part)
    let gep_low = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // store %1, 100
    // %2 = load %1
    let loaded = function.new_typed_value_id(MirType::felt());

    // Build instructions
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::frame_alloc(alloc, MirType::u32()));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::get_element_ptr(
            gep_low,
            Value::operand(alloc),
            Value::integer(0),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::store(
            Value::operand(gep_low),
            Value::integer(100),
            MirType::felt(),
        ));
    function.basic_blocks[entry_block_id]
        .instructions
        .push(Instruction::load(
            loaded,
            MirType::felt(),
            Value::operand(gep_low),
        ));

    // return %2
    function.basic_blocks[entry_block_id].terminator = Terminator::Return {
        values: vec![Value::operand(loaded)],
    };

    // Run mem2reg pass
    let mut pass = Mem2RegSsaPass::new();
    let changed = pass.optimize(&mut function);

    // U32 with GEP should NOT be promoted (requires per-slot phi)
    assert!(!changed);
    assert_eq!(pass.stats.allocations_promoted, 0);
}
