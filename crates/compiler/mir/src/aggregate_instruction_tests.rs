//! Unit tests for aggregate instructions

#[cfg(test)]
mod tests {
    use crate::{Instruction, InstructionKind, MirFunction, MirType, Value, ValueId};

    #[test]
    fn test_make_tuple_validation() {
        let mut function = MirFunction::new("test".to_string());
        let dest = function.new_value_id();
        let elem1 = function.new_value_id();
        let elem2 = function.new_value_id();

        let instr =
            Instruction::make_tuple(dest, vec![Value::operand(elem1), Value::operand(elem2)]);

        // Check destination
        assert_eq!(instr.destination(), Some(dest));

        // Check used values
        let used = instr.used_values();
        assert!(used.contains(&elem1));
        assert!(used.contains(&elem2));
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn test_extract_tuple_bounds() {
        let mut function = MirFunction::new("test".to_string());
        let dest = function.new_value_id();
        let tuple_val = function.new_value_id();

        // Valid extraction
        let instr =
            Instruction::extract_tuple_element(dest, Value::operand(tuple_val), 0, MirType::felt());

        assert_eq!(instr.destination(), Some(dest));
        assert!(instr.used_values().contains(&tuple_val));

        // Test with different indices
        let instr2 = Instruction::extract_tuple_element(
            dest,
            Value::operand(tuple_val),
            5, // High index - validation should happen at semantic level
            MirType::felt(),
        );

        if let InstructionKind::ExtractTupleElement { index, .. } = &instr2.kind {
            assert_eq!(*index, 5);
        } else {
            panic!("Wrong instruction kind");
        }
    }

    #[test]
    fn test_make_struct_field_ordering() {
        let mut function = MirFunction::new("test".to_string());
        let dest = function.new_value_id();
        let x_val = function.new_value_id();
        let y_val = function.new_value_id();

        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };

        let instr = Instruction::make_struct(
            dest,
            vec![
                ("y".to_string(), Value::operand(y_val)), // Different order
                ("x".to_string(), Value::operand(x_val)),
            ],
            struct_type,
        );

        // Check that both values are used
        let used = instr.used_values();
        assert!(used.contains(&x_val));
        assert!(used.contains(&y_val));
    }

    #[test]
    fn test_extract_struct_field() {
        let mut function = MirFunction::new("test".to_string());
        let dest = function.new_value_id();
        let struct_val = function.new_value_id();

        let instr = Instruction::extract_struct_field(
            dest,
            Value::operand(struct_val),
            "field_name".to_string(),
            MirType::felt(),
        );

        assert_eq!(instr.destination(), Some(dest));
        assert!(instr.used_values().contains(&struct_val));

        // Check field name is preserved
        if let InstructionKind::ExtractStructField { field_name, .. } = &instr.kind {
            assert_eq!(field_name, "field_name");
        } else {
            panic!("Wrong instruction kind");
        }
    }

    #[test]
    fn test_insert_field_creates_new_value() {
        let mut function = MirFunction::new("test".to_string());
        let original = function.new_value_id();
        let updated = function.new_value_id();
        let new_value = function.new_value_id();

        let struct_type = MirType::Struct {
            name: "Data".to_string(),
            fields: vec![("value".to_string(), MirType::felt())],
        };

        let instr = Instruction::insert_field(
            updated,
            Value::operand(original),
            "value".to_string(),
            Value::operand(new_value),
            struct_type,
        );

        // InsertField creates a new value (functional update)
        assert_eq!(instr.destination(), Some(updated));
        assert_ne!(updated, original);

        // Both original and new value are used
        let used = instr.used_values();
        assert!(used.contains(&original));
        assert!(used.contains(&new_value));
    }

    #[test]
    fn test_insert_tuple_creates_new_value() {
        let mut function = MirFunction::new("test".to_string());
        let original = function.new_value_id();
        let updated = function.new_value_id();
        let new_value = function.new_value_id();

        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);

        let instr = Instruction::insert_tuple(
            updated,
            Value::operand(original),
            0,
            Value::operand(new_value),
            tuple_type,
        );

        // InsertTuple creates a new value (functional update)
        assert_eq!(instr.destination(), Some(updated));
        assert_ne!(updated, original);

        // Both original and new value are used
        let used = instr.used_values();
        assert!(used.contains(&original));
        assert!(used.contains(&new_value));
    }

    #[test]
    fn test_aggregate_with_literals() {
        let mut function = MirFunction::new("test".to_string());
        let dest = function.new_value_id();

        // Tuple with literal values
        let instr = Instruction::make_tuple(
            dest,
            vec![Value::integer(42), Value::boolean(true), Value::unit()],
        );

        // No value IDs should be in used_values for literals
        assert_eq!(instr.used_values().len(), 0);
        assert_eq!(instr.destination(), Some(dest));
    }

    #[test]
    fn test_nested_aggregate_types() {
        let mut function = MirFunction::new("test".to_string());

        // Create nested type: Struct { tuple: (felt, felt), value: felt }
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let struct_type = MirType::Struct {
            name: "Nested".to_string(),
            fields: vec![
                ("tuple".to_string(), tuple_type.clone()),
                ("value".to_string(), MirType::felt()),
            ],
        };

        // Verify type structure
        if let MirType::Struct { fields, .. } = &struct_type {
            assert_eq!(fields.len(), 2);
            assert!(matches!(&fields[0].1, MirType::Tuple(_)));
        }
    }

    #[test]
    fn test_empty_aggregates() {
        let mut function = MirFunction::new("test".to_string());
        let empty_tuple = function.new_value_id();
        let empty_struct = function.new_value_id();

        // Empty tuple
        let tuple_instr = Instruction::make_tuple(empty_tuple, vec![]);
        assert_eq!(tuple_instr.destination(), Some(empty_tuple));
        assert_eq!(tuple_instr.used_values().len(), 0);

        // Empty struct
        let empty_struct_type = MirType::Struct {
            name: "Empty".to_string(),
            fields: vec![],
        };

        let struct_instr = Instruction::make_struct(empty_struct, vec![], empty_struct_type);
        assert_eq!(struct_instr.destination(), Some(empty_struct));
        assert_eq!(struct_instr.used_values().len(), 0);
    }

    #[test]
    fn test_instruction_cloning() {
        let mut function = MirFunction::new("test".to_string());
        let dest = function.new_value_id();
        let elem = function.new_value_id();

        let original = Instruction::make_tuple(dest, vec![Value::operand(elem)]);

        let cloned = original.clone();

        // Ensure deep equality
        assert_eq!(original.destination(), cloned.destination());
        assert_eq!(original.used_values(), cloned.used_values());

        // Check they're actually different instances
        assert!(!std::ptr::eq(&original, &cloned));
    }
}
