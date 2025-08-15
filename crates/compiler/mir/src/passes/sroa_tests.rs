//! Tests for the SROA (Scalar Replacement of Aggregates) pass

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

#[test]
fn test_sroa_simple_struct() {
    let mut function = create_test_function();

    // Create a struct type
    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    // Build MIR:
    // %0 = framealloc {felt, felt}
    // %1 = getelementptr_typed %0, .x
    // store %1, 10
    // %2 = getelementptr_typed %0, .y
    // store %2, 20
    // %3 = load %1
    // %4 = load %2
    // %5 = %3 + %4
    // return %5

    let alloc = function.new_typed_value_id(struct_type.clone());
    let gep_x = function.new_typed_value_id(MirType::felt());
    let gep_y = function.new_typed_value_id(MirType::felt());
    let load_x = function.new_typed_value_id(MirType::felt());
    let load_y = function.new_typed_value_id(MirType::felt());
    let sum = function.new_typed_value_id(MirType::felt());

    let entry_block = BasicBlock {
        name: None,
        instructions: vec![
            Instruction::frame_alloc(alloc, struct_type.clone()),
            Instruction::get_element_ptr_typed(
                gep_x,
                Value::Operand(alloc),
                vec![crate::AccessPath::Field("x".to_string())],
                struct_type.clone(),
            ),
            Instruction::store(Value::Operand(gep_x), Value::integer(10), MirType::felt()),
            Instruction::get_element_ptr_typed(
                gep_y,
                Value::Operand(alloc),
                vec![crate::AccessPath::Field("y".to_string())],
                struct_type,
            ),
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

    assert!(changed, "SROA should have modified the function");

    // Check that the original struct allocation is gone
    assert!(
        !has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Struct { .. })
        )),
        "Struct allocation should be eliminated"
    );

    // Check that typed GEPs are gone
    assert!(
        !has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::GetElementPtrTyped { .. }
        )),
        "Typed GEPs should be eliminated"
    );

    // Check that we have scalar allocations instead
    let scalar_allocs = count_instruction_kind(&function, |kind| {
        matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Felt)
        )
    });
    assert_eq!(
        scalar_allocs, 2,
        "Should have 2 scalar allocations for x and y"
    );
}

#[test]
fn test_sroa_nested_struct() {
    let mut function = create_test_function();

    // Create nested struct types
    let inner_struct = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let outer_struct = MirType::Struct {
        name: "Line".to_string(),
        fields: vec![
            ("start".to_string(), inner_struct.clone()),
            ("end".to_string(), inner_struct),
        ],
    };

    // Build MIR with nested struct access
    let alloc = function.new_typed_value_id(outer_struct.clone());
    let gep_start_x = function.new_typed_value_id(MirType::felt());
    let load_val = function.new_typed_value_id(MirType::felt());

    let entry_block = BasicBlock {
        name: None,
        instructions: vec![
            Instruction::frame_alloc(alloc, outer_struct.clone()),
            Instruction::get_element_ptr_typed(
                gep_start_x,
                Value::Operand(alloc),
                vec![
                    crate::AccessPath::Field("start".to_string()),
                    crate::AccessPath::Field("x".to_string()),
                ],
                outer_struct,
            ),
            Instruction::store(
                Value::Operand(gep_start_x),
                Value::integer(42),
                MirType::felt(),
            ),
            Instruction::load(load_val, MirType::felt(), Value::Operand(gep_start_x)),
        ],
        terminator: Terminator::Return {
            values: vec![Value::Operand(load_val)],
        },
    };

    function.basic_blocks = index_vec![entry_block];

    // Run SROA
    let mut sroa = SroaPass::new();
    let changed = sroa.run(&mut function);

    assert!(changed, "SROA should have modified the function");

    // Check that nested struct allocation is gone
    assert!(
        !has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Struct { .. })
        )),
        "Nested struct allocation should be eliminated"
    );

    // Should have 4 scalar allocations (start.x, start.y, end.x, end.y)
    let scalar_allocs = count_instruction_kind(&function, |kind| {
        matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Felt)
        )
    });
    assert_eq!(
        scalar_allocs, 4,
        "Should have 4 scalar allocations for nested fields"
    );
}

#[test]
fn test_sroa_tuple() {
    let mut function = create_test_function();

    // Create a tuple type
    let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::u32()]);

    // Build MIR with tuple
    let alloc = function.new_typed_value_id(tuple_type.clone());
    let gep_0 = function.new_typed_value_id(MirType::felt());
    let gep_1 = function.new_typed_value_id(MirType::u32());
    let load_0 = function.new_typed_value_id(MirType::felt());

    let entry_block = BasicBlock {
        name: None,
        instructions: vec![
            Instruction::frame_alloc(alloc, tuple_type.clone()),
            Instruction::get_element_ptr_typed(
                gep_0,
                Value::Operand(alloc),
                vec![crate::AccessPath::TupleIndex(0)],
                tuple_type.clone(),
            ),
            Instruction::store(Value::Operand(gep_0), Value::integer(100), MirType::felt()),
            Instruction::get_element_ptr_typed(
                gep_1,
                Value::Operand(alloc),
                vec![crate::AccessPath::TupleIndex(1)],
                tuple_type,
            ),
            Instruction::store(Value::Operand(gep_1), Value::integer(200), MirType::u32()),
            Instruction::load(load_0, MirType::felt(), Value::Operand(gep_0)),
        ],
        terminator: Terminator::Return {
            values: vec![Value::Operand(load_0)],
        },
    };

    function.basic_blocks = index_vec![entry_block];

    // Run SROA
    let mut sroa = SroaPass::new();
    let changed = sroa.run(&mut function);

    assert!(changed, "SROA should have modified the function");

    // Check that tuple allocation is gone
    assert!(
        !has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Tuple(_))
        )),
        "Tuple allocation should be eliminated"
    );

    // Should have 2 scalar allocations
    let felt_allocs = count_instruction_kind(&function, |kind| {
        matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Felt)
        )
    });
    let u32_allocs = count_instruction_kind(&function, |kind| {
        matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::U32)
        )
    });
    assert_eq!(felt_allocs, 1, "Should have 1 felt allocation");
    assert_eq!(u32_allocs, 1, "Should have 1 u32 allocation");
}

#[test]
fn test_sroa_ssa_aggregates() {
    let mut function = create_test_function();

    // Test SSA aggregate scalarization with BuildTuple/ExtractValue
    let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);

    let val1 = function.new_typed_value_id(MirType::felt());
    let val2 = function.new_typed_value_id(MirType::felt());
    let tuple = function.new_typed_value_id(tuple_type.clone());
    let extract1 = function.new_typed_value_id(MirType::felt());
    let extract2 = function.new_typed_value_id(MirType::felt());
    let sum = function.new_typed_value_id(MirType::felt());

    let entry_block = BasicBlock {
        name: None,
        instructions: vec![
            Instruction::assign(val1, Value::integer(10), MirType::felt()),
            Instruction::assign(val2, Value::integer(20), MirType::felt()),
            Instruction::build_tuple(
                tuple,
                vec![Value::Operand(val1), Value::Operand(val2)],
                tuple_type,
            ),
            Instruction::extract_value(
                extract1,
                Value::Operand(tuple),
                vec![crate::AccessPath::TupleIndex(0)],
                MirType::felt(),
            ),
            Instruction::extract_value(
                extract2,
                Value::Operand(tuple),
                vec![crate::AccessPath::TupleIndex(1)],
                MirType::felt(),
            ),
            Instruction::binary_op(
                crate::BinaryOp::Add,
                sum,
                Value::Operand(extract1),
                Value::Operand(extract2),
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

    assert!(changed, "SROA should have modified the function");

    // Check that BuildTuple is gone
    assert!(
        !has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::BuildTuple { .. }
        )),
        "BuildTuple should be eliminated"
    );

    // Check that ExtractValue is gone
    assert!(
        !has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::ExtractValue { .. }
        )),
        "ExtractValue should be eliminated"
    );

    // The sum should now directly use val1 and val2
    // We can't easily check this without inspecting the actual instruction,
    // but the elimination of Build/Extract is sufficient proof
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

    assert!(!changed, "SROA should not modify escaping allocations");

    // Check that struct allocation is still there
    assert!(
        has_instruction_kind(&function, |kind| matches!(
            kind,
            InstructionKind::FrameAlloc { ty, .. } if matches!(ty, MirType::Struct { .. })
        )),
        "Escaping struct allocation should remain"
    );
}
