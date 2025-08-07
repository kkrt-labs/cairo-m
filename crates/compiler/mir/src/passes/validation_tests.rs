//! Tests for the validation pass

use super::*;
use crate::{
    BasicBlock, BasicBlockId, Instruction, InstructionKind, MirFunction, MirType, Terminator, Value,
};

#[test]
fn test_validation_with_logging() {
    // Set RUST_LOG to enable validation logging
    std::env::set_var("RUST_LOG", "debug");

    let mut function = MirFunction::new("test_validation".to_string());

    // Create a simple CFG with a critical edge
    for _ in 0..4 {
        function.basic_blocks.push(BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let b1 = BasicBlockId::from_raw(1);
    let b2 = BasicBlockId::from_raw(2);
    let merge = BasicBlockId::from_raw(3);

    // Entry branches to B1 or Merge (critical edge: Entry->Merge)
    let cond = function.new_value_id();
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::operand(cond),
        then_target: b1,
        else_target: merge,
    };

    // B1 branches to B2 or Merge (critical edge: B1->Merge)
    let cond2 = function.new_value_id();
    function.basic_blocks[b1].terminator = Terminator::If {
        condition: Value::operand(cond2),
        then_target: b2,
        else_target: merge,
    };

    // B2 -> Merge
    function.basic_blocks[b2].terminator = Terminator::Jump { target: merge };

    // Merge returns
    function.basic_blocks[merge].terminator = Terminator::Return { values: vec![] };

    // Run validation pass - should detect critical edges
    let mut validation = Validation::new();
    let modified = validation.run(&mut function);

    // Validation doesn't modify the function
    assert!(!modified);

    // Clean up
    std::env::remove_var("RUST_LOG");
}

#[test]
fn test_pointer_type_validation() {
    let mut function = MirFunction::new("test_pointer_validation".to_string());

    // Create a block with a load from a non-pointer
    function.basic_blocks.push(BasicBlock::new());

    let non_pointer_val = function.new_value_id();
    let dest = function.new_value_id();

    // This is incorrect - loading from a non-pointer value
    function.basic_blocks[BasicBlockId::from_raw(0)]
        .instructions
        .push(Instruction {
            kind: InstructionKind::Load {
                dest,
                address: Value::operand(non_pointer_val),
                ty: MirType::Felt,
            },
            source_span: None,
            source_expr_id: None,
            comment: Some("Invalid load from non-pointer".to_string()),
        });

    function.basic_blocks[BasicBlockId::from_raw(0)].terminator =
        Terminator::Return { values: vec![] };

    // Run validation - should detect the non-pointer load
    std::env::set_var("RUST_LOG", "error");
    let mut validation = Validation::new();
    let modified = validation.run(&mut function);
    assert!(!modified);
    std::env::remove_var("RUST_LOG");
}

#[test]
fn test_gep_validation() {
    let mut function = MirFunction::new("test_gep_validation".to_string());

    // Create a block with a GEP using raw offset
    function.basic_blocks.push(BasicBlock::new());

    let base_ptr = function.new_value_id();
    let dest_ptr = function.new_value_id();

    // GEP with raw offset (should trigger warning)
    function.basic_blocks[BasicBlockId::from_raw(0)]
        .instructions
        .push(Instruction {
            kind: InstructionKind::GetElementPtr {
                dest: dest_ptr,
                base: Value::operand(base_ptr),
                offset: Value::Literal(Literal::Integer(8)), // Raw offset
            },
            source_span: None,
            source_expr_id: None,
            comment: Some("GEP with raw offset".to_string()),
        });

    function.basic_blocks[BasicBlockId::from_raw(0)].terminator =
        Terminator::Return { values: vec![] };

    // Run validation - should warn about raw offset GEP
    std::env::set_var("RUST_LOG", "warn");
    let mut validation = Validation::new();
    let modified = validation.run(&mut function);
    assert!(!modified);
    std::env::remove_var("RUST_LOG");
}

#[test]
fn test_single_definition_validation() {
    let mut function = MirFunction::new("test_single_def".to_string());

    // Create two blocks
    function.basic_blocks.push(BasicBlock::new());
    function.basic_blocks.push(BasicBlock::new());

    let value_id = function.new_value_id();

    // Define the same value in two different blocks (violation!)
    function.basic_blocks[BasicBlockId::from_raw(0)]
        .instructions
        .push(Instruction::assign(
            value_id,
            Value::Literal(Literal::Integer(1)),
            MirType::Felt,
        ));

    function.basic_blocks[BasicBlockId::from_raw(1)]
        .instructions
        .push(Instruction::assign(
            value_id,
            Value::Literal(Literal::Integer(2)),
            MirType::Felt,
        ));

    function.basic_blocks[BasicBlockId::from_raw(0)].terminator = Terminator::Jump {
        target: BasicBlockId::from_raw(1),
    };
    function.basic_blocks[BasicBlockId::from_raw(1)].terminator =
        Terminator::Return { values: vec![] };

    // Run validation - should detect multiple definitions
    std::env::set_var("RUST_LOG", "error");
    let mut validation = Validation::new();
    let modified = validation.run(&mut function);
    assert!(!modified);
    std::env::remove_var("RUST_LOG");
}
