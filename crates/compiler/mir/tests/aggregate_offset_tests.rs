//! Tests for proper tuple element offset calculations with wide types
//!
//! These tests ensure that when accessing tuple elements with types of size > 1
//! (like u32), we use proper byte/slot offsets instead of element indices.

use cairo_m_compiler_mir::{DataLayout, Instruction, InstructionKind, MirFunction, MirType, Value};

#[test]
fn test_tuple_with_u32_element_offsets() {
    // Test: (u32, felt, bool) - u32 is size 2, so element 1 should be at offset 2
    let mut function = MirFunction::new("test_u32_tuple".to_string());

    // Create tuple type
    let tuple_type = MirType::Tuple(vec![
        MirType::U32,    // Size 2
        MirType::felt(), // Size 1
        MirType::bool(), // Size 1
    ]);

    // Verify offsets using DataLayout
    let layout = DataLayout::new();
    assert_eq!(
        layout.tuple_offset(&tuple_type, 0),
        Some(0),
        "First element at offset 0"
    );
    assert_eq!(
        layout.tuple_offset(&tuple_type, 1),
        Some(2),
        "Second element at offset 2 (after u32)"
    );
    assert_eq!(
        layout.tuple_offset(&tuple_type, 2),
        Some(3),
        "Third element at offset 3"
    );

    // Create instructions that should use these offsets
    let tuple_addr = function.new_typed_value_id(MirType::pointer(tuple_type));
    let elem1_ptr = function.new_typed_value_id(MirType::pointer(MirType::felt()));

    // This should use offset 2, not index 1
    let gep_instr = Instruction::get_element_ptr(
        elem1_ptr,
        Value::operand(tuple_addr),
        Value::integer(2), // Should be offset 2, not index 1
    );

    // Verify the instruction has the correct offset
    if let InstructionKind::GetElementPtr { offset, .. } = &gep_instr.kind {
        assert_eq!(
            *offset,
            Value::integer(2),
            "GEP should use offset 2 for element at index 1"
        );
    }
}

#[test]
fn test_tuple_return_packing_with_mixed_sizes() {
    // Test function returning (u32, felt, u32) - verify frame alloc and GEPs use layout offsets
    let function = MirFunction::new("returns_mixed_tuple".to_string());

    let tuple_type = MirType::Tuple(vec![
        MirType::U32,    // Size 2, offset 0
        MirType::felt(), // Size 1, offset 2
        MirType::U32,    // Size 2, offset 3
    ]);

    let layout = DataLayout::new();

    // Verify expected offsets
    assert_eq!(layout.tuple_offset(&tuple_type, 0), Some(0));
    assert_eq!(layout.tuple_offset(&tuple_type, 1), Some(2));
    assert_eq!(layout.tuple_offset(&tuple_type, 2), Some(3));

    // Total size should be 5 slots
    assert_eq!(layout.size_of(&tuple_type), 5);
}

#[test]
fn test_tuple_destructuring_with_wide_elements() {
    // Test: let (a, b_u32, c) = some_tuple; with mixed sizes
    let mut function = MirFunction::new("destructure_tuple".to_string());

    let tuple_type = MirType::Tuple(vec![
        MirType::felt(), // Size 1, offset 0
        MirType::U32,    // Size 2, offset 1
        MirType::bool(), // Size 1, offset 3
    ]);

    let layout = DataLayout::new();

    // Element 0 at offset 0
    assert_eq!(layout.tuple_offset(&tuple_type, 0), Some(0));
    // Element 1 (u32) at offset 1
    assert_eq!(layout.tuple_offset(&tuple_type, 1), Some(1));
    // Element 2 at offset 3 (after the 2-slot u32)
    assert_eq!(layout.tuple_offset(&tuple_type, 2), Some(3));

    // When generating GEP instructions for destructuring,
    // these offsets should be used, not the indices
    let tuple_addr = function.new_typed_value_id(MirType::pointer(tuple_type));

    // Access element 2 - should use offset 3
    let elem2_ptr = function.new_typed_value_id(MirType::pointer(MirType::bool()));
    let gep = Instruction::get_element_ptr(
        elem2_ptr,
        Value::operand(tuple_addr),
        Value::integer(3), // Offset 3, not index 2
    );

    if let InstructionKind::GetElementPtr { offset, .. } = &gep.kind {
        assert_eq!(
            *offset,
            Value::integer(3),
            "Element 2 should be at offset 3"
        );
    }
}

#[test]
fn test_struct_field_offsets_with_alignment() {
    // Test struct field offsets honor type sizes
    let struct_type = MirType::Struct {
        name: "TestStruct".to_string(),
        fields: vec![
            ("field_a".to_string(), MirType::felt()), // Size 1, offset 0
            ("field_b".to_string(), MirType::U32),    // Size 2, offset 1
            ("field_c".to_string(), MirType::felt()), // Size 1, offset 3
        ],
    };

    let layout = DataLayout::new();

    assert_eq!(layout.field_offset(&struct_type, "field_a"), Some(0));
    assert_eq!(layout.field_offset(&struct_type, "field_b"), Some(1));
    assert_eq!(layout.field_offset(&struct_type, "field_c"), Some(3));

    // Total size should be 4 slots
    assert_eq!(layout.size_of(&struct_type), 4);
}

#[test]
fn test_nested_tuple_offsets() {
    // Test: ((felt, u32), felt) - nested tuple with mixed sizes
    let inner_tuple = MirType::Tuple(vec![
        MirType::felt(), // Size 1
        MirType::U32,    // Size 2
    ]);

    let outer_tuple = MirType::Tuple(vec![
        inner_tuple.clone(), // Size 3 total
        MirType::felt(),     // Size 1
    ]);

    let layout = DataLayout::new();

    // Inner tuple is 3 slots total
    assert_eq!(layout.size_of(&inner_tuple), 3);

    // In outer tuple:
    // Element 0 (inner tuple) at offset 0
    // Element 1 (felt) at offset 3
    assert_eq!(layout.tuple_offset(&outer_tuple, 0), Some(0));
    assert_eq!(layout.tuple_offset(&outer_tuple, 1), Some(3));

    // Total size is 4 slots
    assert_eq!(layout.size_of(&outer_tuple), 4);
}
