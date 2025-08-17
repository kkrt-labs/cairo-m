//! Guards and placeholders for array memory path preservation
//!
//! This module contains guards to ensure that when arrays are implemented,
//! they will use memory-based operations rather than the value-based
//! aggregate operations used for tuples and structs.
//!
//! Arrays are intentionally kept on the memory path because:
//! 1. They often require address-of operations for element access
//! 2. They have more complex memory semantics than simple aggregates
//! 3. They may have dynamic sizing requirements
//! 4. They need to support pointer arithmetic for element access

use crate::MirType;

/// Check if a type should use memory-based lowering
///
/// Arrays always use memory operations (framealloc, load, store, get_element_ptr)
/// while tuples and structs use value-based operations (make_tuple, extract_tuple, etc.)
pub const fn should_use_memory_lowering(ty: &MirType) -> bool {
    match ty {
        MirType::Array { .. } => true,
        MirType::Tuple(_) | MirType::Struct { .. } => false,
        // Primitives and pointers don't need special handling
        _ => false,
    }
}

/// Check if a type supports value-based aggregate operations
///
/// Only tuples and structs can use the new aggregate instructions.
/// Arrays must continue using memory-based operations.
pub const fn supports_value_aggregates(ty: &MirType) -> bool {
    matches!(ty, MirType::Tuple(_) | MirType::Struct { .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_uses_memory_path() {
        let array_type = MirType::Array {
            element_type: Box::new(MirType::felt()),
            size: Some(10),
        };
        assert!(should_use_memory_lowering(&array_type));
        assert!(!supports_value_aggregates(&array_type));
    }

    #[test]
    fn test_tuple_uses_value_path() {
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        assert!(!should_use_memory_lowering(&tuple_type));
        assert!(supports_value_aggregates(&tuple_type));
    }

    #[test]
    fn test_struct_uses_value_path() {
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };
        assert!(!should_use_memory_lowering(&struct_type));
        assert!(supports_value_aggregates(&struct_type));
    }
}
