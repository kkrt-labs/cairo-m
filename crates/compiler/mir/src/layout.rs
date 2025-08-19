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
#[derive(Debug, Clone, Default)]
pub struct DataLayout;

impl DataLayout {
    /// Create a new DataLayout instance
    pub const fn new() -> Self {
        Self
    }

    /// Get the size of a type in slots (field elements)
    ///
    /// This is the primary method for querying type sizes. It returns
    /// the number of field element slots required to store a value of
    /// the given type.
    pub fn size_of(&self, ty: &MirType) -> usize {
        match ty {
            MirType::Felt | MirType::Bool | MirType::Pointer(_) => 1,
            MirType::U32 => 2, // U32 takes 2 field elements (low, high)
            MirType::Tuple(types) => {
                // Sum of all element sizes
                types.iter().map(|t| self.size_of(t)).sum()
            }
            MirType::Struct { fields, .. } => {
                // Sum of all field sizes
                fields
                    .iter()
                    .map(|(_, field_type)| self.size_of(field_type))
                    .sum()
            }
            MirType::Array { .. } => {
                panic!("Array not implemented yet");
            }
            MirType::Function { .. } => 1, // Function pointers
            MirType::Unit => 0,
            MirType::Error | MirType::Unknown => 1, // Safe default
        }
    }

    /// Calculate the offset of a struct field by name
    ///
    /// Returns the offset in slots from the beginning of the struct
    /// to the specified field, or None if the field doesn't exist.
    pub fn field_offset(&self, ty: &MirType, field_name: &str) -> Option<usize> {
        match ty {
            MirType::Struct { fields, .. } => {
                let mut offset = 0;
                for (name, field_type) in fields {
                    if name == field_name {
                        return Some(offset);
                    }
                    offset += self.size_of(field_type);
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
    pub fn tuple_offset(&self, ty: &MirType, index: usize) -> Option<usize> {
        match ty {
            MirType::Tuple(types) => {
                if index >= types.len() {
                    return None;
                }

                // Calculate cumulative offset
                let mut offset = 0;
                for type_at_i in types.iter().take(index) {
                    offset += self.size_of(type_at_i);
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
    pub fn is_promotable(&self, ty: &MirType) -> bool {
        match ty {
            // Single-slot types are always promotable
            MirType::Felt | MirType::Bool | MirType::Pointer(_) => true,
            // U32 is promotable as a 2-slot aggregate
            MirType::U32 => true,
            // Small tuples could be promotable with proper multi-slot phi support
            MirType::Tuple(types) => {
                // For now, only allow single-element tuples until full SROA
                let size = self.size_of(ty);
                size <= 2 && types.iter().all(|t| self.is_promotable(t))
            }
            // Small structs could be promotable with proper multi-slot phi support
            MirType::Struct { fields, .. } => {
                // For now, keep conservative for complex aggregates
                let size = self.size_of(ty);
                size <= 2 && fields.iter().all(|(_, t)| self.is_promotable(t))
            }
            // Don't promote function pointers, error types, units, etc.
            _ => false,
        }
    }

    /// Get the alignment requirement for a type (in slots)
    ///
    /// Currently returns 1 for all types (no alignment padding).
    /// This method provides a centralized place for future alignment strategies.
    ///
    /// ## Future Considerations
    /// - Target-specific alignment requirements for different architectures
    /// - SIMD-friendly alignment for vector types when added
    /// - Cache-line alignment for performance-critical structures
    /// - Natural alignment for primitive types (e.g., U32 aligned to 2 slots)
    pub const fn alignment_of(&self, _ty: &MirType) -> usize {
        1 // All types are currently 1-slot aligned
    }

    /// Calculate the total size needed for a struct with alignment padding
    ///
    /// This method computes the size of a struct including any padding needed
    /// for proper alignment of the struct as a whole. Currently, no padding is
    /// added since all types have 1-slot alignment.
    ///
    /// ## Future Enhancements
    /// When alignment requirements change, this method will:
    /// - Add padding between fields to maintain field alignment
    /// - Add trailing padding to ensure the struct size is a multiple of its alignment
    /// - Support packed vs. aligned struct layouts
    ///
    /// This accounts for any padding that might be needed between fields
    /// or at the end of the struct for alignment purposes.
    pub fn struct_size_with_padding(&self, ty: &MirType) -> usize {
        // For now, no padding is needed since everything is 1-slot aligned
        self.size_of(ty)
    }

    /// Get detailed layout information for a type
    ///
    /// Returns a more detailed breakdown that could be useful for
    /// debugging or advanced optimizations.
    pub fn layout_info(&self, ty: &MirType) -> LayoutInfo {
        LayoutInfo {
            size: self.size_of(ty),
            alignment: self.alignment_of(ty),
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
    /// Alignment requirement in slots
    pub alignment: usize,
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
        let layout = DataLayout::new();

        assert_eq!(layout.size_of(&MirType::Felt), 1);
        assert_eq!(layout.size_of(&MirType::Bool), 1);
        assert_eq!(layout.size_of(&MirType::U32), 2);
        assert_eq!(layout.size_of(&MirType::Unit), 0);
        assert_eq!(layout.size_of(&MirType::pointer(MirType::Felt)), 1);
    }

    #[test]
    fn test_tuple_layout() {
        let layout = DataLayout::new();

        let tuple = MirType::tuple(vec![MirType::Felt, MirType::U32, MirType::Bool]);

        assert_eq!(layout.size_of(&tuple), 4); // 1 + 2 + 1
        assert_eq!(layout.tuple_offset(&tuple, 0), Some(0));
        assert_eq!(layout.tuple_offset(&tuple, 1), Some(1));
        assert_eq!(layout.tuple_offset(&tuple, 2), Some(3));
        assert_eq!(layout.tuple_offset(&tuple, 3), None); // Out of bounds
    }

    #[test]
    fn test_struct_layout() {
        let layout = DataLayout::new();

        let struct_type = MirType::struct_type(
            "Point".to_string(),
            vec![
                ("x".to_string(), MirType::Felt),
                ("y".to_string(), MirType::U32),
                ("z".to_string(), MirType::Bool),
            ],
        );

        assert_eq!(layout.size_of(&struct_type), 4); // 1 + 2 + 1
        assert_eq!(layout.field_offset(&struct_type, "x"), Some(0));
        assert_eq!(layout.field_offset(&struct_type, "y"), Some(1));
        assert_eq!(layout.field_offset(&struct_type, "z"), Some(3));
        assert_eq!(layout.field_offset(&struct_type, "unknown"), None);
    }

    #[test]
    fn test_nested_types() {
        let layout = DataLayout::new();

        let inner_tuple = MirType::tuple(vec![MirType::Felt, MirType::Bool]);
        let outer_struct = MirType::struct_type(
            "Container".to_string(),
            vec![
                ("data".to_string(), MirType::U32),
                ("pair".to_string(), inner_tuple),
            ],
        );

        assert_eq!(layout.size_of(&outer_struct), 4); // 2 + (1 + 1)
        assert_eq!(layout.field_offset(&outer_struct, "data"), Some(0));
        assert_eq!(layout.field_offset(&outer_struct, "pair"), Some(2));
    }

    #[test]
    fn test_promotable_types() {
        let layout = DataLayout::new();

        // Single-slot types are promotable
        assert!(layout.is_promotable(&MirType::Felt));
        assert!(layout.is_promotable(&MirType::Bool));

        // U32 is now promotable (2 slots, but handled specially)
        assert!(layout.is_promotable(&MirType::U32));

        // Small tuples (size <= 2) with promotable elements are promotable
        assert!(layout.is_promotable(&MirType::tuple(vec![MirType::Felt])));
        assert!(layout.is_promotable(&MirType::tuple(vec![MirType::Bool, MirType::Bool])));

        // Larger tuples are not promotable
        assert!(!layout.is_promotable(&MirType::tuple(vec![
            MirType::Felt,
            MirType::Felt,
            MirType::Felt
        ])));
    }
}
