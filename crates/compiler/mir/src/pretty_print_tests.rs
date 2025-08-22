//! Tests for pretty printing of MIR instructions, especially aggregate operations

#[cfg(test)]
mod tests {
    use crate::instruction::Instruction;
    use crate::mir_types::MirType;
    use crate::value::Value;
    use crate::MirFunction;
    use crate::PrettyPrint; // PrettyPrint is in lib.rs

    fn make_test_function() -> MirFunction {
        MirFunction::new("test".to_string())
    }

    #[test]
    fn test_make_tuple_pretty_print() {
        let mut func = make_test_function();
        let elem1 = func.new_value_id(); // %0
        let elem2 = func.new_value_id(); // %1
        let dest = func.new_value_id(); // %2

        // Regular tuple
        let instr =
            Instruction::make_tuple(dest, vec![Value::operand(elem1), Value::operand(elem2)]);
        assert_eq!(instr.pretty_print(0), "%2 = maketuple %0, %1");

        // Empty tuple
        let empty_dest = func.new_value_id(); // %3
        let empty_instr = Instruction::make_tuple(empty_dest, vec![]);
        assert_eq!(empty_instr.pretty_print(0), "%3 = maketuple");

        // Single element tuple
        let single_dest = func.new_value_id(); // %4
        let single_instr = Instruction::make_tuple(single_dest, vec![Value::operand(elem1)]);
        assert_eq!(single_instr.pretty_print(0), "%4 = maketuple %0");

        // Tuple with literals
        let lit_dest = func.new_value_id(); // %5
        let lit_instr =
            Instruction::make_tuple(lit_dest, vec![Value::integer(42), Value::operand(elem1)]);
        assert_eq!(lit_instr.pretty_print(0), "%5 = maketuple 42, %0");
    }

    #[test]
    fn test_extract_tuple_pretty_print() {
        let mut func = make_test_function();
        let tuple_val = func.new_value_id(); // %0
        let dest = func.new_value_id(); // %1

        let instr =
            Instruction::extract_tuple_element(dest, Value::operand(tuple_val), 1, MirType::felt());
        assert_eq!(instr.pretty_print(0), "%1 = extracttuple %0, 1");

        // Extract from literal (unusual but valid)
        let dest2 = func.new_value_id(); // %2
        let instr2 = Instruction::extract_tuple_element(
            dest2,
            Value::unit(), // Unit tuple
            0,
            MirType::felt(),
        );
        assert_eq!(instr2.pretty_print(0), "%2 = extracttuple (), 0");
    }

    #[test]
    fn test_make_struct_pretty_print() {
        let mut func = make_test_function();
        let x_val = func.new_value_id(); // %0
        let y_val = func.new_value_id(); // %1
        let dest = func.new_value_id(); // %2

        // Regular struct
        let instr = Instruction::make_struct(
            dest,
            vec![
                ("x".to_string(), Value::operand(x_val)),
                ("y".to_string(), Value::operand(y_val)),
            ],
            MirType::Struct {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), MirType::felt()),
                    ("y".to_string(), MirType::felt()),
                ],
            },
        );
        assert_eq!(instr.pretty_print(0), "%2 = makestruct { x: %0, y: %1 }");

        // Empty struct
        let empty_dest = func.new_value_id(); // %3
        let empty_instr = Instruction::make_struct(
            empty_dest,
            vec![],
            MirType::Struct {
                name: "Empty".to_string(),
                fields: vec![],
            },
        );
        assert_eq!(empty_instr.pretty_print(0), "%3 = makestruct {  }");

        // Single field struct
        let single_dest = func.new_value_id(); // %4
        let single_instr = Instruction::make_struct(
            single_dest,
            vec![("value".to_string(), Value::integer(42))],
            MirType::Struct {
                name: "Wrapper".to_string(),
                fields: vec![("value".to_string(), MirType::felt())],
            },
        );
        assert_eq!(
            single_instr.pretty_print(0),
            "%4 = makestruct { value: 42 }"
        );
    }

    #[test]
    fn test_extract_struct_field_pretty_print() {
        let mut func = make_test_function();
        let struct_val = func.new_value_id(); // %0
        let dest = func.new_value_id(); // %1

        let instr = Instruction::extract_struct_field(
            dest,
            Value::operand(struct_val),
            "field_name".to_string(),
            MirType::felt(),
        );
        assert_eq!(
            instr.pretty_print(0),
            "%1 = extractfield %0, \"field_name\""
        );

        // Field with special characters
        let dest2 = func.new_value_id(); // %2
        let instr2 = Instruction::extract_struct_field(
            dest2,
            Value::operand(struct_val),
            "field-with-dash".to_string(),
            MirType::felt(),
        );
        assert_eq!(
            instr2.pretty_print(0),
            "%2 = extractfield %0, \"field-with-dash\""
        );
    }

    #[test]
    fn test_insert_field_pretty_print() {
        let mut func = make_test_function();
        let struct_val = func.new_value_id(); // %0
        let new_val = func.new_value_id(); // %1
        let dest = func.new_value_id(); // %2

        let instr = Instruction::insert_field(
            dest,
            Value::operand(struct_val),
            "x".to_string(),
            Value::operand(new_val),
            MirType::Struct {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), MirType::felt()),
                    ("y".to_string(), MirType::felt()),
                ],
            },
        );
        assert_eq!(instr.pretty_print(0), "%2 = insertfield %0, \"x\", %1");

        // Insert with literal value
        let dest2 = func.new_value_id(); // %3
        let instr2 = Instruction::insert_field(
            dest2,
            Value::operand(struct_val),
            "count".to_string(),
            Value::integer(100),
            MirType::Struct {
                name: "Counter".to_string(),
                fields: vec![("count".to_string(), MirType::felt())],
            },
        );
        assert_eq!(
            instr2.pretty_print(0),
            "%3 = insertfield %0, \"count\", 100"
        );
    }

    #[test]
    fn test_insert_tuple_pretty_print() {
        let mut func = make_test_function();
        let tuple_val = func.new_value_id(); // %0
        let new_val = func.new_value_id(); // %1
        let dest = func.new_value_id(); // %2

        let instr = Instruction::insert_tuple(
            dest,
            Value::operand(tuple_val),
            1,
            Value::operand(new_val),
            MirType::Tuple(vec![MirType::felt(), MirType::felt()]),
        );
        assert_eq!(instr.pretty_print(0), "%2 = inserttuple %0, 1, %1");

        // Insert with literal value
        let dest2 = func.new_value_id(); // %3
        let instr2 = Instruction::insert_tuple(
            dest2,
            Value::operand(tuple_val),
            0,
            Value::boolean(true),
            MirType::Tuple(vec![MirType::bool(), MirType::felt()]),
        );
        assert_eq!(instr2.pretty_print(0), "%3 = inserttuple %0, 0, true");
    }

    #[test]
    fn test_complex_nested_pretty_print() {
        let mut func = make_test_function();

        // Create a struct containing a tuple
        let elem1 = func.new_value_id(); // %0
        let elem2 = func.new_value_id(); // %1
        let tuple_dest = func.new_value_id(); // %2
        let tuple_instr = Instruction::make_tuple(
            tuple_dest,
            vec![Value::operand(elem1), Value::operand(elem2)],
        );

        let struct_dest = func.new_value_id(); // %3
        let struct_instr = Instruction::make_struct(
            struct_dest,
            vec![
                ("position".to_string(), Value::operand(tuple_dest)),
                ("active".to_string(), Value::boolean(true)),
            ],
            MirType::Struct {
                name: "Entity".to_string(),
                fields: vec![
                    (
                        "position".to_string(),
                        MirType::Tuple(vec![MirType::felt(), MirType::felt()]),
                    ),
                    ("active".to_string(), MirType::bool()),
                ],
            },
        );

        assert_eq!(tuple_instr.pretty_print(0), "%2 = maketuple %0, %1");
        assert_eq!(
            struct_instr.pretty_print(0),
            "%3 = makestruct { position: %2, active: true }"
        );

        // Extract nested value
        let extract_dest = func.new_value_id(); // %4
        let extract_instr = Instruction::extract_struct_field(
            extract_dest,
            Value::operand(struct_dest),
            "position".to_string(),
            MirType::Tuple(vec![MirType::felt(), MirType::felt()]),
        );
        assert_eq!(
            extract_instr.pretty_print(0),
            "%4 = extractfield %3, \"position\""
        );
    }

    #[test]
    fn test_instruction_with_comment() {
        let mut func = make_test_function();
        let elem = func.new_value_id(); // %0
        let dest = func.new_value_id(); // %1

        let instr = Instruction::make_tuple(dest, vec![Value::operand(elem)])
            .with_comment("Create singleton tuple".to_string());

        let output = instr.pretty_print(0);
        assert!(output.contains("// Create singleton tuple"));
        assert!(output.contains("%1 = maketuple %0"));
    }
}
