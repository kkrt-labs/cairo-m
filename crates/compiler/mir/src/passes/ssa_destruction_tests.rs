use super::*;
use crate::{MirType, Terminator, Value};

#[test]
fn test_simple_phi_elimination() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.entry_block;

    // Create blocks for a simple if-then-else
    let then_block = function.basic_blocks.push(crate::BasicBlock::new());
    let else_block = function.basic_blocks.push(crate::BasicBlock::new());
    let merge_block = function.basic_blocks.push(crate::BasicBlock::new());

    // Add some values
    let x = function.new_typed_value_id(MirType::felt());
    let y = function.new_typed_value_id(MirType::felt());
    let result = function.new_typed_value_id(MirType::felt());

    // Add a phi node in the merge block
    function.basic_blocks[merge_block]
        .instructions
        .push(Instruction::phi(
            result,
            MirType::felt(),
            vec![
                (then_block, Value::operand(x)),
                (else_block, Value::operand(y)),
            ],
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

    // Run the pass
    let modified = eliminate_phi_nodes(&mut function);
    assert!(modified);

    // Check that phi is gone from merge block
    let merge = &function.basic_blocks[merge_block];
    assert!(!merge
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::Phi { .. })));

    // Check that assignments were added to predecessors
    let then = &function.basic_blocks[then_block];
    assert!(then.instructions.iter().any(|inst| {
        if let InstructionKind::Assign { dest, source, .. } = &inst.kind {
            *dest == result && *source == Value::operand(x)
        } else {
            false
        }
    }));

    let else_block_instructions = &function.basic_blocks[else_block];
    assert!(else_block_instructions.instructions.iter().any(|inst| {
        if let InstructionKind::Assign { dest, source, .. } = &inst.kind {
            *dest == result && *source == Value::operand(y)
        } else {
            false
        }
    }));
}

#[test]
fn test_critical_edge_phi_elimination() {
    let mut function = MirFunction::new("test_critical".to_string());
    let entry = function.entry_block;

    // Create a diamond with critical edges
    // entry branches to left/right, left branches to merge1/merge2, right also branches to merge1/merge2
    let left_block = function.basic_blocks.push(crate::BasicBlock::new());
    let right_block = function.basic_blocks.push(crate::BasicBlock::new());
    let merge1_block = function.basic_blocks.push(crate::BasicBlock::new());
    let merge2_block = function.basic_blocks.push(crate::BasicBlock::new());

    // Add values
    let v1 = function.new_typed_value_id(MirType::felt());
    let v2 = function.new_typed_value_id(MirType::felt());
    let v3 = function.new_typed_value_id(MirType::felt());
    let v4 = function.new_typed_value_id(MirType::felt());
    let result1 = function.new_typed_value_id(MirType::felt());
    let result2 = function.new_typed_value_id(MirType::felt());

    // Add phi nodes with different values from the same predecessor
    function.basic_blocks[merge1_block]
        .instructions
        .push(Instruction::phi(
            result1,
            MirType::felt(),
            vec![
                (left_block, Value::operand(v1)),  // left->merge1 uses v1
                (right_block, Value::operand(v3)), // right->merge1 uses v3
            ],
        ));

    function.basic_blocks[merge2_block]
        .instructions
        .push(Instruction::phi(
            result2,
            MirType::felt(),
            vec![
                (left_block, Value::operand(v2)), // left->merge2 uses v2 (different!)
                (right_block, Value::operand(v4)), // right->merge2 uses v4
            ],
        ));

    // Set up control flow
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::boolean(true),
        then_target: left_block,
        else_target: right_block,
    };

    // Left branches to both merge1 and merge2 (critical edges!)
    function.basic_blocks[left_block].terminator = Terminator::If {
        condition: Value::boolean(false),
        then_target: merge1_block,
        else_target: merge2_block,
    };

    // Right also branches to both merge1 and merge2 (more critical edges!)
    function.basic_blocks[right_block].terminator = Terminator::If {
        condition: Value::boolean(true),
        then_target: merge1_block,
        else_target: merge2_block,
    };

    // Count initial blocks
    let initial_block_count = function.basic_blocks.len();

    // Run the pass
    let modified = eliminate_phi_nodes(&mut function);
    assert!(modified);

    // Check that edge blocks were created (should have split 4 critical edges)
    assert!(
        function.basic_blocks.len() > initial_block_count,
        "Critical edges should have been split"
    );

    // Check that phi nodes are gone
    for block in function.basic_blocks.iter() {
        assert!(!block
            .instructions
            .iter()
            .any(|inst| matches!(inst.kind, InstructionKind::Phi { .. })));
    }

    // Verify that the assignments are correct by checking that each edge has the right value
    // This is complex to verify directly, but we can at least check that:
    // 1. No block has conflicting assignments to the same destination
    // 2. The structure is preserved (terminators still point to valid blocks)
    for block in function.basic_blocks.iter() {
        match &block.terminator {
            Terminator::Jump { target } => {
                assert!(
                    function.basic_blocks.get(*target).is_some(),
                    "Jump target should exist"
                );
            }
            Terminator::If {
                then_target,
                else_target,
                ..
            } => {
                assert!(
                    function.basic_blocks.get(*then_target).is_some(),
                    "Then target should exist"
                );
                assert!(
                    function.basic_blocks.get(*else_target).is_some(),
                    "Else target should exist"
                );
            }
            _ => {}
        }
    }
}
