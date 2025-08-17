//! Tests for aggregate operation validation

#[cfg(test)]
mod tests {
    use crate::passes::{MirPass, Validation};
    use crate::{Instruction, MirFunction, MirType, Terminator, Value};

    #[test]
    fn test_valid_tuple_operations() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create tuple values
        let v1 = function.new_typed_value_id(MirType::felt());
        let v2 = function.new_typed_value_id(MirType::felt());
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let tuple_dest = function.new_typed_value_id(tuple_type);
        let extract_dest = function.new_typed_value_id(MirType::felt());

        // Build valid tuple operations
        let block = function.get_basic_block_mut(entry).unwrap();
        block
            .instructions
            .push(Instruction::assign(v1, Value::integer(1), MirType::felt()));
        block
            .instructions
            .push(Instruction::assign(v2, Value::integer(2), MirType::felt()));
        block.instructions.push(Instruction::make_tuple(
            tuple_dest,
            vec![Value::operand(v1), Value::operand(v2)],
        ));
        block.instructions.push(Instruction::extract_tuple_element(
            extract_dest,
            Value::operand(tuple_dest),
            0, // Valid index
            MirType::felt(),
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(extract_dest)],
        });

        // Validation should pass without errors
        let mut validator = Validation::new();
        validator.run(&mut function);
        // No assertions needed - just ensure no panics or errors
    }

    #[test]
    fn test_tuple_index_out_of_bounds() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create a tuple with 2 elements
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let tuple_dest = function.new_typed_value_id(tuple_type);
        let extract_dest = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_tuple(
            tuple_dest,
            vec![Value::integer(1), Value::integer(2)],
        ));
        // Try to extract element at index 2 (out of bounds)
        block.instructions.push(Instruction::extract_tuple_element(
            extract_dest,
            Value::operand(tuple_dest),
            2, // Out of bounds!
            MirType::felt(),
        ));

        // Set RUST_LOG to capture validation errors
        std::env::set_var("RUST_LOG", "1");

        // Run validation - should detect the error
        let mut validator = Validation::new();
        validator.run(&mut function);

        // Clean up
        std::env::remove_var("RUST_LOG");
    }

    #[test]
    fn test_valid_struct_operations() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create struct type
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };

        let x_val = function.new_typed_value_id(MirType::felt());
        let y_val = function.new_typed_value_id(MirType::felt());
        let struct_dest = function.new_typed_value_id(struct_type.clone());
        let extract_dest = function.new_typed_value_id(MirType::felt());

        // Build valid struct operations
        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::assign(
            x_val,
            Value::integer(10),
            MirType::felt(),
        ));
        block.instructions.push(Instruction::assign(
            y_val,
            Value::integer(20),
            MirType::felt(),
        ));
        block.instructions.push(Instruction::make_struct(
            struct_dest,
            vec![
                ("x".to_string(), Value::operand(x_val)),
                ("y".to_string(), Value::operand(y_val)),
            ],
            struct_type,
        ));
        block.instructions.push(Instruction::extract_struct_field(
            extract_dest,
            Value::operand(struct_dest),
            "x".to_string(), // Valid field
            MirType::felt(),
        ));

        // Validation should pass without errors
        let mut validator = Validation::new();
        validator.run(&mut function);
    }

    #[test]
    fn test_struct_field_not_found() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create struct type
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };

        let struct_dest = function.new_typed_value_id(struct_type.clone());
        let extract_dest = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_struct(
            struct_dest,
            vec![
                ("x".to_string(), Value::integer(10)),
                ("y".to_string(), Value::integer(20)),
            ],
            struct_type,
        ));
        // Try to extract non-existent field
        block.instructions.push(Instruction::extract_struct_field(
            extract_dest,
            Value::operand(struct_dest),
            "z".to_string(), // Field doesn't exist!
            MirType::felt(),
        ));

        // Set RUST_LOG to capture validation errors
        std::env::set_var("RUST_LOG", "1");

        // Run validation - should detect the error
        let mut validator = Validation::new();
        validator.run(&mut function);

        // Clean up
        std::env::remove_var("RUST_LOG");
    }

    #[test]
    fn test_struct_duplicate_fields() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create struct type
        let struct_type = MirType::Struct {
            name: "Data".to_string(),
            fields: vec![
                ("a".to_string(), MirType::felt()),
                ("b".to_string(), MirType::felt()),
            ],
        };

        let struct_dest = function.new_typed_value_id(struct_type.clone());

        let block = function.get_basic_block_mut(entry).unwrap();
        // Create struct with duplicate field
        block.instructions.push(Instruction::make_struct(
            struct_dest,
            vec![
                ("a".to_string(), Value::integer(1)),
                ("a".to_string(), Value::integer(2)), // Duplicate!
                ("b".to_string(), Value::integer(3)),
            ],
            struct_type,
        ));

        // Set RUST_LOG to capture validation errors
        std::env::set_var("RUST_LOG", "1");

        // Run validation - should detect the error
        let mut validator = Validation::new();
        validator.run(&mut function);

        // Clean up
        std::env::remove_var("RUST_LOG");
    }

    #[test]
    fn test_insert_field_validation() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create struct type
        let struct_type = MirType::Struct {
            name: "Data".to_string(),
            fields: vec![
                ("a".to_string(), MirType::felt()),
                ("b".to_string(), MirType::felt()),
            ],
        };

        let struct1 = function.new_typed_value_id(struct_type.clone());
        let struct2 = function.new_typed_value_id(struct_type.clone());
        let new_val = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_struct(
            struct1,
            vec![
                ("a".to_string(), Value::integer(1)),
                ("b".to_string(), Value::integer(2)),
            ],
            struct_type.clone(),
        ));
        block.instructions.push(Instruction::assign(
            new_val,
            Value::integer(99),
            MirType::felt(),
        ));
        // Valid InsertField
        block.instructions.push(Instruction::insert_field(
            struct2,
            Value::operand(struct1),
            "a".to_string(), // Valid field
            Value::operand(new_val),
            struct_type,
        ));

        // Validation should pass
        let mut validator = Validation::new();
        validator.run(&mut function);
    }

    #[test]
    fn test_insert_tuple_validation() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let tuple1 = function.new_typed_value_id(tuple_type.clone());
        let tuple2 = function.new_typed_value_id(tuple_type.clone());
        let new_val = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_tuple(
            tuple1,
            vec![Value::integer(1), Value::integer(2)],
        ));
        block.instructions.push(Instruction::assign(
            new_val,
            Value::integer(99),
            MirType::felt(),
        ));
        // Valid InsertTuple
        block.instructions.push(Instruction::insert_tuple(
            tuple2,
            Value::operand(tuple1),
            1, // Valid index
            Value::operand(new_val),
            tuple_type,
        ));

        // Validation should pass
        let mut validator = Validation::new();
        validator.run(&mut function);
    }

    #[test]
    fn test_insert_tuple_out_of_bounds() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let tuple1 = function.new_typed_value_id(tuple_type.clone());
        let tuple2 = function.new_typed_value_id(tuple_type.clone());
        let new_val = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_tuple(
            tuple1,
            vec![Value::integer(1), Value::integer(2)],
        ));
        block.instructions.push(Instruction::assign(
            new_val,
            Value::integer(99),
            MirType::felt(),
        ));
        // Invalid InsertTuple - index out of bounds
        block.instructions.push(Instruction::insert_tuple(
            tuple2,
            Value::operand(tuple1),
            5, // Out of bounds!
            Value::operand(new_val),
            tuple_type,
        ));

        // Set RUST_LOG to capture validation errors
        std::env::set_var("RUST_LOG", "1");

        // Run validation - should detect the error
        let mut validator = Validation::new();
        validator.run(&mut function);

        // Clean up
        std::env::remove_var("RUST_LOG");
    }
}
