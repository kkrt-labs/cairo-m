//! Tests for InsertField and InsertTuple instructions

#[cfg(test)]
mod tests {
    use crate::{Instruction, MirFunction, MirType, PrettyPrint, Value};

    #[test]
    fn test_insert_field_instruction() {
        let mut function = MirFunction::new("test".to_string());
        let _entry = function.entry_block;

        // Create a struct value
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };
        let struct_val = function.new_typed_value_id(struct_type.clone());

        // Create new field value
        let new_x = Value::integer(42);

        // Create InsertField instruction
        let updated_struct = function.new_typed_value_id(struct_type.clone());
        let insert_instr = Instruction::insert_field(
            updated_struct,
            Value::operand(struct_val),
            "x".to_string(),
            new_x,
            struct_type,
        );

        // Verify instruction properties
        assert_eq!(insert_instr.destinations(), vec![updated_struct]);
        assert!(insert_instr.used_values().contains(&struct_val));
        assert!(insert_instr.validate().is_ok());

        // Verify pretty printing
        let pretty = insert_instr.kind.into_instruction().pretty_print(0);
        assert!(pretty.contains("insertfield"));
        assert!(pretty.contains("\"x\""));
    }

    #[test]
    fn test_insert_tuple_instruction() {
        let mut function = MirFunction::new("test".to_string());
        let _entry = function.entry_block;

        // Create a tuple value
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt(), MirType::felt()]);
        let tuple_val = function.new_typed_value_id(tuple_type.clone());

        // Create new element value
        let new_elem = Value::integer(99);

        // Create InsertTuple instruction
        let updated_tuple = function.new_typed_value_id(tuple_type.clone());
        let insert_instr = Instruction::insert_tuple(
            updated_tuple,
            Value::operand(tuple_val),
            1, // Update index 1
            new_elem,
            tuple_type,
        );

        // Verify instruction properties
        assert_eq!(insert_instr.destinations(), vec![updated_tuple]);
        assert!(insert_instr.used_values().contains(&tuple_val));
        assert!(insert_instr.validate().is_ok());

        // Verify pretty printing
        let pretty = insert_instr.kind.into_instruction().pretty_print(0);
        assert!(pretty.contains("inserttuple"));
        assert!(pretty.contains(", 1,")); // Index 1
    }

    #[test]
    fn test_insert_field_used_values() {
        let mut function = MirFunction::new("test".to_string());

        let struct_type = MirType::Struct {
            name: "Data".to_string(),
            fields: vec![("value".to_string(), MirType::felt())],
        };

        let struct_val = function.new_typed_value_id(struct_type.clone());
        let field_val = function.new_typed_value_id(MirType::felt());
        let dest = function.new_typed_value_id(struct_type.clone());

        let instr = Instruction::insert_field(
            dest,
            Value::operand(struct_val),
            "value".to_string(),
            Value::operand(field_val),
            struct_type,
        );

        let used = instr.used_values();
        assert!(used.contains(&struct_val));
        assert!(used.contains(&field_val));
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn test_insert_tuple_used_values() {
        let mut function = MirFunction::new("test".to_string());

        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let tuple_val = function.new_typed_value_id(tuple_type.clone());
        let elem_val = function.new_typed_value_id(MirType::felt());
        let dest = function.new_typed_value_id(tuple_type.clone());

        let instr = Instruction::insert_tuple(
            dest,
            Value::operand(tuple_val),
            0,
            Value::operand(elem_val),
            tuple_type,
        );

        let used = instr.used_values();
        assert!(used.contains(&tuple_val));
        assert!(used.contains(&elem_val));
        assert_eq!(used.len(), 2);
    }
}

// Helper to turn InstructionKind into an Instruction for testing
impl crate::InstructionKind {
    fn into_instruction(self) -> crate::Instruction {
        crate::Instruction {
            kind: self,
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }
}
