//! Tests for the SROA (Scalar Replacement of Aggregates) pass
//!
//! NOTE: These tests have been disabled because SROA relied on GetElementPtrTyped
//! which has been removed from the instruction set. The compiler now uses only
//! regular GetElementPtr instructions with integer offsets, and SROA would need
//! to be updated to analyze constant offset patterns to determine field accesses.

use crate::passes::sroa::SroaPass;
use crate::passes::MirPass;
use crate::{
    BasicBlock, FunctionId, Instruction, InstructionKind, MirFunction, MirType, Terminator, Value,
};
use index_vec::index_vec;

/// Helper to create a test MIR function
fn create_test_function() -> MirFunction {
    MirFunction::new("test_func".to_string())
}

/// Helper to check if a function contains an instruction of a specific kind
fn has_instruction_kind(func: &MirFunction, check: impl Fn(&InstructionKind) -> bool) -> bool {
    func.basic_blocks
        .iter()
        .any(|block| block.instructions.iter().any(|inst| check(&inst.kind)))
}

/// Helper to count instructions of a specific kind
#[allow(dead_code)]
fn count_instruction_kind(func: &MirFunction, check: impl Fn(&InstructionKind) -> bool) -> usize {
    func.basic_blocks
        .iter()
        .map(|block| {
            block
                .instructions
                .iter()
                .filter(|inst| check(&inst.kind))
                .count()
        })
        .sum()
}

// Tests have been disabled because they relied on GetElementPtrTyped and SSA aggregate
// instructions (BuildStruct, BuildTuple, ExtractValue, InsertValue) which have been
// removed from the instruction set.
//
// To re-enable SROA optimization, the pass would need to be updated to:
// 1. Analyze regular GetElementPtr instructions with constant offsets
// 2. Track field access patterns through offset analysis
// 3. Map constant offsets back to struct/tuple fields using DataLayout

#[test]
fn test_sroa_no_longer_optimizes_regular_gep() {
    // This test verifies that SROA no longer optimizes structs accessed via regular GEP
    let mut function = create_test_function();

    // Create a struct type
    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let alloc = function.new_typed_value_id(struct_type.clone());
    let gep_x = function.new_typed_value_id(MirType::felt());
    let gep_y = function.new_typed_value_id(MirType::felt());
    let load_x = function.new_typed_value_id(MirType::felt());
    let load_y = function.new_typed_value_id(MirType::felt());
    let sum = function.new_typed_value_id(MirType::felt());

    let entry_block = BasicBlock {
        name: None,
        instructions: vec![
            Instruction::frame_alloc(alloc, struct_type),
            // Regular GEP with constant offsets (not GetElementPtrTyped)
            Instruction::get_element_ptr(gep_x, Value::Operand(alloc), Value::integer(0)),
            Instruction::store(Value::Operand(gep_x), Value::integer(10), MirType::felt()),
            Instruction::get_element_ptr(gep_y, Value::Operand(alloc), Value::integer(1)),
            Instruction::store(Value::Operand(gep_y), Value::integer(20), MirType::felt()),
            Instruction::load(load_x, MirType::felt(), Value::Operand(gep_x)),
            Instruction::load(load_y, MirType::felt(), Value::Operand(gep_y)),
            Instruction::binary_op(
                crate::BinaryOp::Add,
                sum,
                Value::Operand(load_x),
                Value::Operand(load_y),
            ),
        ],
        terminator: Terminator::Return {
            values: vec![Value::Operand(sum)],
        },
    };

    function.basic_blocks = index_vec![entry_block];

    // Run SROA
    let mut sroa = SroaPass::new();
    let changed = sroa.run(&mut function);

    // SROA should not optimize regular GEP instructions
    assert!(
        !changed,
        "SROA should not modify functions with regular GEP"
    );

    // The struct allocation should still be present
    assert!(
        has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Struct { .. })
        )),
        "Struct allocation should still be present"
    );
}

#[test]
fn test_sroa_escaping_allocation() {
    let mut function = create_test_function();

    // Create a struct that escapes (passed to a call)
    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let alloc = function.new_typed_value_id(struct_type.clone());
    let dummy_func = FunctionId::from_raw(1);

    let entry_block = BasicBlock {
        name: None,
        instructions: vec![
            Instruction::frame_alloc(alloc, struct_type.clone()),
            Instruction::void_call(
                dummy_func,
                vec![Value::Operand(alloc)],
                crate::instruction::CalleeSignature {
                    param_types: vec![struct_type],
                    return_types: vec![],
                },
            ),
        ],
        terminator: Terminator::Return { values: vec![] },
    };

    function.basic_blocks = index_vec![entry_block];

    // Run SROA
    let mut sroa = SroaPass::new();
    let changed = sroa.run(&mut function);

    // Escaping allocations should not be optimized
    assert!(!changed, "SROA should not modify escaping allocations");

    // The struct allocation should still be present
    assert!(
        has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Struct { .. })
        )),
        "Escaping struct allocation should be preserved"
    );
}

#[test]
fn test_sroa_empty_function() {
    let mut function = create_test_function();

    // Empty function
    let entry_block = BasicBlock {
        name: None,
        instructions: vec![],
        terminator: Terminator::Return { values: vec![] },
    };

    function.basic_blocks = index_vec![entry_block];

    // Run SROA
    let mut sroa = SroaPass::new();
    let changed = sroa.run(&mut function);

    assert!(!changed, "SROA should not modify empty functions");
}
