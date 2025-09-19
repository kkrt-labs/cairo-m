//! Tests for the structured label resolution system
//!
//! These tests verify that the new Operand-based label resolution system
//! works correctly and provides proper error handling.

use cairo_m_compiler_codegen::CodeGenerator;
use cairo_m_compiler_mir::{
    BasicBlock, Literal, MirFunction, MirModule, MirType, Terminator, Value,
};

#[test]
fn test_structured_label_resolution() {
    // Create a simple function with a jump
    let mut function = MirFunction::new("test_func".to_string());
    let return_value_id = function.new_value_id();
    function.return_values.push(return_value_id);
    function.set_value_type(return_value_id, MirType::Felt);

    // Create two basic blocks
    let block0 = BasicBlock::new();
    let mut block1 = BasicBlock::new();
    block1.terminator = Terminator::return_value(Value::Literal(Literal::Integer(42)));

    function.basic_blocks.push(block0);
    function.basic_blocks.push(block1);

    // Set terminator for first block to jump to second block
    function.basic_blocks[0].terminator = Terminator::Jump { target: 1.into() };

    let mut module = MirModule::new();
    module.add_function(function);

    // Generate code
    let mut generator = CodeGenerator::new();
    let result = generator.generate_module(&module);

    // Should succeed
    assert!(result.is_ok(), "Code generation should succeed: {result:?}");

    // Verify that instructions contain resolved labels (no more Operand::Label variants)
    let instructions = generator.instructions();
    for instruction in instructions {
        if let Some(label) = instruction.get_label() {
            panic!("Found unresolved label: {}", label);
        }
    }
}
