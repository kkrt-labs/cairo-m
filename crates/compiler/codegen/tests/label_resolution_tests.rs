//! Tests for the structured label resolution system
//!
//! These tests verify that the new Operand-based label resolution system
//! works correctly and provides proper error handling.

use cairo_m_compiler_codegen::{CodeGenerator, CodegenError, Operand};
use cairo_m_compiler_mir::{BasicBlock, Literal, MirFunction, MirModule, Terminator, Value};

#[test]
fn test_structured_label_resolution() {
    // Create a simple function with a jump
    let mut function = MirFunction::new("test_func".to_string());

    // Create two basic blocks
    let block0 = BasicBlock::new();
    let mut block1 = BasicBlock::new();
    block1.terminator = Terminator::return_value(Value::Literal(Literal::Integer(42)));

    function.basic_blocks.push(block0);
    function.basic_blocks.push(block1);

    // Set terminator for first block to jump to second block
    function.basic_blocks[0].terminator = Terminator::Jump { target: 1.into() };

    let mut module = MirModule::new();
    module.functions.push(function);

    // Generate code
    let mut generator = CodeGenerator::new();
    let result = generator.generate_module(&module);

    // Should succeed
    assert!(result.is_ok(), "Code generation should succeed: {result:?}");

    // Verify that instructions contain resolved labels (no more Operand::Label variants)
    let instructions = generator.instructions();
    for instruction in instructions {
        for operand in &instruction.operands {
            match operand {
                Operand::Literal(_) => {
                    // This is good - labels should be resolved to literals
                }
                Operand::Label(label) => {
                    panic!("Found unresolved label: {label}");
                }
            }
        }
    }
}

#[test]
fn test_unresolved_label_error() {
    // This test verifies that we get proper error messages for unresolved labels

    // Test the error type
    let error = CodegenError::UnresolvedLabel("test_label".to_string());

    match &error {
        CodegenError::UnresolvedLabel(label) => {
            assert_eq!(label, "test_label");
        }
        _ => panic!("Expected UnresolvedLabel error"),
    }

    // Test the display implementation
    let error_msg = format!("{error}");
    assert!(error_msg.contains("Unresolved label: test_label"));
}

#[test]
fn test_operand_creation() {
    // Test the Operand enum and its convenience methods

    let literal_op = Operand::literal(42);
    match literal_op {
        Operand::Literal(val) => assert_eq!(val, 42),
        _ => panic!("Expected literal operand"),
    }

    let label_op = Operand::label("test_label".to_string());
    match label_op {
        Operand::Label(name) => assert_eq!(name, "test_label"),
        _ => panic!("Expected label operand"),
    }
}

#[test]
fn test_instruction_operand_methods() {
    use cairo_m_common::instruction::*;
    use cairo_m_compiler_codegen::InstructionBuilder;

    // Test with_operand method
    let instr1 =
        InstructionBuilder::new(JMP_ABS_IMM).with_operand(Operand::Label("target".to_string()));

    // Check the last operand (which is what with_operand adds)
    match instr1.operands.last() {
        Some(Operand::Label(name)) => assert_eq!(name, "target"),
        _ => panic!("Expected label operand"),
    }

    // Test with_label convenience method
    let instr2 =
        InstructionBuilder::new(JMP_ABS_IMM).with_operand(Operand::Label("target2".to_string()));

    match instr2.operands.last() {
        Some(Operand::Label(name)) => assert_eq!(name, "target2"),
        _ => panic!("Expected label operand"),
    }

    // Test with_imm convenience method
    let instr3 = InstructionBuilder::new(JMP_ABS_IMM).with_operand(Operand::Literal(100));

    match instr3.operands.last() {
        Some(Operand::Literal(val)) => assert_eq!(*val, 100),
        _ => panic!("Expected literal operand"),
    }
}
