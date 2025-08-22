#[cfg(test)]
mod aggregate_instruction_tests {
    use crate::{Instruction, InstructionKind, MirType, Value, ValueId};

    #[test]
    fn test_make_tuple_instruction() {
        let dest = ValueId::new(0);
        let elem1 = Value::Operand(ValueId::new(1));
        let elem2 = Value::Operand(ValueId::new(2));
        let elements = vec![elem1, elem2];

        let instr = Instruction::make_tuple(dest, elements.clone());

        match &instr.kind {
            InstructionKind::MakeTuple {
                dest: d,
                elements: e,
            } => {
                assert_eq!(*d, dest);
                assert_eq!(*e, elements);
            }
            _ => panic!("Expected MakeTuple instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
        assert!(instr.used_values().contains(&ValueId::new(2)));
    }

    #[test]
    fn test_extract_tuple_element_instruction() {
        let dest = ValueId::new(0);
        let tuple = Value::Operand(ValueId::new(1));
        let index = 1;
        let element_ty = MirType::felt();

        let instr = Instruction::extract_tuple_element(dest, tuple, index, element_ty.clone());

        match &instr.kind {
            InstructionKind::ExtractTupleElement {
                dest: d,
                tuple: t,
                index: i,
                element_ty: ty,
            } => {
                assert_eq!(*d, dest);
                assert_eq!(*t, tuple);
                assert_eq!(*i, index);
                assert_eq!(*ty, element_ty);
            }
            _ => panic!("Expected ExtractTupleElement instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
    }

    #[test]
    fn test_make_struct_instruction() {
        let dest = ValueId::new(0);
        let fields = vec![
            ("x".to_string(), Value::Operand(ValueId::new(1))),
            ("y".to_string(), Value::Operand(ValueId::new(2))),
        ];
        let struct_ty = MirType::simple_struct_type("Point".to_string());

        let instr = Instruction::make_struct(dest, fields.clone(), struct_ty.clone());

        match &instr.kind {
            InstructionKind::MakeStruct {
                dest: d,
                fields: f,
                struct_ty: ty,
            } => {
                assert_eq!(*d, dest);
                assert_eq!(*f, fields);
                assert_eq!(*ty, struct_ty);
            }
            _ => panic!("Expected MakeStruct instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
        assert!(instr.used_values().contains(&ValueId::new(2)));
    }

    #[test]
    fn test_extract_struct_field_instruction() {
        let dest = ValueId::new(0);
        let struct_val = Value::Operand(ValueId::new(1));
        let field_name = "x".to_string();
        let field_ty = MirType::felt();

        let instr = Instruction::extract_struct_field(
            dest,
            struct_val,
            field_name.clone(),
            field_ty.clone(),
        );

        match &instr.kind {
            InstructionKind::ExtractStructField {
                dest: d,
                struct_val: s,
                field_name: f,
                field_ty: ty,
            } => {
                assert_eq!(*d, dest);
                assert_eq!(*s, struct_val);
                assert_eq!(*f, field_name);
                assert_eq!(*ty, field_ty);
            }
            _ => panic!("Expected ExtractStructField instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
    }

    #[test]
    fn test_pretty_print_aggregate_instructions() {
        use crate::PrettyPrint;

        let dest = ValueId::new(0);
        let elem1 = Value::Operand(ValueId::new(1));
        let elem2 = Value::Operand(ValueId::new(2));

        // Test MakeTuple pretty print
        let tuple_instr = Instruction::make_tuple(dest, vec![elem1, elem2]);
        let tuple_pretty = tuple_instr.pretty_print(0);
        assert!(tuple_pretty.contains("maketuple"));
        assert!(tuple_pretty.contains("%0"));
        assert!(tuple_pretty.contains("%1"));
        assert!(tuple_pretty.contains("%2"));

        // Test ExtractTupleElement pretty print
        let extract_instr = Instruction::extract_tuple_element(dest, elem1, 0, MirType::felt());
        let extract_pretty = extract_instr.pretty_print(0);
        assert!(extract_pretty.contains("extracttuple"));
        assert!(extract_pretty.contains("0"));

        // Test MakeStruct pretty print
        let fields = vec![("x".to_string(), elem1)];
        let struct_instr = Instruction::make_struct(
            dest,
            fields,
            MirType::simple_struct_type("Point".to_string()),
        );
        let struct_pretty = struct_instr.pretty_print(0);
        assert!(struct_pretty.contains("makestruct"));
        assert!(struct_pretty.contains("x:"));

        // Test ExtractStructField pretty print
        let field_instr =
            Instruction::extract_struct_field(dest, elem1, "x".to_string(), MirType::felt());
        let field_pretty = field_instr.pretty_print(0);
        assert!(field_pretty.contains("extractfield"));
        assert!(field_pretty.contains("\"x\""));
    }

    #[test]
    fn test_aggregate_instruction_validation() {
        let dest = ValueId::new(0);
        let elem1 = Value::Operand(ValueId::new(1));

        // Test that all new instructions pass validation
        let make_tuple = Instruction::make_tuple(dest, vec![elem1]);
        assert!(make_tuple.validate().is_ok());

        let extract_tuple = Instruction::extract_tuple_element(dest, elem1, 0, MirType::felt());
        assert!(extract_tuple.validate().is_ok());

        let make_struct = Instruction::make_struct(
            dest,
            vec![("field".to_string(), elem1)],
            MirType::simple_struct_type("Test".to_string()),
        );
        assert!(make_struct.validate().is_ok());

        let extract_field =
            Instruction::extract_struct_field(dest, elem1, "field".to_string(), MirType::felt());
        assert!(extract_field.validate().is_ok());
    }
}
