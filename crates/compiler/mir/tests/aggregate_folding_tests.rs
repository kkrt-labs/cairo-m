//! Tests for aggregate folding optimizations

use cairo_m_compiler_mir::passes::{const_fold::ConstFoldPass, MirPass};
use cairo_m_compiler_mir::*;

/// Helper to create a test function with aggregate operations
fn create_folding_test_function() -> MirFunction {
    MirFunction::new("test_folding".to_string())
}

#[test]
fn test_extract_make_tuple_folding() {
    let mut function = create_folding_test_function();

    // Create: tuple = MakeTuple(a, b, c); result = ExtractTuple(tuple, 1)
    // Should fold to: result = b

    let a = function.new_value_id();
    let b = function.new_value_id();
    let c = function.new_value_id();
    let tuple_val = function.new_value_id();
    let result = function.new_value_id();

    let block_id = function.add_basic_block();
    {
        let block = function.get_basic_block_mut(block_id).unwrap();

        // Create tuple
        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::operand(a), Value::operand(b), Value::operand(c)],
        ));

        // Extract middle element
        block.instructions.push(Instruction::extract_tuple_element(
            result,
            Value::operand(tuple_val),
            1,
            MirType::felt(),
        ));

        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(result)],
        });
    }

    // Run optimization
    let initial_count = function.basic_blocks[block_id].instructions.len();
    let mut pass = ConstFoldPass::new();
    let modified = pass.run(&mut function);
    let final_count = function.basic_blocks[block_id].instructions.len();

    // Should have folded the extraction
    assert!(modified);
    assert!(final_count < initial_count);
}

#[test]
fn test_extract_make_struct_folding() {
    let mut function = create_folding_test_function();

    // Create: struct = MakeStruct{x: a, y: b}; result = ExtractField(struct, "y")
    // Should fold to: result = b

    let a = function.new_value_id();
    let b = function.new_value_id();
    let struct_val = function.new_value_id();
    let result = function.new_value_id();

    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let block_id = function.add_basic_block();
    {
        let block = function.get_basic_block_mut(block_id).unwrap();

        // Create struct
        block.instructions.push(Instruction::make_struct(
            struct_val,
            vec![
                ("x".to_string(), Value::operand(a)),
                ("y".to_string(), Value::operand(b)),
            ],
            struct_type,
        ));

        // Extract y field
        block.instructions.push(Instruction::extract_struct_field(
            result,
            Value::operand(struct_val),
            "y".to_string(),
            MirType::felt(),
        ));

        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(result)],
        });
    }

    // Run optimization
    let initial_count = function.basic_blocks[block_id].instructions.len();
    let mut pass = ConstFoldPass::new();
    let modified = pass.run(&mut function);
    let final_count = function.basic_blocks[block_id].instructions.len();

    // Should have folded the extraction
    assert!(modified);
    assert!(final_count < initial_count);
}

#[test]
fn test_no_folding_when_values_escape() {
    let mut function = create_folding_test_function();

    // Create: tuple = MakeTuple(a, b); x = ExtractTuple(tuple, 0); return (tuple, x)
    // Should NOT fold because tuple escapes

    let a = function.new_value_id();
    let b = function.new_value_id();
    let tuple_val = function.new_value_id();
    let extracted = function.new_value_id();

    let block_id = function.add_basic_block();
    {
        let block = function.get_basic_block_mut(block_id).unwrap();

        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::operand(a), Value::operand(b)],
        ));

        block.instructions.push(Instruction::extract_tuple_element(
            extracted,
            Value::operand(tuple_val),
            0,
            MirType::felt(),
        ));

        // Return both tuple and extracted value
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(tuple_val), Value::operand(extracted)],
        });
    }

    // Run optimization
    let initial_count = function.basic_blocks[block_id].instructions.len();
    let mut pass = ConstFoldPass::new();
    pass.run(&mut function);
    let final_count = function.basic_blocks[block_id].instructions.len();

    // Should NOT fold because tuple is used in return
    assert_eq!(final_count, initial_count);
}

#[test]
fn test_chained_aggregate_folding() {
    let mut function = create_folding_test_function();

    // Create: t1 = MakeTuple(a, b); t2 = MakeTuple(t1, c);
    //         inner = ExtractTuple(t2, 0); result = ExtractTuple(inner, 1)
    // Should fold to: result = b

    let a = function.new_value_id();
    let b = function.new_value_id();
    let c = function.new_value_id();
    let t1 = function.new_value_id();
    let t2 = function.new_value_id();
    let inner = function.new_value_id();
    let result = function.new_value_id();

    let block_id = function.add_basic_block();
    {
        let block = function.get_basic_block_mut(block_id).unwrap();

        // Create inner tuple
        block.instructions.push(Instruction::make_tuple(
            t1,
            vec![Value::operand(a), Value::operand(b)],
        ));

        // Create outer tuple containing inner
        block.instructions.push(Instruction::make_tuple(
            t2,
            vec![Value::operand(t1), Value::operand(c)],
        ));

        // Extract inner tuple
        block.instructions.push(Instruction::extract_tuple_element(
            inner,
            Value::operand(t2),
            0,
            MirType::Tuple(vec![MirType::felt(), MirType::felt()]),
        ));

        // Extract from inner tuple
        block.instructions.push(Instruction::extract_tuple_element(
            result,
            Value::operand(inner),
            1,
            MirType::felt(),
        ));

        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(result)],
        });
    }

    // Run optimization - may need multiple passes for chained folding
    let initial_count = function.basic_blocks[block_id].instructions.len();
    let mut pass = ConstFoldPass::new();

    // Run multiple times to handle chained optimizations
    let mut total_modified = false;
    for _ in 0..3 {
        if pass.run(&mut function) {
            total_modified = true;
        }
    }

    let final_count = function.basic_blocks[block_id].instructions.len();

    // Should have significantly reduced instruction count
    assert!(total_modified);
    assert!(final_count < initial_count);
}

#[test]
fn test_insert_field_on_fresh_struct() {
    let mut function = create_folding_test_function();

    // Create: s = MakeStruct{x: a, y: b}; s2 = InsertField(s, "x", c)
    // Could optimize to: s2 = MakeStruct{x: c, y: b} if s is not used elsewhere

    let a = function.new_value_id();
    let b = function.new_value_id();
    let c = function.new_value_id();
    let s1 = function.new_value_id();
    let s2 = function.new_value_id();

    let struct_type = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::felt()),
            ("y".to_string(), MirType::felt()),
        ],
    };

    let block_id = function.add_basic_block();
    {
        let block = function.get_basic_block_mut(block_id).unwrap();

        // Create struct
        block.instructions.push(Instruction::make_struct(
            s1,
            vec![
                ("x".to_string(), Value::operand(a)),
                ("y".to_string(), Value::operand(b)),
            ],
            struct_type.clone(),
        ));

        // Update field
        block.instructions.push(Instruction::insert_field(
            s2,
            Value::operand(s1),
            "x".to_string(),
            Value::operand(c),
            struct_type,
        ));

        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(s2)],
        });
    }

    // Run optimization
    let mut pass = ConstFoldPass::new();
    pass.run(&mut function);

    // Verify the function still works correctly - may optimize to 1 instruction
    // if the pass can fold InsertField on a fresh struct
    let final_count = function.basic_blocks[block_id].instructions.len();
    assert!(final_count == 1 || final_count == 2);
}

#[test]
fn test_literal_aggregate_folding() {
    let mut function = create_folding_test_function();

    // Create: tuple = MakeTuple(1, 2, 3); result = ExtractTuple(tuple, 2)
    // Should fold to: result = 3

    let tuple_val = function.new_value_id();
    let result = function.new_value_id();

    let block_id = function.add_basic_block();
    {
        let block = function.get_basic_block_mut(block_id).unwrap();

        // Create tuple with literals
        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::integer(1), Value::integer(2), Value::integer(3)],
        ));

        // Extract last element
        block.instructions.push(Instruction::extract_tuple_element(
            result,
            Value::operand(tuple_val),
            2,
            MirType::felt(),
        ));

        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(result)],
        });
    }

    // Run optimization
    let initial_count = function.basic_blocks[block_id].instructions.len();
    let mut pass = ConstFoldPass::new();
    let modified = pass.run(&mut function);
    let final_count = function.basic_blocks[block_id].instructions.len();

    // Should have folded
    assert!(modified);
    assert!(final_count < initial_count);
}
