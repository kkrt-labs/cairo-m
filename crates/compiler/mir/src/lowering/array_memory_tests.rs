//! Tests to ensure arrays use memory path when implemented
//!
//! These tests verify that the guards are in place to ensure arrays
//! will use memory-based operations (framealloc, get_element_ptr, load, store)
//! rather than value-based aggregate operations.

#[cfg(test)]
mod tests {
    use crate::mir_types::MirType;

    #[test]
    fn test_array_memory_path_guard() {
        // Create an array type
        let array_type = MirType::Array {
            element_type: Box::new(MirType::felt()),
            size: Some(10),
        };

        // Verify array uses memory lowering
        assert!(array_type.requires_memory_path());
        assert!(!array_type.uses_value_aggregates());
    }

    #[test]
    fn test_dynamic_array_memory_path() {
        // Dynamic arrays (size unknown at compile time)
        let dynamic_array = MirType::Array {
            element_type: Box::new(MirType::felt()),
            size: None,
        };

        // Should also use memory path
        assert!(dynamic_array.requires_memory_path());
        assert!(!dynamic_array.uses_value_aggregates());
    }

    #[test]
    fn test_nested_array_memory_path() {
        // Array of arrays (2D array)
        let inner_array = MirType::Array {
            element_type: Box::new(MirType::felt()),
            size: Some(5),
        };
        let outer_array = MirType::Array {
            element_type: Box::new(inner_array),
            size: Some(3),
        };

        // Nested arrays should use memory path
        assert!(outer_array.requires_memory_path());
        assert!(!outer_array.uses_value_aggregates());
    }

    #[test]
    fn test_array_of_structs_memory_path() {
        // Array of structs
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };
        let array_of_structs = MirType::Array {
            element_type: Box::new(struct_type),
            size: Some(10),
        };

        // Array containing structs should still use memory path
        assert!(array_of_structs.requires_memory_path());
        assert!(!array_of_structs.uses_value_aggregates());
    }

    #[test]
    fn test_tuple_value_path() {
        // Tuples should use value path, not memory
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::bool()]);

        assert!(!tuple_type.requires_memory_path());
        assert!(tuple_type.uses_value_aggregates());
    }

    #[test]
    fn test_struct_value_path() {
        // Structs should use value path, not memory
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
        let pointer = MirType::pointer(MirType::felt());

        assert!(!felt.requires_memory_path());
        assert!(!felt.uses_value_aggregates());

        assert!(!bool_type.requires_memory_path());
        assert!(!bool_type.uses_value_aggregates());

        assert!(!u32.requires_memory_path());
        assert!(!u32.uses_value_aggregates());

        assert!(!pointer.requires_memory_path());
        assert!(!pointer.uses_value_aggregates());
    }

    #[test]
    fn test_tuple_containing_array() {
        // A tuple containing an array - the tuple uses value path but the array inside would use memory
        let array_type = MirType::Array {
            element_type: Box::new(MirType::felt()),
            size: Some(5),
        };
        let tuple_with_array = MirType::Tuple(vec![MirType::felt(), array_type.clone()]);

        // The tuple itself uses value path
        assert!(!tuple_with_array.requires_memory_path());
        assert!(tuple_with_array.uses_value_aggregates());

        // But the array element would use memory path
        assert!(array_type.requires_memory_path());
        assert!(!array_type.uses_value_aggregates());
    }

    #[test]
    fn test_struct_containing_array() {
        // A struct containing an array field
        let array_type = MirType::Array {
            element_type: Box::new(MirType::felt()),
            size: Some(10),
        };
        let struct_with_array = MirType::Struct {
            name: "Container".to_string(),
            fields: vec![
                ("count".to_string(), MirType::felt()),
                ("data".to_string(), array_type.clone()),
            ],
        };

        // The struct uses value path
        assert!(!struct_with_array.requires_memory_path());
        assert!(struct_with_array.uses_value_aggregates());

        // But the array field would use memory path
        assert!(array_type.requires_memory_path());
        assert!(!array_type.uses_value_aggregates());
    }
}
