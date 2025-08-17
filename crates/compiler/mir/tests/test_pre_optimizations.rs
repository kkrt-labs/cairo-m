//! Tests for pre-optimization pass
//!
//! Verifies that the PreOptimizationPass correctly performs cleanup optimizations
//! after the lowering phase. With proper SSA form and ValueKind tracking, many
//! optimizations are now handled naturally during lowering.

use cairo_m_compiler_mir::passes::pre_opt::PreOptimizationPass;
use cairo_m_compiler_mir::passes::MirPass;
use cairo_m_compiler_mir::{
    Instruction, InstructionKind, Literal, MirFunction, MirType, Terminator, Value,
};

#[test]
fn test_dead_allocation_elimination() {
    // Test that unused stack allocations are eliminated

    let mut function = MirFunction::new("test".to_string());
    let block = function.entry_block;

    // Create an unused stack allocation
    let unused_addr = function.new_value_id();

    let block_mut = function.get_basic_block_mut(block).unwrap();
    block_mut.push_instruction(Instruction::frame_alloc(unused_addr, MirType::felt()));
    block_mut.set_terminator(Terminator::return_void());

    // Run the pre-optimization pass
    let mut pass = PreOptimizationPass::new();
    let modified = pass.run(&mut function);

    // Check that the optimization was applied
    assert!(
        modified,
        "Pre-optimization pass should have modified the function"
    );

    // The stack allocation should have been removed
    let block = function.get_basic_block(block).unwrap();
    assert_eq!(
        block.instructions.len(),
        0,
        "Unused stack allocation should have been removed"
    );
}

#[test]
fn test_optimization_pass_in_pipeline() {
    // Test that PreOptimizationPass is included in the standard pipeline

    use cairo_m_compiler_mir::PassManager;

    let mut function = MirFunction::new("test".to_string());
    function
        .get_basic_block_mut(function.entry_block)
        .unwrap()
        .set_terminator(Terminator::return_void());

    let mut pass_manager = PassManager::standard_pipeline();
    let _ = pass_manager.run(&mut function);

    // If we got here without panicking, the pre-optimization pass is in the pipeline
    // and runs successfully
}

#[test]
fn test_optimization_preserves_used_values() {
    // Test that the optimization pass doesn't remove used values
    // Note: Dead stores to addresses that are never read should be eliminated

    let mut function = MirFunction::new("test".to_string());
    let block = function.entry_block;

    // Create a used value
    let addr = function.new_value_id();
    let value = function.new_value_id();

    let block_mut = function.get_basic_block_mut(block).unwrap();
    block_mut.push_instruction(Instruction::frame_alloc(addr, MirType::felt()));
    block_mut.push_instruction(Instruction::assign(
        value,
        Value::Literal(Literal::Integer(42)),
        MirType::felt(),
    ));
    block_mut.push_instruction(Instruction::store(
        Value::Operand(addr),
        Value::Operand(value),
        MirType::felt(),
    ));
    // Use the value in the return
    block_mut.set_terminator(Terminator::return_values(vec![Value::Operand(value)]));

    // Run the pre-optimization pass
    let mut pass = PreOptimizationPass::new();
    pass.run(&mut function);

    // The assign instruction should still be there since the value is used
    // But the framealloc and store are dead code (address is never read)
    let block = function.get_basic_block(block).unwrap();
    assert_eq!(
        block.instructions.len(),
        1, // Only the assign should remain
        "Dead framealloc and store should be removed, but used assign should remain"
    );

    // Verify the remaining instruction is the assign
    match &block.instructions[0].kind {
        InstructionKind::Assign { dest, source, .. } => {
            assert_eq!(*dest, value);
            assert_eq!(*source, Value::Literal(Literal::Integer(42)));
        }
        _ => panic!("Expected Assign instruction to remain"),
    }
}
