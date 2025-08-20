//! Tests for the SROA pass

use crate::instruction::InstructionKind;
use crate::passes::sroa::*;
use crate::{
    BinaryOp, Instruction, Literal, MirFunction, MirModule, MirType, PrettyPrint, Terminator, Value,
};

#[test]
fn test_simple_tuple_scalarization() {
    // Create a simple function with tuple operations
    let mut function = MirFunction::new("test_tuple".to_string());
    let entry = function.entry_block;

    // Create tuple elements
    let x = function.new_typed_value_id(MirType::Felt);
    let y = function.new_typed_value_id(MirType::Felt);

    // Make tuple: t = (x, y)
    let t = function.new_typed_value_id(MirType::tuple(vec![MirType::Felt, MirType::Felt]));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_tuple(
            t,
            vec![Value::operand(x), Value::operand(y)],
        ));

    // Extract elements: a = t.0, b = t.1
    let a = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_tuple_element(
            a,
            Value::operand(t),
            0,
            MirType::Felt,
        ));

    let b = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_tuple_element(
            b,
            Value::operand(t),
            1,
            MirType::Felt,
        ));

    // Add: result = a + b
    let result = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            result,
            Value::operand(a),
            Value::operand(b),
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut function);
    assert!(modified, "SROA should modify the function");

    // Check that MakeTuple was eliminated
    let block = function.get_basic_block(entry).unwrap();
    let has_make_tuple = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeTuple { .. }));
    assert!(!has_make_tuple, "MakeTuple should be eliminated");

    // Check that extracts were replaced with assigns
    let extract_count = block
        .instructions
        .iter()
        .filter(|inst| matches!(inst.kind, InstructionKind::ExtractTupleElement { .. }))
        .count();
    assert_eq!(extract_count, 0, "Extracts should be replaced");

    // Check we have assigns instead
    let assign_count = block
        .instructions
        .iter()
        .filter(|inst| matches!(inst.kind, InstructionKind::Assign { .. }))
        .count();
    assert_eq!(
        assign_count, 2,
        "Should have 2 assigns for the extracted values"
    );
}

#[test]
fn test_tuple_materialization_for_call() {
    let mut module = MirModule::new();

    // Create a function that takes a tuple
    let mut tuple_fn = MirFunction::new("takes_tuple".to_string());
    let tuple_param =
        tuple_fn.new_typed_value_id(MirType::tuple(vec![MirType::Felt, MirType::Felt]));
    tuple_fn.parameters.push(tuple_param);
    tuple_fn.return_values = vec![];
    let tuple_func = module.add_function(tuple_fn);

    // Create main function
    let mut main_fn = MirFunction::new("main".to_string());
    let entry = main_fn.entry_block;

    // Create tuple
    let x = main_fn.new_typed_value_id(MirType::Felt);
    let y = main_fn.new_typed_value_id(MirType::Felt);
    let t = main_fn.new_typed_value_id(MirType::tuple(vec![MirType::Felt, MirType::Felt]));
    main_fn
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_tuple(
            t,
            vec![Value::operand(x), Value::operand(y)],
        ));

    // Extract first element (to make SROA track the tuple)
    let elem = main_fn.new_typed_value_id(MirType::Felt);
    main_fn
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_tuple_element(
            elem,
            Value::operand(t),
            0,
            MirType::Felt,
        ));

    // Call function with tuple
    let signature = crate::instruction::CalleeSignature {
        param_types: vec![MirType::tuple(vec![MirType::Felt, MirType::Felt])],
        return_types: vec![MirType::Felt],
    };
    let ret_val = main_fn.new_typed_value_id(MirType::Felt);
    main_fn
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::call(
            vec![ret_val],
            tuple_func,
            vec![Value::operand(t)],
            signature,
        ));

    // Set terminator
    main_fn.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(ret_val)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut main_fn);
    assert!(modified);

    // Check that a MakeTuple exists before the call (materialization)
    let block = main_fn.get_basic_block(entry).unwrap();
    let make_tuples: Vec<_> = block
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, inst)| matches!(inst.kind, InstructionKind::MakeTuple { .. }))
        .collect();

    assert_eq!(
        make_tuples.len(),
        1,
        "Should have exactly one MakeTuple for materialization"
    );

    // The materialized tuple should appear right before the call
    let call_idx = block
        .instructions
        .iter()
        .position(|inst| matches!(inst.kind, InstructionKind::Call { .. }))
        .unwrap();

    let make_tuple_idx = make_tuples[0].0;
    assert!(make_tuple_idx < call_idx, "MakeTuple should precede Call");
}

#[test]
fn test_struct_partial_update() {
    // Create function
    let mut function = MirFunction::new("test_struct".to_string());
    let entry = function.entry_block;

    // Define struct type
    let struct_ty = MirType::struct_type(
        "Point".to_string(),
        vec![
            ("x".to_string(), MirType::Felt),
            ("y".to_string(), MirType::Felt),
        ],
    );

    // Create struct: p = Point { x: 1, y: 2 }
    let x = function.new_typed_value_id(MirType::Felt);
    let y = function.new_typed_value_id(MirType::Felt);
    let p = function.new_typed_value_id(struct_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            p,
            vec![
                ("x".to_string(), Value::operand(x)),
                ("y".to_string(), Value::operand(y)),
            ],
            struct_ty.clone(),
        ));

    // Update field: p2 = insert(p, "y", 3)
    let new_y = function.new_typed_value_id(MirType::Felt);
    let p2 = function.new_typed_value_id(struct_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::insert_field(
            p2,
            Value::operand(p),
            "y".to_string(),
            Value::operand(new_y),
            struct_ty,
        ));

    // Extract updated field: result = p2.y
    let result = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            result,
            Value::operand(p2),
            "y".to_string(),
            MirType::Felt,
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut function);
    assert!(modified);

    // Check that struct operations were eliminated
    let block = function.get_basic_block(entry).unwrap();

    let has_make_struct = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeStruct { .. }));
    assert!(!has_make_struct, "MakeStruct should be eliminated");

    let has_insert = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::InsertField { .. }));
    assert!(!has_insert, "InsertField should be eliminated");

    let has_extract = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::ExtractStructField { .. }));
    assert!(!has_extract, "ExtractStructField should be eliminated");

    // Should have an assign that directly uses new_y
    let assigns: Vec<_> = block
        .instructions
        .iter()
        .filter(|inst| matches!(inst.kind, InstructionKind::Assign { .. }))
        .collect();
    assert!(!assigns.is_empty(), "Should have assign instructions");
}

#[test]
fn test_aggregate_copy_forwarding() {
    let mut function = MirFunction::new("test_copy".to_string());
    let entry = function.entry_block;

    // Create tuple
    let x = function.new_typed_value_id(MirType::Felt);
    let y = function.new_typed_value_id(MirType::Felt);
    let t1 = function.new_typed_value_id(MirType::tuple(vec![MirType::Felt, MirType::Felt]));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_tuple(
            t1,
            vec![Value::operand(x), Value::operand(y)],
        ));

    // Copy tuple: t2 = t1
    let t2 = function.new_typed_value_id(MirType::tuple(vec![MirType::Felt, MirType::Felt]));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            t2,
            Value::operand(t1),
            MirType::tuple(vec![MirType::Felt, MirType::Felt]),
        ));

    // Extract from copy: result = t2.0
    let result = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_tuple_element(
            result,
            Value::operand(t2),
            0,
            MirType::Felt,
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut function);
    assert!(modified);

    // Check that tuple operations were eliminated
    let block = function.get_basic_block(entry).unwrap();

    // Should have no MakeTuple
    let has_make_tuple = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeTuple { .. }));
    assert!(!has_make_tuple, "MakeTuple should be eliminated");

    // Should have no aggregate assigns (they get forwarded)
    let agg_assign_count = block
        .instructions
        .iter()
        .filter(|inst| {
            if let InstructionKind::Assign { ty, .. } = &inst.kind {
                matches!(ty, MirType::Tuple(_))
            } else {
                false
            }
        })
        .count();
    assert_eq!(agg_assign_count, 0, "Aggregate assigns should be forwarded");
}

#[test]
fn test_nested_struct_scalarization() {
    // Create function
    let mut function = MirFunction::new("line_length_squared".to_string());
    let entry = function.entry_block;

    // Define types
    let point_ty = MirType::struct_type(
        "Point".to_string(),
        vec![
            ("x".to_string(), MirType::Felt),
            ("y".to_string(), MirType::Felt),
        ],
    );

    let line_ty = MirType::struct_type(
        "Line".to_string(),
        vec![
            ("start".to_string(), point_ty.clone()),
            ("end".to_string(), point_ty.clone()),
        ],
    );

    // Create inner Points
    let zero = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            zero,
            Value::Literal(Literal::Integer(0)),
            MirType::Felt,
        ));

    let three = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            three,
            Value::Literal(Literal::Integer(3)),
            MirType::Felt,
        ));

    let four = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            four,
            Value::Literal(Literal::Integer(4)),
            MirType::Felt,
        ));

    // Create start point: Point { x: 0, y: 0 }
    let start_point = function.new_typed_value_id(point_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            start_point,
            vec![
                ("x".to_string(), Value::operand(zero)),
                ("y".to_string(), Value::operand(zero)),
            ],
            point_ty.clone(),
        ));

    // Create end point: Point { x: 3, y: 4 }
    let end_point = function.new_typed_value_id(point_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            end_point,
            vec![
                ("x".to_string(), Value::operand(three)),
                ("y".to_string(), Value::operand(four)),
            ],
            point_ty.clone(),
        ));

    // Create line: Line { start: start_point, end: end_point }
    let line = function.new_typed_value_id(line_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            line,
            vec![
                ("start".to_string(), Value::operand(start_point)),
                ("end".to_string(), Value::operand(end_point)),
            ],
            line_ty,
        ));

    // Extract line.end
    let line_end = function.new_typed_value_id(point_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            line_end,
            Value::operand(line),
            "end".to_string(),
            point_ty.clone(),
        ));

    // Extract line.end.x
    let end_x = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            end_x,
            Value::operand(line_end),
            "x".to_string(),
            MirType::Felt,
        ));

    // Extract line.start
    let line_start = function.new_typed_value_id(point_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            line_start,
            Value::operand(line),
            "start".to_string(),
            point_ty,
        ));

    // Extract line.start.x
    let start_x = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            start_x,
            Value::operand(line_start),
            "x".to_string(),
            MirType::Felt,
        ));

    // Calculate dx = line.end.x - line.start.x
    let dx = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Sub,
            dx,
            Value::operand(end_x),
            Value::operand(start_x),
        ));

    // Extract line.end.y
    let end_y = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            end_y,
            Value::operand(line_end),
            "y".to_string(),
            MirType::Felt,
        ));

    // Extract line.start.y
    let start_y = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::extract_struct_field(
            start_y,
            Value::operand(line_start),
            "y".to_string(),
            MirType::Felt,
        ));

    // Calculate dy = line.end.y - line.start.y
    let dy = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Sub,
            dy,
            Value::operand(end_y),
            Value::operand(start_y),
        ));

    // Calculate dx * dx
    let dx_squared = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Mul,
            dx_squared,
            Value::operand(dx),
            Value::operand(dx),
        ));

    // Calculate dy * dy
    let dy_squared = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Mul,
            dy_squared,
            Value::operand(dy),
            Value::operand(dy),
        ));

    // Calculate result = dx*dx + dy*dy
    let result = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            result,
            Value::operand(dx_squared),
            Value::operand(dy_squared),
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    println!("Before SROA:");
    for inst in &function.get_basic_block(entry).unwrap().instructions {
        println!("  {}", inst.pretty_print(0));
    }

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut function);
    assert!(modified, "SROA should modify the function");

    println!("\nAfter SROA:");
    for inst in &function.get_basic_block(entry).unwrap().instructions {
        println!("  {}", inst.pretty_print(0));
    }

    // Verify the result is still correct - all operations should work
    let block = function.get_basic_block(entry).unwrap();

    // Check that nested struct operations were handled correctly
    // The issue is that when we extract a nested struct (like line.end),
    // and that struct itself was scalarized, we need to ensure the
    // extraction returns something that can be further extracted from

    // Verify no dangling references
    for inst in &block.instructions {
        if let InstructionKind::ExtractStructField { struct_val, .. } = &inst.kind {
            if let Value::Operand(id) = struct_val {
                // Check if this ID is defined somewhere
                let is_defined = block
                    .instructions
                    .iter()
                    .any(|i| i.destination() == Some(*id))
                    || function.parameters.contains(id);

                assert!(
                    is_defined,
                    "ExtractStructField references undefined value {id:?}"
                );
            }
        }
    }
}
