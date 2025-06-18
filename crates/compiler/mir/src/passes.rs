//! # MIR Optimization Passes
//!
//! This module implements various optimization passes that can be applied to MIR functions
//! to improve code quality and remove dead code.

use cairo_m_compiler_parser::parser::BinaryOp;

use crate::{InstructionKind, MirFunction, Terminator, Value};

/// A trait for MIR optimization passes
pub trait MirPass {
    /// Apply this pass to a MIR function
    /// Returns true if the function was modified
    fn run(&mut self, function: &mut MirFunction) -> bool;

    /// Get the name of this pass for debugging
    fn name(&self) -> &'static str;
}

/// Fuse Compare and Branch Pass
///
/// This pass identifies a `BinaryOp` performing a comparison (e.g., `Eq`)
/// whose result is only used in a subsequent `If` terminator, and fuses them
/// into a single, more efficient `BranchCmp` terminator.
///
/// ### Before:
/// ```mir
/// block_N:
///   %1 = binary_op Eq, %a, %b
///   if %1 then jump then_block else jump else_block
/// ```
///
/// ### After:
/// ```mir
/// block_N:
///   if %a Eq %b then jump then_block else jump else_block
/// ```
#[derive(Debug, Default)]
pub struct FuseCmpBranch;

impl FuseCmpBranch {
    /// Create a new pass
    pub const fn new() -> Self {
        Self
    }

    /// Returns true if an op is a comparison that can be fused.
    const fn is_fusible_comparison(op: BinaryOp) -> bool {
        // For now, only Eq is guaranteed to work. In the future we might have Le / Ge opcodes
        matches!(op, BinaryOp::Eq)
    }
}

impl MirPass for FuseCmpBranch {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;
        let use_counts = function.get_value_use_counts();

        for block in function.basic_blocks.iter_mut() {
            // We are looking for a block that ends in `If`.
            if let Terminator::If {
                condition: Value::Operand(cond_val_id),
                then_target,
                else_target,
            } = block.terminator
            {
                // The condition's result must be used exactly once (in this `If`).
                if use_counts.get(&cond_val_id).cloned() != Some(1) {
                    continue;
                }

                // The instruction defining the condition must be the last one in the block.
                if let Some(last_instr) = block.instructions.last() {
                    if last_instr.destination() == Some(cond_val_id) {
                        if let InstructionKind::BinaryOp {
                            op, left, right, ..
                        } = &last_instr.kind
                        {
                            if Self::is_fusible_comparison(*op) {
                                // We found the pattern! Perform the fusion.
                                block.terminator = Terminator::branch_cmp(
                                    *op,
                                    *left,
                                    *right,
                                    then_target,
                                    else_target,
                                );

                                // Remove the now-redundant BinaryOp instruction.
                                block.instructions.pop();

                                modified = true;
                            }
                        }
                    }
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "FuseCmpBranch"
    }
}

/// Dead Code Elimination Pass
///
/// This pass identifies and removes unreachable basic blocks from the function.
/// It uses the function's built-in reachability analysis to find dead blocks.
#[derive(Debug, Default)]
pub struct DeadCodeElimination;

impl DeadCodeElimination {
    /// Create a new dead code elimination pass
    pub const fn new() -> Self {
        Self
    }
}

impl MirPass for DeadCodeElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let unreachable = function.unreachable_blocks();

        if unreachable.is_empty() {
            return false; // No changes made
        }

        // Sort in reverse order to avoid index invalidation when removing
        let mut unreachable = unreachable;
        unreachable.sort_by_key(|a| std::cmp::Reverse(a.index()));

        // Remove unreachable blocks
        for block_id in unreachable {
            // Note: We need to be careful about removing blocks because IndexVec doesn't
            // directly support removal. For now, we'll mark them as "dead" by replacing
            // with empty blocks. A more sophisticated implementation would compact the CFG.
            if let Some(block) = function.get_basic_block_mut(block_id) {
                block.instructions.clear();
                block.set_terminator(crate::Terminator::Unreachable);
            }
        }

        true // Modified the function
    }

    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
}

/// MIR Validation Pass
///
/// This pass validates the MIR function to ensure it meets all invariants.
/// It's useful to run after other passes to ensure correctness.
#[derive(Debug, Default)]
pub struct Validation;

impl Validation {
    /// Create a new validation pass
    pub const fn new() -> Self {
        Self
    }
}

impl MirPass for Validation {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        if let Err(err) = function.validate() {
            eprintln!(
                "MIR Validation failed for function '{}': {}",
                function.name, err
            );
            // Validation passes don't modify the function
            return false;
        }

        // Check for additional invariants
        self.validate_value_usage(function);

        false // Validation doesn't modify the function
    }

    fn name(&self) -> &'static str {
        "Validation"
    }
}

impl Validation {
    /// Validate that all used values are defined before use within each block
    fn validate_value_usage(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            let mut defined_in_block = std::collections::HashSet::new();

            // Collect all values defined by instructions in this block
            for instruction in &block.instructions {
                if let Some(dest) = instruction.destination() {
                    defined_in_block.insert(dest);
                }
            }

            // Check that all used values are either defined in this block or are parameters
            let used_values = block.used_values();
            for used_value in used_values {
                if !defined_in_block.contains(&used_value)
                    && !function.parameters.contains(&used_value)
                {
                    eprintln!(
                        "Warning: Block {block_id:?} uses value {used_value:?} that is not defined in the block or as a parameter"
                    );
                }
            }
        }
    }
}

/// A pass manager that can run multiple passes in sequence
#[derive(Default)]
pub struct PassManager {
    passes: Vec<Box<dyn MirPass>>,
}

impl PassManager {
    /// Create a new pass manager
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the manager
    pub fn add_pass<P: MirPass + 'static>(mut self, pass: P) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// Run all passes on the function
    /// Returns true if any pass modified the function
    pub fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        for pass in &mut self.passes {
            if pass.run(function) {
                modified = true;
                eprintln!(
                    "Pass '{}' modified function '{}'",
                    pass.name(),
                    function.name
                );
            }
        }

        modified
    }

    /// Create a standard optimization pipeline
    pub fn standard_pipeline() -> Self {
        Self::new()
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Instruction, Terminator};

    #[test]
    fn test_dead_code_elimination() {
        let mut function = MirFunction::new("test_function".to_string());

        // Create some basic blocks - one reachable, one unreachable
        let entry_block = function.entry_block;
        let reachable_block = function.add_basic_block();
        let unreachable_block = function.add_basic_block();

        // Set up the control flow: entry -> reachable, unreachable is orphaned
        function
            .get_basic_block_mut(entry_block)
            .unwrap()
            .set_terminator(Terminator::jump(reachable_block));
        function
            .get_basic_block_mut(reachable_block)
            .unwrap()
            .set_terminator(Terminator::return_void());

        // Add an instruction to the unreachable block so we can verify it gets cleared
        function
            .get_basic_block_mut(unreachable_block)
            .unwrap()
            .push_instruction(Instruction::debug(
                "This should be removed".to_string(),
                vec![],
            ));
        function
            .get_basic_block_mut(unreachable_block)
            .unwrap()
            .set_terminator(Terminator::return_void());

        // Verify the unreachable block exists and has content before DCE
        assert_eq!(function.basic_blocks.len(), 3);
        assert!(!function
            .get_basic_block(unreachable_block)
            .unwrap()
            .instructions
            .is_empty());

        // Run dead code elimination
        let mut dce = DeadCodeElimination::new();
        let modified = dce.run(&mut function);

        // Verify the pass made changes
        assert!(modified);

        // Verify the unreachable block was cleaned (instructions cleared and marked unreachable)
        let cleaned_block = function.get_basic_block(unreachable_block).unwrap();
        assert!(cleaned_block.instructions.is_empty());
        assert!(matches!(cleaned_block.terminator, Terminator::Unreachable));
    }

    #[test]
    fn test_validation_pass() {
        let mut function = MirFunction::new("test_function".to_string());

        // Create a simple valid function
        let entry_block = function.entry_block;
        function
            .get_basic_block_mut(entry_block)
            .unwrap()
            .set_terminator(Terminator::return_void());

        // Run validation pass
        let mut validation = Validation::new();
        let modified = validation.run(&mut function);

        // Validation should not modify the function
        assert!(!modified);
    }

    #[test]
    fn test_pass_manager() {
        let mut function = MirFunction::new("test_function".to_string());

        // Set up a function with unreachable code
        let entry_block = function.entry_block;
        let unreachable_block = function.add_basic_block();

        function
            .get_basic_block_mut(entry_block)
            .unwrap()
            .set_terminator(Terminator::return_void());
        function
            .get_basic_block_mut(unreachable_block)
            .unwrap()
            .push_instruction(Instruction::debug("Unreachable".to_string(), vec![]));
        function
            .get_basic_block_mut(unreachable_block)
            .unwrap()
            .set_terminator(Terminator::return_void());

        // Run standard optimization pipeline
        let mut pass_manager = PassManager::standard_pipeline();
        let modified = pass_manager.run(&mut function);

        // Should be modified by DCE
        assert!(modified);

        // Verify unreachable block was cleaned
        let cleaned_block = function.get_basic_block(unreachable_block).unwrap();
        assert!(cleaned_block.instructions.is_empty());
    }
}
