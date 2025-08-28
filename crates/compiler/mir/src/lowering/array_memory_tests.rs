//! Tests for fixed-size arrays as value-based aggregates
//!
//! These tests verify that fixed-size arrays are treated as value-based
//! aggregates (like tuples and structs) in MIR, following the aggregate-first design.

#[cfg(test)]
mod tests {
    use crate::mir_types::MirType;

    #[test]
    fn test_fixed_array_value_path() {
        // Fixed-size arrays use value-based aggregate operations
        let array_type = MirType::FixedArray {
            element_type: Box::new(MirType::felt()),
            size: 10,
        };

        // Fixed arrays are value-based like tuples/structs
        assert!(!array_type.requires_memory_path());
        assert!(array_type.uses_value_aggregates());
    }

    #[test]
    fn test_empty_array_value_path() {
        // Zero-sized arrays are still value-based
        let empty_array = MirType::FixedArray {
            element_type: Box::new(MirType::felt()),
            size: 0,
        };

        // Even empty arrays use value path
        assert!(!empty_array.requires_memory_path());
        assert!(empty_array.uses_value_aggregates());
    }

    #[test]
    fn test_nested_array_value_path() {
        // Nested arrays (if we supported them) would still be value-based
        // Note: Currently blocked in semantic validation
        let inner_array = MirType::FixedArray {
            element_type: Box::new(MirType::felt()),
            size: 5,
        };
        let outer_array = MirType::FixedArray {
            element_type: Box::new(inner_array),
            size: 3,
        };

        // Even nested arrays would use value path
        assert!(!outer_array.requires_memory_path());
        assert!(outer_array.uses_value_aggregates());
    }

    #[test]
    fn test_array_of_structs_value_path() {
        // Array of structs
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };
        let array_of_structs = MirType::FixedArray {
            element_type: Box::new(struct_type),
            size: 10,
        };

        // Arrays of structs are value-based
        assert!(!array_of_structs.requires_memory_path());
        assert!(array_of_structs.uses_value_aggregates());
    }

    #[test]
    fn test_tuple_value_path() {
        // Tuples should use value path
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::bool()]);

        assert!(!tuple_type.requires_memory_path());
        assert!(tuple_type.uses_value_aggregates());
    }

    #[test]
    fn test_struct_value_path() {
        // Structs should use value path
        let struct_type = MirType::Struct {
            name: "Person".to_string(),
            fields: vec![
                ("name".to_string(), MirType::felt()),
                ("age".to_string(), MirType::u32()),
            ],
        };

        assert!(!struct_type.requires_memory_path());
        assert!(struct_type.uses_value_aggregates());
    }

    #[test]
    fn test_primitives_no_special_handling() {
        // Primitive types don't need special aggregate handling
        let felt = MirType::felt();
        let bool_type = MirType::bool();
        let u32 = MirType::u32();

        assert!(!felt.requires_memory_path());
        assert!(!felt.uses_value_aggregates());

        assert!(!bool_type.requires_memory_path());
        assert!(!bool_type.uses_value_aggregates());

        assert!(!u32.requires_memory_path());
        assert!(!u32.uses_value_aggregates());
    }

    #[test]
    fn test_tuple_containing_array() {
        // A tuple containing an array - both use value path
        let array_type = MirType::FixedArray {
            element_type: Box::new(MirType::felt()),
            size: 5,
        };
        let tuple_with_array = MirType::Tuple(vec![MirType::felt(), array_type.clone()]);

        // Both tuple and array use value path
        assert!(!tuple_with_array.requires_memory_path());
        assert!(tuple_with_array.uses_value_aggregates());

        assert!(!array_type.requires_memory_path());
        assert!(array_type.uses_value_aggregates());
    }

    #[test]
    fn test_struct_containing_array() {
        // A struct containing an array field - both use value path
        let array_type = MirType::FixedArray {
            element_type: Box::new(MirType::felt()),
            size: 10,
        };
        let struct_with_array = MirType::Struct {
            name: "Container".to_string(),
            fields: vec![
                ("count".to_string(), MirType::felt()),
                ("data".to_string(), array_type.clone()),
            ],
        };

        // Both struct and array use value path
        assert!(!struct_with_array.requires_memory_path());
        assert!(struct_with_array.uses_value_aggregates());

        assert!(!array_type.requires_memory_path());
        assert!(array_type.uses_value_aggregates());
    }

    #[test]
    fn test_array_size_calculation() {
        // Test that array size is correctly calculated
        let felt_array = MirType::FixedArray {
            element_type: Box::new(MirType::felt()),
            size: 5,
        };

        let u32_array = MirType::FixedArray {
            element_type: Box::new(MirType::u32()),
            size: 3,
        };

        // Using DataLayout for size calculation
        use crate::DataLayout;
        assert_eq!(DataLayout::value_size_of(&felt_array), 5); // 5 felts
        assert_eq!(DataLayout::value_size_of(&u32_array), 6); // 3 * 2 slots for u32
    }
}
