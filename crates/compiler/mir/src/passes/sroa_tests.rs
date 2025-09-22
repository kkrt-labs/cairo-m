//! Tests for the SROA pass

use crate::instruction::InstructionKind;
use crate::passes::sroa::*;
use crate::{
    BinaryOp, Instruction, Literal, MirFunction, MirModule, MirType, Place, Terminator, Value,
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
    let has_assigns = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::Assign { .. }));
    assert!(has_assigns, "Should have assign instructions");
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

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut function);
    assert!(modified, "SROA should modify the function");

    // Verify the result is still correct - all operations should work
    let block = function.get_basic_block(entry).unwrap();

    // Check that nested struct operations were handled correctly
    // The issue is that when we extract a nested struct (like line.end),
    // and that struct itself was scalarized, we need to ensure the
    // extraction returns something that can be further extracted from

    // Verify no dangling references
    for inst in &block.instructions {
        if let InstructionKind::ExtractStructField {
            struct_val: Value::Operand(id),
            ..
        } = &inst.kind
        {
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

#[test]
fn test_dynamic_array_indexing_prevents_sroa() {
    // Test that arrays with dynamic indexing are NOT scalarized
    let mut function = MirFunction::new("test_dynamic_array".to_string());
    let entry = function.entry_block;

    let array_ty = MirType::FixedArray {
        element_type: Box::new(MirType::Felt),
        size: 3,
    };

    // Create array elements
    let elem0 = function.new_typed_value_id(MirType::Felt);
    let elem1 = function.new_typed_value_id(MirType::Felt);
    let elem2 = function.new_typed_value_id(MirType::Felt);

    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            elem0,
            Value::Literal(Literal::Integer(10)),
            MirType::Felt,
        ));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            elem1,
            Value::Literal(Literal::Integer(20)),
            MirType::Felt,
        ));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            elem2,
            Value::Literal(Literal::Integer(30)),
            MirType::Felt,
        ));

    // Create the array: arr = [10, 20, 30]
    let arr = function.new_typed_value_id(array_ty);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            arr,
            vec![
                Value::operand(elem0),
                Value::operand(elem1),
                Value::operand(elem2),
            ],
            MirType::Felt,
        ));

    // Create a runtime index value
    let index = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            index,
            Value::Literal(Literal::Integer(1)),
            MirType::Felt,
        ));

    // Dynamic array indexing: result = arr[index]
    let result = function.new_typed_value_id(MirType::Felt);
    let load_place = Place::new(arr).with_index(Value::operand(index));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::load(result, load_place, MirType::Felt));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let _modified = sroa.run(&mut function);

    // The function may be modified for other reasons, but the array should NOT be scalarized
    let block = function.get_basic_block(entry).unwrap();

    // The MakeFixedArray instruction should still be present
    let has_make_array = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeFixedArray { .. }));
    assert!(
        has_make_array,
        "MakeFixedArray should NOT be eliminated when dynamic indexing is present"
    );

    // The Load instruction should still be present
    let has_load = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::Load { .. }));
    assert!(has_load, "Load should remain");
}

#[test]
fn test_array_family_with_dynamic_indexing() {
    // Test that the entire SSA family is preserved when any member has dynamic indexing
    let mut function = MirFunction::new("test_array_family".to_string());
    let entry = function.entry_block;

    let array_ty = MirType::FixedArray {
        element_type: Box::new(MirType::Felt),
        size: 2,
    };

    // Create initial array: arr1 = [1, 2]
    let arr1 = function.new_typed_value_id(array_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            arr1,
            vec![
                Value::Literal(Literal::Integer(1)),
                Value::Literal(Literal::Integer(2)),
            ],
            MirType::Felt,
        ));

    // Copy array: arr2 = arr1
    let arr2 = function.new_typed_value_id(array_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            arr2,
            Value::operand(arr1),
            array_ty.clone(),
        ));

    // Update array in place: arr2[0] := 10
    let store_place = Place::new(arr2).with_index(Value::Literal(Literal::Integer(0)));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::store(
            store_place,
            Value::Literal(Literal::Integer(10)),
            MirType::Felt,
        ));

    // Copy array: arr3 = arr2
    let arr3 = function.new_typed_value_id(array_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(arr3, Value::operand(arr2), array_ty));

    // Dynamic indexing on arr3 (part of the family)
    let index = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::assign(
            index,
            Value::Literal(Literal::Integer(0)),
            MirType::Felt,
        ));

    let result = function.new_typed_value_id(MirType::Felt);
    let dyn_place = Place::new(arr3).with_index(Value::operand(index));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::load(result, dyn_place, MirType::Felt));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    sroa.run(&mut function);

    let block = function.get_basic_block(entry).unwrap();

    // All array operations in the family should be preserved
    let has_make_array = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeFixedArray { .. }));
    assert!(
        has_make_array,
        "MakeFixedArray should be preserved (family has dynamic indexing)"
    );

    let has_store = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::Store { .. }));
    assert!(
        has_store,
        "Store should be preserved (family has dynamic indexing)"
    );

    let array_assign_count = block
        .instructions
        .iter()
        .filter(|inst| {
            if let InstructionKind::Assign { ty, .. } = &inst.kind {
                matches!(ty, MirType::FixedArray { .. })
            } else {
                false
            }
        })
        .count();
    assert!(
        array_assign_count > 0,
        "Array assignments should be preserved (family has dynamic indexing)"
    );
}

#[test]
fn test_array_without_dynamic_indexing_is_scalarized() {
    // Test that arrays WITHOUT dynamic indexing CAN be scalarized
    let mut function = MirFunction::new("test_static_array".to_string());
    let entry = function.entry_block;

    let array_ty = MirType::FixedArray {
        element_type: Box::new(MirType::Felt),
        size: 2,
    };

    // Create array: arr = [10, 20]
    let arr = function.new_typed_value_id(array_ty);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            arr,
            vec![
                Value::Literal(Literal::Integer(10)),
                Value::Literal(Literal::Integer(20)),
            ],
            MirType::Felt,
        ));

    // Static indexing only: elem0 = arr[0], elem1 = arr[1]
    let elem0 = function.new_typed_value_id(MirType::Felt);
    let elem0_place = Place::new(arr).with_index(Value::Literal(Literal::Integer(0)));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::load(elem0, elem0_place, MirType::Felt));

    let elem1 = function.new_typed_value_id(MirType::Felt);
    let elem1_place = Place::new(arr).with_index(Value::Literal(Literal::Integer(1)));
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::load(elem1, elem1_place, MirType::Felt));

    // Add the elements
    let sum = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            sum,
            Value::operand(elem0),
            Value::operand(elem1),
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(sum)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut function);
    assert!(
        modified,
        "SROA should modify arrays with only static indexing"
    );

    let block = function.get_basic_block(entry).unwrap();

    // The MakeFixedArray should be eliminated (scalarized)
    let has_make_array = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeFixedArray { .. }));
    assert!(
        !has_make_array,
        "MakeFixedArray should be eliminated for static-only arrays"
    );

    // Extract operations should be replaced with assigns
    let has_load = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::Load { .. }));
    assert!(!has_load, "Load should be replaced with assigns");
}

#[test]
fn test_array_in_struct_not_scalarized() {
    // Test that arrays used as struct fields are properly materialized
    let mut function = MirFunction::new("test_array_in_struct".to_string());
    let entry = function.entry_block;

    // Create array elements
    let a = function.new_typed_value_id(MirType::Felt);
    let b = function.new_typed_value_id(MirType::Felt);

    // Make array: arr = [a, b]
    let array_ty = MirType::FixedArray {
        element_type: Box::new(MirType::Felt),
        size: 2,
    };
    let arr = function.new_typed_value_id(array_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            arr,
            vec![Value::operand(a), Value::operand(b)],
            MirType::Felt,
        ));

    // Create sum value
    let sum = function.new_typed_value_id(MirType::Felt);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            sum,
            Value::operand(a),
            Value::operand(b),
        ));

    // Make struct: result = { values: arr, sum: sum }
    let struct_ty = MirType::Struct {
        name: "Result".to_string(),
        fields: vec![
            ("values".to_string(), array_ty),
            ("sum".to_string(), MirType::Felt),
        ],
    };
    let result = function.new_typed_value_id(struct_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            result,
            vec![
                ("values".to_string(), Value::operand(arr)),
                ("sum".to_string(), Value::operand(sum)),
            ],
            struct_ty,
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(result)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let _modified = sroa.run(&mut function);

    // The array should NOT be scalarized when used in a struct
    let block = function.get_basic_block(entry).unwrap();
    let has_make_array = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeFixedArray { .. }));
    assert!(
        has_make_array,
        "MakeFixedArray should NOT be eliminated when array is used in struct"
    );
}

#[test]
fn test_array_in_tuple_not_scalarized() {
    // Test that arrays used as tuple elements are properly materialized
    let mut function = MirFunction::new("test_array_in_tuple".to_string());
    let entry = function.entry_block;

    // Create array elements
    let x = function.new_typed_value_id(MirType::Felt);
    let y = function.new_typed_value_id(MirType::Felt);

    // Make array: arr = [x, y]
    let array_ty = MirType::FixedArray {
        element_type: Box::new(MirType::Felt),
        size: 2,
    };
    let arr = function.new_typed_value_id(array_ty.clone());
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            arr,
            vec![Value::operand(x), Value::operand(y)],
            MirType::Felt,
        ));

    // Make tuple: t = (x, arr)
    let tuple_ty = MirType::tuple(vec![MirType::Felt, array_ty]);
    let t = function.new_typed_value_id(tuple_ty);
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .push_instruction(Instruction::make_tuple(
            t,
            vec![Value::operand(x), Value::operand(arr)],
        ));

    // Set terminator
    function.get_basic_block_mut(entry).unwrap().terminator = Terminator::Return {
        values: vec![Value::operand(t)],
    };

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let _modified = sroa.run(&mut function);

    // The array should NOT be scalarized when used in a tuple
    let block = function.get_basic_block(entry).unwrap();
    let has_make_array = block
        .instructions
        .iter()
        .any(|inst| matches!(inst.kind, InstructionKind::MakeFixedArray { .. }));
    assert!(
        has_make_array,
        "MakeFixedArray should NOT be eliminated when array is used in tuple"
    );
}

#[test]
fn test_store_materializes_tracked_struct_for_pointer_base() {
    // Scenario: We build a struct value, SROA tracks it and removes the MakeStruct.
    // Then we store that value into a pointer base with index projection (heap memory).
    // Forwarding is not possible (base not tracked), so SROA must materialize the
    // aggregate right before the Store and rewrite the Store to use it.

    // Define struct type: Point { x: felt, y: felt }
    let point_ty = MirType::Struct {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MirType::Felt),
            ("y".to_string(), MirType::Felt),
        ],
    };

    // Pointer to Point (heap-allocated)
    let ptr_to_point = MirType::Pointer {
        element: Box::new(point_ty.clone()),
    };

    let mut f = MirFunction::new("store_materialization".to_string());
    let b = f.entry_block;

    // Allocate some heap cells and get a pointer (typed as *Point)
    let heap_ptr = f.new_typed_value_id(ptr_to_point);
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::heap_alloc_cells(
            heap_ptr,
            Value::Literal(Literal::Integer(4)),
        ));

    // Build two Point values
    let p0 = f.new_typed_value_id(point_ty.clone());
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            p0,
            vec![
                ("x".to_string(), Value::Literal(Literal::Integer(1))),
                ("y".to_string(), Value::Literal(Literal::Integer(2))),
            ],
            point_ty.clone(),
        ));

    let p1 = f.new_typed_value_id(point_ty.clone());
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::make_struct(
            p1,
            vec![
                ("x".to_string(), Value::Literal(Literal::Integer(3))),
                ("y".to_string(), Value::Literal(Literal::Integer(4))),
            ],
            point_ty.clone(),
        ));

    // Store the points into heap_ptr[0] and heap_ptr[1]
    let place0 = Place::new(heap_ptr).with_index(Value::Literal(Literal::Integer(0)));
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::store(
            place0,
            Value::operand(p0),
            point_ty.clone(),
        ));

    let place1 = Place::new(heap_ptr).with_index(Value::Literal(Literal::Integer(1)));
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::store(place1, Value::operand(p1), point_ty));

    // No return value needed for this pass-level test
    f.get_basic_block_mut(b)
        .unwrap()
        .set_terminator(Terminator::Return { values: vec![] });

    // Run SROA
    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut f);
    assert!(modified, "SROA should modify the function");

    let block = f.get_basic_block(b).unwrap();

    // Collect MakeStructs and Stores
    let make_struct_ids: Vec<_> = block
        .instructions
        .iter()
        .filter_map(|inst| match &inst.kind {
            InstructionKind::MakeStruct { dest, .. } => Some(*dest),
            _ => None,
        })
        .collect();

    let store_infos: Vec<(usize, crate::ValueId)> = block
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(i, inst)| match &inst.kind {
            InstructionKind::Store { value, .. } => value.as_operand().map(|id| (i, id)),
            _ => None,
        })
        .collect();

    // We expect two Stores and two preceding MakeStruct materializations
    assert_eq!(store_infos.len(), 2, "Should have exactly two stores");
    assert_eq!(
        make_struct_ids.len(),
        2,
        "Should materialize exactly two structs before stores"
    );

    // Each store should use one of the materialized ids, and that id must be defined earlier
    for (store_idx, used_id) in &store_infos {
        assert!(
            make_struct_ids.contains(used_id),
            "Store should use a freshly materialized struct value"
        );

        // Find the defining MakeStruct and ensure it appears before the Store
        let def_idx = block
            .instructions
            .iter()
            .position(|inst| matches!(inst.kind, InstructionKind::MakeStruct { dest, .. } if dest == *used_id))
            .expect("Materialized struct must be defined in the block");
        assert!(def_idx < *store_idx, "Materialization must precede Store");
    }

    // Ensure original aggregate ids (p0, p1) are no longer used after SROA
    for inst in &block.instructions {
        let used = inst.used_values();
        assert!(
            !used.contains(&p0) && !used.contains(&p1),
            "Original aggregate ids should not be used after SROA"
        );
    }
}

#[test]
fn test_store_materializes_tracked_tuple_for_pointer_base() {
    // Tuple type (felt, felt)
    let tuple_ty = MirType::tuple(vec![MirType::Felt, MirType::Felt]);
    let ptr_to_tuple = MirType::Pointer {
        element: Box::new(tuple_ty.clone()),
    };

    let mut f = MirFunction::new("store_tuple_mat".to_string());
    let b = f.entry_block;

    // pointer to tuple
    let heap_ptr = f.new_typed_value_id(ptr_to_tuple);
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::heap_alloc_cells(
            heap_ptr,
            Value::Literal(Literal::Integer(4)),
        ));

    // tracked tuple values
    let t0 = f.new_typed_value_id(tuple_ty.clone());
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::make_tuple(
            t0,
            vec![
                Value::Literal(Literal::Integer(1)),
                Value::Literal(Literal::Integer(2)),
            ],
        ));
    let t1 = f.new_typed_value_id(tuple_ty.clone());
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::make_tuple(
            t1,
            vec![
                Value::Literal(Literal::Integer(3)),
                Value::Literal(Literal::Integer(4)),
            ],
        ));

    // store tuples into memory
    let p0 = Place::new(heap_ptr).with_index(Value::Literal(Literal::Integer(0)));
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::store(p0, Value::operand(t0), tuple_ty.clone()));
    let p1 = Place::new(heap_ptr).with_index(Value::Literal(Literal::Integer(1)));
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::store(p1, Value::operand(t1), tuple_ty));

    f.get_basic_block_mut(b)
        .unwrap()
        .set_terminator(Terminator::Return { values: vec![] });

    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut f);
    assert!(modified);

    let block = f.get_basic_block(b).unwrap();
    let make_tuple_ids: Vec<_> = block
        .instructions
        .iter()
        .filter_map(|inst| match &inst.kind {
            InstructionKind::MakeTuple { dest, .. } => Some(*dest),
            _ => None,
        })
        .collect();
    let store_vals: Vec<_> = block
        .instructions
        .iter()
        .filter_map(|inst| match &inst.kind {
            InstructionKind::Store { value, .. } => value.as_operand(),
            _ => None,
        })
        .collect();

    assert_eq!(store_vals.len(), 2);
    assert_eq!(make_tuple_ids.len(), 2);
    for used in store_vals {
        assert!(make_tuple_ids.contains(&used));
    }
}

#[test]
fn test_store_materializes_tracked_array_for_pointer_base() {
    // Fixed array type [felt; 2]
    let array_ty = MirType::FixedArray {
        element_type: Box::new(MirType::Felt),
        size: 2,
    };
    let ptr_to_array = MirType::Pointer {
        element: Box::new(array_ty.clone()),
    };

    let mut f = MirFunction::new("store_array_mat".to_string());
    let b = f.entry_block;

    // pointer to array
    let heap_ptr = f.new_typed_value_id(ptr_to_array);
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::heap_alloc_cells(
            heap_ptr,
            Value::Literal(Literal::Integer(4)),
        ));

    // tracked arrays
    let a0 = f.new_typed_value_id(array_ty.clone());
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            a0,
            vec![
                Value::Literal(Literal::Integer(10)),
                Value::Literal(Literal::Integer(20)),
            ],
            MirType::Felt,
        ));
    let a1 = f.new_typed_value_id(array_ty.clone());
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::make_fixed_array(
            a1,
            vec![
                Value::Literal(Literal::Integer(30)),
                Value::Literal(Literal::Integer(40)),
            ],
            MirType::Felt,
        ));

    // store arrays into memory
    let p0 = Place::new(heap_ptr).with_index(Value::Literal(Literal::Integer(0)));
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::store(p0, Value::operand(a0), array_ty.clone()));
    let p1 = Place::new(heap_ptr).with_index(Value::Literal(Literal::Integer(1)));
    f.get_basic_block_mut(b)
        .unwrap()
        .push_instruction(Instruction::store(p1, Value::operand(a1), array_ty));

    f.get_basic_block_mut(b)
        .unwrap()
        .set_terminator(Terminator::Return { values: vec![] });

    let mut sroa = ScalarReplacementOfAggregates::new();
    let modified = sroa.run(&mut f);
    assert!(modified);

    let block = f.get_basic_block(b).unwrap();
    let make_arr_ids: Vec<_> = block
        .instructions
        .iter()
        .filter_map(|inst| match &inst.kind {
            InstructionKind::MakeFixedArray { dest, .. } => Some(*dest),
            _ => None,
        })
        .collect();
    let store_vals: Vec<_> = block
        .instructions
        .iter()
        .filter_map(|inst| match &inst.kind {
            InstructionKind::Store { value, .. } => value.as_operand(),
            _ => None,
        })
        .collect();

    assert_eq!(store_vals.len(), 2);
    assert_eq!(make_arr_ids.len(), 2);
    for used in store_vals {
        assert!(make_arr_ids.contains(&used));
    }
}
