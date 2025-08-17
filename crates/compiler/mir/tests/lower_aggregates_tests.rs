//! Tests for the LowerAggregatesPass
//!
//! These tests ensure that:
//! 1. ValueIds are properly allocated using new_typed_value_id
//! 2. InsertField/InsertTuple copy unchanged fields/elements
//! 3. Proper offsets are used for tuple/struct access

use cairo_m_compiler_mir::{
    passes::{LowerAggregatesPass, MirPass},
    Instruction, InstructionKind, MirFunction, MirType, Value,
};

#[test]
fn test_lower_aggregates_uses_proper_value_ids() {
    let mut function = MirFunction::new("test_value_ids".to_string());
    let entry = function.entry_block;

    // Track initial value count by creating a dummy value
    let initial_dummy = function.new_value_id();
    let initial_value_count = initial_dummy.index();

    // Create a MakeTuple instruction
    let elem1 = function.new_typed_value_id(MirType::felt());
    let elem2 = function.new_typed_value_id(MirType::U32);
    let tuple_dest =
        function.new_typed_value_id(MirType::Tuple(vec![MirType::felt(), MirType::U32]));

    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::make_tuple(
            tuple_dest,
            vec![Value::operand(elem1), Value::operand(elem2)],
        ));

    // Run the lowering pass
    let mut pass = LowerAggregatesPass::new();
    let modified = pass.run(&mut function);

    assert!(modified, "Pass should modify the function");

    // Check that new ValueIds were allocated properly (not hardcoded 1000+)
    let final_dummy = function.new_value_id();
    let final_value_count = final_dummy.index();
    assert!(
        final_value_count > initial_value_count,
        "New ValueIds should have been allocated through function.new_typed_value_id"
    );

    // Verify no ValueId is >= 1000 (which would indicate hardcoded IDs)
    for block in &function.basic_blocks {
        for instr in &block.instructions {
            check_no_hardcoded_ids(instr);
        }
    }
}

fn check_no_hardcoded_ids(instr: &Instruction) {
    match &instr.kind {
        InstructionKind::GetElementPtr { dest, .. }
        | InstructionKind::Load { dest, .. }
        | InstructionKind::FrameAlloc { dest, .. }
        | InstructionKind::Assign { dest, .. } => {
            assert!(
                dest.index() < 1000,
                "Found hardcoded ValueId {} >= 1000",
                dest.index()
            );
        }
        _ => {}
    }
}

#[test]
fn test_insert_field_copies_other_fields() {
    let mut function = MirFunction::new("test_insert_field".to_string());
    let entry = function.entry_block;

    let struct_type = MirType::Struct {
        name: "TestStruct".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    // Create initial struct
    let x_val = function.new_typed_value_id(MirType::felt());
    let y_val = function.new_typed_value_id(MirType::felt());
    let struct1 = function.new_typed_value_id(struct_type.clone());

    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::make_struct(
            struct1,
            vec![
                ("x".to_string(), Value::operand(x_val)),
                ("y".to_string(), Value::operand(y_val)),
            ],
            struct_type.clone(),
        ));

    // Insert a new value for field "x"
    let new_x = function.new_typed_value_id(MirType::felt());
    let struct2 = function.new_typed_value_id(struct_type.clone());

    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::insert_field(
            struct2,
            Value::operand(struct1),
            "x".to_string(),
            Value::operand(new_x),
            struct_type,
        ));

    // Run the lowering pass
    let mut pass = LowerAggregatesPass::new();
    pass.run(&mut function);

    // The lowered code should:
    // 1. Allocate memory for struct1
    // 2. Store x_val and y_val
    // 3. Allocate memory for struct2
    // 4. Copy y_val from struct1 to struct2 (this is what we're testing)
    // 5. Store new_x to struct2's x field

    // Look for the copy operations
    let instructions = &function.basic_blocks[entry.index()].instructions;

    // Count load and store operations (should have loads/stores for copying)
    let load_count = instructions
        .iter()
        .filter(|i| matches!(i.kind, InstructionKind::Load { .. }))
        .count();
    let store_count = instructions
        .iter()
        .filter(|i| matches!(i.kind, InstructionKind::Store { .. }))
        .count();

    // We expect at least:
    // - 2 stores for initial struct (x, y)
    // - 1 load for copying y from struct1
    // - 2 stores for struct2 (copied y, new x)
    assert!(
        store_count >= 4,
        "Should have stores for initial struct and updated struct"
    );
    assert!(
        load_count >= 1,
        "Should have load for copying unchanged field"
    );
}

#[test]
fn test_insert_tuple_copies_other_elements() {
    let mut function = MirFunction::new("test_insert_tuple".to_string());
    let entry = function.entry_block;

    let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::U32, MirType::felt()]);

    // Create initial tuple
    let elem0 = function.new_typed_value_id(MirType::felt());
    let elem1 = function.new_typed_value_id(MirType::U32);
    let elem2 = function.new_typed_value_id(MirType::felt());
    let tuple1 = function.new_typed_value_id(tuple_type.clone());

    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::make_tuple(
            tuple1,
            vec![
                Value::operand(elem0),
                Value::operand(elem1),
                Value::operand(elem2),
            ],
        ));

    // Insert a new value at index 1
    let new_elem1 = function.new_typed_value_id(MirType::U32);
    let tuple2 = function.new_typed_value_id(tuple_type.clone());

    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::insert_tuple(
            tuple2,
            Value::operand(tuple1),
            1,
            Value::operand(new_elem1),
            tuple_type,
        ));

    // Run the lowering pass
    let mut pass = LowerAggregatesPass::new();
    pass.run(&mut function);

    // Check that elements 0 and 2 are copied
    let instructions = &function.basic_blocks[entry.index()].instructions;

    let load_count = instructions
        .iter()
        .filter(|i| matches!(i.kind, InstructionKind::Load { .. }))
        .count();
    let store_count = instructions
        .iter()
        .filter(|i| matches!(i.kind, InstructionKind::Store { .. }))
        .count();

    // We expect loads for copying elements 0 and 2
    assert!(
        load_count >= 2,
        "Should have loads for copying unchanged elements"
    );
    // We expect stores for all initial elements plus all elements in the new tuple
    assert!(store_count >= 6, "Should have stores for both tuples");
}

#[test]
fn test_tuple_lowering_uses_proper_offsets() {
    let mut function = MirFunction::new("test_offsets".to_string());
    let entry = function.entry_block;

    // Create tuple with u32 (size 2) as first element
    let tuple_type = MirType::Tuple(vec![
        MirType::U32,    // Size 2, offset 0
        MirType::felt(), // Size 1, offset 2
    ]);

    let elem0 = function.new_typed_value_id(MirType::U32);
    let elem1 = function.new_typed_value_id(MirType::felt());
    let tuple = function.new_typed_value_id(tuple_type);

    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::make_tuple(
            tuple,
            vec![Value::operand(elem0), Value::operand(elem1)],
        ));

    // Extract element 1
    let extracted = function.new_typed_value_id(MirType::felt());
    function.basic_blocks[entry.index()]
        .instructions
        .push(Instruction::extract_tuple_element(
            extracted,
            Value::operand(tuple),
            1,
            MirType::felt(),
        ));

    // Run the lowering pass
    let mut pass = LowerAggregatesPass::new();
    pass.run(&mut function);

    // Find the GEP instruction for accessing element 1
    let instructions = &function.basic_blocks[entry.index()].instructions;
    let mut found_correct_offset = false;

    for instr in instructions {
        if let InstructionKind::GetElementPtr { offset, .. } = &instr.kind {
            // Check if this is accessing element 1 (should use offset 2, not 1)
            if let Some(comment) = &instr.comment {
                if comment.contains("element 1") || comment.contains("element 1") {
                    assert_eq!(
                        *offset,
                        Value::integer(2),
                        "Element 1 should be at offset 2 (after U32 at offset 0-1)"
                    );
                    found_correct_offset = true;
                }
            }
        }
    }

    assert!(
        found_correct_offset,
        "Should have found GEP with correct offset for element 1"
    );
}
