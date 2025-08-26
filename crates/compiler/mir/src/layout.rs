//! # Data Layout Module
//!
//! This module centralizes all memory layout calculations for MIR types,
//! including sizes, offsets, and alignment. This abstraction prepares the
//! compiler for more sophisticated type layouts and removes the scatter
//! of "size in slots" calculations throughout the codebase.

use crate::MirType;

/// Data layout service for MIR types
///
/// This struct encapsulates all layout calculations, making it easier to
/// change layout strategies in the future (e.g., for optimized packing,
/// alignment requirements, or target-specific layouts).
///
/// Note: All methods are static since the current implementation doesn't
/// require any instance state. This may change in the future if we need
/// to support different layout configurations (e.g., for different targets).
#[derive(Debug, Clone, Default)]
pub struct DataLayout;

impl DataLayout {
    /// Create a new DataLayout instance
    pub const fn new() -> Self {
        Self
    }

    /// Get the value size of a type in field elements
    ///
    /// This is the primary method for querying type sizes. It returns
    /// the number of field element slots required to store a value of
    /// the given type.
    pub fn value_size_of(ty: &MirType) -> usize {
        match ty {
            MirType::Felt | MirType::Bool | MirType::Pointer(_) => 1,
            MirType::U32 => 2, // U32 takes 2 field elements (low, high)
            MirType::Tuple(types) => {
                // Sum of all element sizes
                types.iter().map(Self::value_size_of).sum()
            }
            MirType::Struct { fields, .. } => {
                // Sum of all field sizes
                fields
                    .iter()
                    .map(|(_, field_type)| Self::value_size_of(field_type))
                    .sum()
            }
            MirType::FixedArray { element_type, size } => {
                // Fixed-size arrays have compile-time known size
                Self::value_size_of(element_type) * size
            }
            MirType::Function { .. } => 1, // Function pointers
            MirType::Unit => 0,
            MirType::Error | MirType::Unknown => 1, // Safe default
        }
    }

    /// Get the memory size of a type in field elements
    ///
    /// Fixed-size arrays are manipulated as pointers, so they take 1 slot of memory; although the underlying data takes more.
    /// See `value_size_of` for the value size of the data.
    pub fn memory_size_of(ty: &MirType) -> usize {
        match ty {
            MirType::Felt | MirType::Bool | MirType::Pointer(_) => 1,
            MirType::U32 => 2,
            MirType::Tuple(ts) => ts.iter().map(Self::memory_size_of).sum(),
            MirType::Struct { fields, .. } => {
                fields.iter().map(|(_, t)| Self::memory_size_of(t)).sum()
            }
            MirType::FixedArray { .. } => 1, // passed by pointer
            MirType::Unit => 0,
            MirType::Function { .. } => 1, // Function pointers
            MirType::Error | MirType::Unknown => 1, // Safe default
        }
    }

    /// Calculate the offset of a struct field by name
    ///
    /// Returns the offset in slots from the beginning of the struct
    /// to the specified field, or None if the field doesn't exist.
    pub fn field_offset(ty: &MirType, field_name: &str) -> Option<usize> {
        match ty {
            MirType::Struct { fields, .. } => {
                let mut offset = 0;
                for (name, field_type) in fields {
                    if name == field_name {
                        return Some(offset);
                    }
                    offset += Self::memory_size_of(field_type);
                }
                None
            }
            _ => None,
        }
    }

    /// Calculate the offset of a tuple element by index
    ///
    /// Returns the offset in slots from the beginning of the tuple
    /// to the element at the specified index, or None if out of bounds.
    pub fn tuple_offset(ty: &MirType, index: usize) -> Option<usize> {
        match ty {
            MirType::Tuple(types) => {
                if index >= types.len() {
                    return None;
                }

                // Calculate cumulative offset
                let mut offset = 0;
                for type_at_i in types.iter().take(index) {
                    offset += Self::memory_size_of(type_at_i);
                }
                Some(offset)
            }
            _ => None,
        }
    }

    /// Check if a type can be promoted to registers
    ///
    /// This is used by optimization passes like mem2reg to determine
    /// if a value can be kept in registers instead of memory.
    /// Single-slot types and U32 (2 slots) are promotable.
    /// Small aggregates could be promotable with proper SROA support.
    pub fn is_promotable(ty: &MirType) -> bool {
        match ty {
            // Single-slot types are always promotable
            MirType::Felt | MirType::Bool | MirType::Pointer(_) => true,
            // U32 is promotable as a 2-slot aggregate
            MirType::U32 => true,
            // Small tuples could be promotable with proper multi-slot phi support
            MirType::Tuple(types) => {
                // For now, only allow single-element tuples until full SROA
                let size = Self::memory_size_of(ty);
                size <= 2 && types.iter().all(Self::is_promotable)
            }
            // Small structs could be promotable with proper multi-slot phi support
            MirType::Struct { fields, .. } => {
                // For now, keep conservative for complex aggregates
                let size = Self::memory_size_of(ty);
                size <= 2 && fields.iter().all(|(_, t)| Self::is_promotable(t))
            }
            // Don't promote function pointers, error types, units, etc.
            _ => false,
        }
    }
    /// Get detailed layout information for a type
    ///
    /// Returns a more detailed breakdown that could be useful for
    /// debugging or advanced optimizations.
    pub fn layout_info(ty: &MirType) -> LayoutInfo {
        LayoutInfo {
            size: Self::memory_size_of(ty),
            is_aggregate: matches!(ty, MirType::Struct { .. } | MirType::Tuple(_)),
            is_scalar: matches!(
                ty,
                MirType::Felt | MirType::Bool | MirType::U32 | MirType::Pointer(_)
            ),
        }
    }
}

/// Detailed layout information for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutInfo {
    /// Size in slots
    pub size: usize,
    /// Whether this is an aggregate type (struct/tuple)
    pub is_aggregate: bool,
    /// Whether this is a scalar type
    pub is_scalar: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_sizes() {
        // No longer need DataLayout instance - using static methods

        assert_eq!(DataLayout::value_size_of(&MirType::Felt), 1);
        assert_eq!(DataLayout::value_size_of(&MirType::Bool), 1);
        assert_eq!(DataLayout::value_size_of(&MirType::U32), 2);
        assert_eq!(DataLayout::value_size_of(&MirType::Unit), 0);
        assert_eq!(
            DataLayout::value_size_of(&MirType::pointer(MirType::Felt)),
            1
        );
    }

    #[test]
    fn test_tuple_layout() {
        // No longer need DataLayout instance - using static methods

        let tuple = MirType::tuple(vec![MirType::Felt, MirType::U32, MirType::Bool]);

        assert_eq!(DataLayout::value_size_of(&tuple), 4); // 1 + 2 + 1
        assert_eq!(DataLayout::tuple_offset(&tuple, 0), Some(0));
        assert_eq!(DataLayout::tuple_offset(&tuple, 1), Some(1));
        assert_eq!(DataLayout::tuple_offset(&tuple, 2), Some(3));
        assert_eq!(DataLayout::tuple_offset(&tuple, 3), None); // Out of bounds
    }

    #[test]
    fn test_struct_layout() {
        // No longer need DataLayout instance - using static methods

        let struct_type = MirType::struct_type(
            "Point".to_string(),
            vec![
                ("x".to_string(), MirType::Felt),
                ("y".to_string(), MirType::U32),
                ("z".to_string(), MirType::Bool),
            ],
        );

        assert_eq!(DataLayout::value_size_of(&struct_type), 4); // 1 + 2 + 1
        assert_eq!(DataLayout::field_offset(&struct_type, "x"), Some(0));
        assert_eq!(DataLayout::field_offset(&struct_type, "y"), Some(1));
        assert_eq!(DataLayout::field_offset(&struct_type, "z"), Some(3));
        assert_eq!(DataLayout::field_offset(&struct_type, "unknown"), None);
    }

    #[test]
    fn test_nested_types() {
        // No longer need DataLayout instance - using static methods

        let inner_tuple = MirType::tuple(vec![MirType::Felt, MirType::Bool]);
        let outer_struct = MirType::struct_type(
            "Container".to_string(),
            vec![
                ("data".to_string(), MirType::U32),
                ("pair".to_string(), inner_tuple),
            ],
        );

        assert_eq!(DataLayout::value_size_of(&outer_struct), 4); // 2 + (1 + 1)
        assert_eq!(DataLayout::field_offset(&outer_struct, "data"), Some(0));
        assert_eq!(DataLayout::field_offset(&outer_struct, "pair"), Some(2));
    }

    #[test]
    fn test_promotable_types() {
        // No longer need DataLayout instance - using static methods

        // Single-slot types are promotable
        assert!(DataLayout::is_promotable(&MirType::Felt));
        assert!(DataLayout::is_promotable(&MirType::Bool));

        // U32 is now promotable (2 slots, but handled specially)
        assert!(DataLayout::is_promotable(&MirType::U32));

        // Small tuples (size <= 2) with promotable elements are promotable
        assert!(DataLayout::is_promotable(&MirType::tuple(vec![
            MirType::Felt
        ])));
        assert!(DataLayout::is_promotable(&MirType::tuple(vec![
            MirType::Bool,
            MirType::Bool
        ])));

        // Larger tuples are not promotable
        assert!(!DataLayout::is_promotable(&MirType::tuple(vec![
            MirType::Felt,
            MirType::Felt,
            MirType::Felt
        ])));
    }
}
