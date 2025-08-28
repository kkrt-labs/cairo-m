//! Integration tests for aggregate patterns in MIR

use cairo_m_compiler_mir::*;

/// Test helper to create a simple MIR function for testing
fn create_test_function(name: &str) -> MirFunction {
    MirFunction::new(name.to_string())
}

/// Test helper to create a MIR module with aggregate configuration
fn compile_with_aggregate_config(enabled: bool) -> MirModule {
    let mut module = MirModule::new();
    let mut function = create_test_function("test");

    // Create test based on config
    if enabled {
        // Use aggregate instructions
        let _elem1 = function.new_value_id();
        let _elem2 = function.new_value_id();
        let tuple_val = function.new_value_id();

        let block_id = function.add_basic_block();
        let block = function.get_basic_block_mut(block_id).unwrap();

        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::integer(1), Value::integer(2)],
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(tuple_val)],
        });
    } else {
        // Use memory operations (simulated)
        let alloca = function.new_value_id();
        let block_id = function.add_basic_block();
        let block = function.get_basic_block_mut(block_id).unwrap();

        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(alloca)],
        });
    }

    module.add_function(function);
    module
}

#[test]
fn test_tuple_creation_and_extraction() {
    let mut function = create_test_function("test_tuple");

    // Create values
    let elem1 = function.new_value_id();
    let elem2 = function.new_value_id();
    let tuple_val = function.new_value_id();
    let extracted = function.new_value_id();

    // Create basic block
    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Add instructions
    block.instructions.push(Instruction::make_tuple(
        tuple_val,
        vec![Value::operand(elem1), Value::operand(elem2)],
    ));

    block.instructions.push(Instruction::extract_tuple_element(
        extracted,
        Value::operand(tuple_val),
        0,
        MirType::felt(),
    ));

    block.set_terminator(Terminator::Return {
        values: vec![Value::operand(extracted)],
    });

    // Verify structure
    assert_eq!(block.instructions.len(), 2);
    assert!(matches!(
        block.instructions[0].kind,
        InstructionKind::MakeTuple { .. }
    ));
    assert!(matches!(
        block.instructions[1].kind,
        InstructionKind::ExtractTupleElement { .. }
    ));
}

#[test]
fn test_struct_creation_and_field_access() {
    let mut function = create_test_function("test_struct");

    // Create values
    let x_val = function.new_value_id();
    let y_val = function.new_value_id();
    let struct_val = function.new_value_id();
    let extracted = function.new_value_id();

    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    // Create basic block
    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Add instructions
    block.instructions.push(Instruction::make_struct(
        struct_val,
        vec![
            ("x".to_string(), Value::operand(x_val)),
            ("y".to_string(), Value::operand(y_val)),
        ],
        struct_type,
    ));

    block.instructions.push(Instruction::extract_struct_field(
        extracted,
        Value::operand(struct_val),
        "x".to_string(),
        MirType::felt(),
    ));

    block.set_terminator(Terminator::Return {
        values: vec![Value::operand(extracted)],
    });

    // Verify structure
    assert_eq!(block.instructions.len(), 2);
    assert!(matches!(
        block.instructions[0].kind,
        InstructionKind::MakeStruct { .. }
    ));
    assert!(matches!(
        block.instructions[1].kind,
        InstructionKind::ExtractStructField { .. }
    ));
}

#[test]
fn test_insert_field_operation() {
    let mut function = create_test_function("test_insert");

    // Create values
    let struct_val = function.new_value_id();
    let _new_val = function.new_value_id();
    let updated = function.new_value_id();

    let struct_type = MirType::Struct {
        name: "Data".to_string(),
        fields: vec![
            ("value".to_string(), MirType::felt()),
            ("flag".to_string(), MirType::bool()),
        ],
    };

    // Create basic block
    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Add instructions
    block.instructions.push(Instruction::make_struct(
        struct_val,
        vec![
            ("value".to_string(), Value::integer(10)),
            ("flag".to_string(), Value::boolean(false)),
        ],
        struct_type.clone(),
    ));

    block.instructions.push(Instruction::insert_field(
        updated,
        Value::operand(struct_val),
        "value".to_string(),
        Value::integer(20),
        struct_type,
    ));

    block.set_terminator(Terminator::Return {
        values: vec![Value::operand(updated)],
    });

    // Verify structure
    assert_eq!(block.instructions.len(), 2);
    assert!(matches!(
        block.instructions[1].kind,
        InstructionKind::InsertField { .. }
    ));
}

#[test]
fn test_insert_tuple_operation() {
    let mut function = create_test_function("test_insert_tuple");

    // Create values
    let tuple_val = function.new_value_id();
    let updated = function.new_value_id();

    let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt(), MirType::felt()]);

    // Create basic block
    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Add instructions
    block.instructions.push(Instruction::make_tuple(
        tuple_val,
        vec![Value::integer(1), Value::integer(2), Value::integer(3)],
    ));

    block.instructions.push(Instruction::insert_tuple(
        updated,
        Value::operand(tuple_val),
        1, // Update middle element
        Value::integer(42),
        tuple_type,
    ));

    block.set_terminator(Terminator::Return {
        values: vec![Value::operand(updated)],
    });

    // Verify structure
    assert_eq!(block.instructions.len(), 2);
    assert!(matches!(
        block.instructions[1].kind,
        InstructionKind::InsertTuple { .. }
    ));
}

#[test]
fn test_nested_aggregates() {
    let mut function = create_test_function("test_nested");

    // Create nested structure: struct with tuple field
    let inner_tuple = function.new_value_id();
    let outer_struct = function.new_value_id();
    let extracted_tuple = function.new_value_id();
    let extracted_elem = function.new_value_id();

    let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
    let struct_type = MirType::Struct {
        name: "Container".to_string(),
        fields: vec![
            ("data".to_string(), tuple_type.clone()),
            ("count".to_string(), MirType::felt()),
        ],
    };

    // Create basic block
    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Create inner tuple
    block.instructions.push(Instruction::make_tuple(
        inner_tuple,
        vec![Value::integer(10), Value::integer(20)],
    ));

    // Create outer struct with tuple field
    block.instructions.push(Instruction::make_struct(
        outer_struct,
        vec![
            ("data".to_string(), Value::operand(inner_tuple)),
            ("count".to_string(), Value::integer(2)),
        ],
        struct_type,
    ));

    // Extract tuple from struct
    block.instructions.push(Instruction::extract_struct_field(
        extracted_tuple,
        Value::operand(outer_struct),
        "data".to_string(),
        tuple_type,
    ));

    // Extract element from tuple
    block.instructions.push(Instruction::extract_tuple_element(
        extracted_elem,
        Value::operand(extracted_tuple),
        0,
        MirType::felt(),
    ));

    block.set_terminator(Terminator::Return {
        values: vec![Value::operand(extracted_elem)],
    });

    // Verify all instructions are present
    assert_eq!(block.instructions.len(), 4);
}

#[test]
fn test_aggregate_mode_switching() {
    // Test with aggregates enabled
    let module_agg = compile_with_aggregate_config(true);

    // Test with aggregates disabled (memory mode)
    let module_mem = compile_with_aggregate_config(false);

    // Both should have one function
    assert_eq!(module_agg.function_count(), 1);
    assert_eq!(module_mem.function_count(), 1);

    // Check instruction types
    let has_aggregate_instructions = |module: &MirModule| {
        for (_id, func) in module.functions() {
            for block in func.basic_blocks.iter() {
                for instr in &block.instructions {
                    if matches!(
                        instr.kind,
                        InstructionKind::MakeTuple { .. }
                            | InstructionKind::ExtractTupleElement { .. }
                            | InstructionKind::MakeStruct { .. }
                            | InstructionKind::ExtractStructField { .. }
                    ) {
                        return true;
                    }
                }
            }
        }
        false
    };

    assert!(has_aggregate_instructions(&module_agg));
}

#[test]
fn test_empty_aggregates() {
    let mut function = create_test_function("test_empty");

    // Test empty tuple
    let empty_tuple = function.new_value_id();

    // Test empty struct
    let empty_struct = function.new_value_id();
    let empty_struct_type = MirType::Struct {
        name: "Empty".to_string(),
        fields: vec![],
    };

    // Create basic block
    let block_id = function.add_basic_block();
    let block = function.get_basic_block_mut(block_id).unwrap();

    // Create empty tuple
    block
        .instructions
        .push(Instruction::make_tuple(empty_tuple, vec![]));

    // Create empty struct
    block.instructions.push(Instruction::make_struct(
        empty_struct,
        vec![],
        empty_struct_type,
    ));

    block.set_terminator(Terminator::Return { values: vec![] });

    // Verify instructions
    assert_eq!(block.instructions.len(), 2);
}
