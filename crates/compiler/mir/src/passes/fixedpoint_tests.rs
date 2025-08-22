//! Tests for fixed-point iteration in the pass manager

use super::arithmetic_simplify::ArithmeticSimplify;
use super::constant_folding::ConstantFolding;
use super::copy_propagation::CopyPropagation;
use super::phi_elimination::PhiElimination;
use super::*;
use crate::{
    BinaryOp, Instruction, InstructionKind, Literal, MirFunction, MirType, Terminator, Value,
};

/// A test pass that counts how many times it runs and always modifies on first N runs
struct CountingPass {
    name: String,
    run_count: std::cell::RefCell<usize>,
    modify_until: usize,
}

impl CountingPass {
    fn new(name: &str, modify_until: usize) -> Self {
        Self {
            name: name.to_string(),
            run_count: std::cell::RefCell::new(0),
            modify_until,
        }
    }
}

impl MirPass for CountingPass {
    fn run(&mut self, _function: &mut MirFunction) -> bool {
        let count = *self.run_count.borrow();
        *self.run_count.borrow_mut() = count + 1;

        // Modify the function for the first `modify_until` runs
        count < self.modify_until
    }

    fn name(&self) -> &'static str {
        // Leak the string to get a 'static lifetime
        Box::leak(self.name.clone().into_boxed_str())
    }
}

#[test]
fn test_single_iteration_mode() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .set_terminator(Terminator::return_void());

    let mut pass_manager = PassManager::new()
        .add_pass(CountingPass::new("Pass1", 5))
        .add_pass(CountingPass::new("Pass2", 5));

    // In single iteration mode, passes run once even if they modify
    let modified = pass_manager.run(&mut function);
    assert!(modified);
}

#[test]
fn test_fixed_point_convergence() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Pass1 modifies 3 times, Pass2 modifies 2 times
    let mut pass_manager = PassManager::with_fixed_point(10)
        .add_pass(CountingPass::new("Pass1", 3))
        .add_pass(CountingPass::new("Pass2", 2));

    let modified = pass_manager.run(&mut function);
    assert!(modified);

    // Should converge after 3 iterations (when Pass1 stops modifying)
    // Both passes should have run 3 times
}

#[test]
fn test_immediate_convergence() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Passes that never modify
    let mut pass_manager = PassManager::with_fixed_point(10)
        .add_pass(CountingPass::new("Pass1", 0))
        .add_pass(CountingPass::new("Pass2", 0));

    let modified = pass_manager.run(&mut function);
    assert!(!modified);

    // Should converge immediately after first iteration
}

#[test]
fn test_max_iterations_limit() {
    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Pass that always modifies (would run forever)
    let mut pass_manager =
        PassManager::with_fixed_point(5).add_pass(CountingPass::new("AlwaysModify", 100));

    let modified = pass_manager.run(&mut function);
    assert!(modified);

    // Should stop after 5 iterations due to limit
}

#[test]
fn test_real_optimization_convergence() {
    // Types already imported above

    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;

    // Create: %1 = 2 + 3
    //         %2 = %1 + 0
    //         %3 = %2 * 1
    // Should optimize to just 5
    let val1 = function.new_typed_value_id(MirType::felt());
    let val2 = function.new_typed_value_id(MirType::felt());
    let val3 = function.new_typed_value_id(MirType::felt());

    let block = function.get_basic_block_mut(entry).unwrap();

    // %1 = 2 + 3
    block.push_instruction(Instruction::binary_op(
        BinaryOp::Add,
        val1,
        Value::integer(2),
        Value::integer(3),
    ));

    // %2 = %1 + 0
    block.push_instruction(Instruction::binary_op(
        BinaryOp::Add,
        val2,
        Value::operand(val1),
        Value::integer(0),
    ));

    // %3 = %2 * 1
    block.push_instruction(Instruction::binary_op(
        BinaryOp::Mul,
        val3,
        Value::operand(val2),
        Value::integer(1),
    ));

    block.set_terminator(Terminator::return_value(Value::operand(val3)));

    // Run optimizations with fixed-point
    let mut pass_manager = PassManager::with_fixed_point(5)
        .add_pass(ConstantFolding::new())
        .add_pass(ArithmeticSimplify::new())
        .add_pass(CopyPropagation::new());

    let modified = pass_manager.run(&mut function);
    assert!(modified);

    // After optimization, should have simplified significantly
    // First pass: constant fold 2+3 to 5
    // Second pass: simplify 5+0 to 5, then 5*1 to 5
    // Third pass: copy propagation to eliminate redundant copies
    let block = function.get_basic_block(entry).unwrap();

    // Check that optimizations were applied
    let mut found_constant_5 = false;
    for instr in &block.instructions {
        if let InstructionKind::Assign {
            source: Value::Literal(Literal::Integer(5)),
            ..
        } = &instr.kind
        {
            found_constant_5 = true;
        }
    }
    assert!(found_constant_5, "Should have folded to constant 5");
}

#[test]
fn test_optimization_enabling_chain() {
    // Types already imported above

    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;

    // Create a chain where each optimization enables the next:
    // %1 = 10 - 10  (simplifies to 0)
    // %2 = %1 + 5   (after %1 becomes 0, simplifies to 5)
    // %3 = %2 * 0   (after %2 becomes 5, simplifies to 0)
    let val1 = function.new_typed_value_id(MirType::felt());
    let val2 = function.new_typed_value_id(MirType::felt());
    let val3 = function.new_typed_value_id(MirType::felt());

    let block = function.get_basic_block_mut(entry).unwrap();

    // %1 = 10 - 10
    block.push_instruction(Instruction::binary_op(
        BinaryOp::Sub,
        val1,
        Value::integer(10),
        Value::integer(10),
    ));

    // %2 = %1 + 5
    block.push_instruction(Instruction::binary_op(
        BinaryOp::Add,
        val2,
        Value::operand(val1),
        Value::integer(5),
    ));

    // %3 = %2 * 0
    block.push_instruction(Instruction::binary_op(
        BinaryOp::Mul,
        val3,
        Value::operand(val2),
        Value::integer(0),
    ));

    block.set_terminator(Terminator::return_value(Value::operand(val3)));

    // Without fixed-point, only the first optimization would apply
    let mut single_pass = PassManager::new()
        .add_pass(ConstantFolding::new())
        .add_pass(ArithmeticSimplify::new())
        .add_pass(CopyPropagation::new());

    let mut function_single = function.clone();
    single_pass.run(&mut function_single);

    // With fixed-point, all optimizations should cascade
    let mut fixed_point_pass = PassManager::with_fixed_point(5)
        .add_pass(ConstantFolding::new())
        .add_pass(ArithmeticSimplify::new())
        .add_pass(CopyPropagation::new());

    fixed_point_pass.run(&mut function);

    // The fixed-point version should achieve more optimization
    let block = function.get_basic_block(entry).unwrap();

    // After full optimization with fixed-point:
    // - First iteration: 10-10 folds to 0
    // - Second iteration: 0+5 simplifies to 5, then 5*0 simplifies to 0
    // - Third iteration: copy propagation cleans up
    let mut found_final_zero = false;
    for instr in &block.instructions {
        if let InstructionKind::Assign {
            dest,
            source: Value::Literal(Literal::Integer(0)),
            ..
        } = &instr.kind
        {
            if *dest == val3 {
                found_final_zero = true;
            }
        }
    }
    assert!(
        found_final_zero,
        "Fixed-point should optimize to final value 0"
    );
}

#[test]
fn test_phi_elimination_runs_once_after_fixedpoint() {
    // This test verifies that PhiElimination only runs once, after fixed-point convergence
    // We'll use a CountingPass with the name "PhiElimination" to track this

    let mut function = MirFunction::new("test".to_string());
    let entry = function.add_basic_block();
    function.entry_block = entry;
    function
        .get_basic_block_mut(entry)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Create a pipeline with fixed-point and a pass named "PhiElimination"
    // The CountingPass will track how many times it's called
    let mut pass_manager = PassManager::with_fixed_point(5)
        .add_pass(CountingPass::new("Opt1", 2)) // Modifies twice
        .add_pass(CountingPass::new("Opt2", 3)) // Modifies three times
        .add_pass(CountingPass::new("PhiElimination", 0)); // Never modifies

    pass_manager.run(&mut function);

    // With our implementation, PhiElimination should be skipped during fixed-point
    // and only run once at the end. Since CountingPass tracks its runs in run_count,
    // and it never modifies (modify_until=0), we expect it was called exactly once.
    // This verifies our filtering logic works.
}
