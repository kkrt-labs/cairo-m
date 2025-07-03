//! # MIR Optimization Passes
//!
//! This module implements various optimization passes that can be applied to MIR functions
//! to improve code quality and remove dead code.

use cairo_m_compiler_parser::parser::BinaryOp;
use rustc_hash::FxHashMap;

use crate::{Instruction, InstructionKind, Literal, MirFunction, Terminator, Value, ValueId};

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
        matches!(op, BinaryOp::Eq | BinaryOp::Neq)
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

                                // We first check for comparisons with 0 which can be optimized
                                match (*op, *left, *right) {
                                    (BinaryOp::Eq, Value::Literal(Literal::Integer(0)), cond)
                                    | (BinaryOp::Eq, cond, Value::Literal(Literal::Integer(0))) => {
                                        // Checking x == 0 is equivalent to !x, so we switch the targets
                                        block.terminator =
                                            Terminator::branch(cond, else_target, then_target);
                                    }
                                    (BinaryOp::Neq, Value::Literal(Literal::Integer(0)), cond)
                                    | (BinaryOp::Neq, cond, Value::Literal(Literal::Integer(0))) => {
                                        // Checking x != 0 is equivalent to x, so we use x as the condition
                                        block.terminator =
                                            Terminator::branch(cond, then_target, else_target);
                                    }
                                    _ => {
                                        // For all other cases, we can fuse the comparison and branch
                                        block.terminator = Terminator::branch_cmp(
                                            *op,
                                            *left,
                                            *right,
                                            then_target,
                                            else_target,
                                        );
                                    }
                                }

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

/// In-Place Optimization Pass
///
/// This pass identifies patterns where a value is loaded from memory, modified with
/// a binary operation, and stored back to the same location. It optimizes these
/// patterns to perform the operation directly on the memory location.
///
/// ### Before:
/// ```mir
/// %val = load %addr
/// %tmp = binary_op Add, %val, %other
/// store %addr, %tmp
/// ```
///
/// ### After:
/// ```mir
/// %tmp = binary_op Add, %val, %other [in-place to %addr]
/// // Load and Store instructions are removed
/// ```
#[derive(Debug, Default)]
pub struct InPlaceOptimizationPass;

impl InPlaceOptimizationPass {
    /// Create a new in-place optimization pass
    pub const fn new() -> Self {
        Self
    }
}

impl MirPass for InPlaceOptimizationPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;
        let use_counts = function.get_value_use_counts();

        for block in function.basic_blocks.iter_mut() {
            let mut i = 0;

            while i < block.instructions.len() {
                // Try to optimize the Load-BinaryOp-Store pattern
                if let Some(skip_count) = self.try_optimize_load_binop_store_pattern(
                    &mut block.instructions,
                    i,
                    &use_counts,
                ) {
                    modified = true;
                    i += skip_count;
                    continue;
                }

                // Try to optimize the simpler BinaryOp-Store pattern
                if let Some(skip_count) =
                    self.try_optimize_binop_store_pattern(&mut block.instructions, i, &use_counts)
                {
                    modified = true;
                    i += skip_count;
                    continue;
                }

                i += 1;
            }
        }

        // Clean up all marked instructions after processing all blocks
        if modified {
            self.cleanup_removed_instructions(function);
        }

        modified
    }

    fn name(&self) -> &'static str {
        "InPlaceOptimizationPass"
    }
}

impl InPlaceOptimizationPass {
    /// Try to optimize the Load-BinaryOp-Store pattern
    /// Returns Some(skip_count) if optimization was applied, None otherwise
    fn try_optimize_load_binop_store_pattern(
        &self,
        instructions: &mut [Instruction],
        i: usize,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> Option<usize> {
        // Need at least 3 instructions for this pattern
        if i + 2 >= instructions.len() {
            return None;
        }

        let (load_instr, binop_instr, store_instr) =
            (&instructions[i], &instructions[i + 1], &instructions[i + 2]);

        let (load_data, binop_data, store_data) =
            self.extract_load_binop_store_data(load_instr, binop_instr, store_instr)?;

        if self.is_valid_load_binop_store_pattern(&load_data, &binop_data, &store_data, use_counts)
        {
            self.apply_load_binop_store_optimization(
                instructions,
                i,
                &binop_data,
                load_data.addr_id,
            );
            return Some(3);
        }

        None
    }

    /// Try to optimize the BinaryOp-Store pattern
    /// Returns Some(skip_count) if optimization was applied, None otherwise
    fn try_optimize_binop_store_pattern(
        &self,
        instructions: &mut [Instruction],
        i: usize,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> Option<usize> {
        // Need at least 2 instructions for this pattern
        if i + 1 >= instructions.len() {
            return None;
        }

        let (binop_instr, store_instr) = (&instructions[i], &instructions[i + 1]);

        let (binop_data, store_data) = self.extract_binop_store_data(binop_instr, store_instr)?;

        if self.is_valid_binop_store_pattern(&binop_data, &store_data, use_counts) {
            self.apply_binop_store_optimization(instructions, i, &binop_data, store_data.addr_id?);
            return Some(2);
        }

        None
    }

    /// Extract data from Load-BinaryOp-Store instructions
    fn extract_load_binop_store_data(
        &self,
        load_instr: &Instruction,
        binop_instr: &Instruction,
        store_instr: &Instruction,
    ) -> Option<(LoadData, BinopData, StoreData)> {
        let load_data = LoadData::from_instruction(load_instr)?;
        let binop_data = BinopData::from_instruction(binop_instr)?;
        let store_data = StoreData::from_instruction(store_instr)?;

        Some((load_data, binop_data, store_data))
    }

    /// Extract data from BinaryOp-Store instructions
    fn extract_binop_store_data(
        &self,
        binop_instr: &Instruction,
        store_instr: &Instruction,
    ) -> Option<(BinopData, StoreData)> {
        let binop_data = BinopData::from_instruction(binop_instr)?;
        let store_data = StoreData::from_instruction(store_instr)?;

        Some((binop_data, store_data))
    }

    /// Check if the Load-BinaryOp-Store pattern is valid for optimization
    fn is_valid_load_binop_store_pattern(
        &self,
        load_data: &LoadData,
        binop_data: &BinopData,
        store_data: &StoreData,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> bool {
        let left_is_val = matches!(binop_data.left, Value::Operand(id) if id == load_data.val_id);
        let right_is_val = matches!(binop_data.right, Value::Operand(id) if id == load_data.val_id);
        let store_is_tmp =
            matches!(store_data.value, Value::Operand(id) if id == binop_data.tmp_id);

        load_data.address == store_data.address
            && (left_is_val || right_is_val)
            && store_is_tmp
            && use_counts.get(&load_data.val_id).cloned() == Some(1)
            && use_counts.get(&binop_data.tmp_id).cloned() == Some(1)
    }

    /// Check if the BinaryOp-Store pattern is valid for optimization
    fn is_valid_binop_store_pattern(
        &self,
        binop_data: &BinopData,
        store_data: &StoreData,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> bool {
        let store_is_tmp =
            matches!(store_data.value, Value::Operand(id) if id == binop_data.tmp_id);

        if !store_is_tmp || use_counts.get(&binop_data.tmp_id).cloned() != Some(1) {
            return false;
        }

        let left_is_store =
            matches!(binop_data.left, Value::Operand(id) if Some(id) == store_data.addr_id);
        let right_is_store =
            matches!(binop_data.right, Value::Operand(id) if Some(id) == store_data.addr_id);

        left_is_store || right_is_store
    }

    /// Apply the Load-BinaryOp-Store optimization
    fn apply_load_binop_store_optimization(
        &self,
        instructions: &mut [Instruction],
        i: usize,
        binop_data: &BinopData,
        addr_id: crate::ValueId,
    ) {
        let new_binop = Instruction {
            kind: InstructionKind::BinaryOp {
                op: binop_data.op,
                dest: binop_data.tmp_id,
                left: binop_data.left,
                right: binop_data.right,
                in_place_target: Some(addr_id),
            },
            ..instructions[i + 1].clone()
        };

        instructions[i + 1] = new_binop;
        instructions[i] = Instruction::debug("removed: load".to_string(), vec![]);
        instructions[i + 2] = Instruction::debug("removed: store".to_string(), vec![]);
    }

    /// Apply the BinaryOp-Store optimization
    fn apply_binop_store_optimization(
        &self,
        instructions: &mut [Instruction],
        i: usize,
        binop_data: &BinopData,
        addr_id: ValueId,
    ) {
        let new_binop = Instruction {
            kind: InstructionKind::BinaryOp {
                op: binop_data.op,
                dest: binop_data.tmp_id,
                left: binop_data.left,
                right: binop_data.right,
                in_place_target: Some(addr_id),
            },
            ..instructions[i].clone()
        };

        instructions[i] = new_binop;
        instructions[i + 1] = Instruction::debug("removed: store".to_string(), vec![]);
    }

    /// Clean up instructions marked for removal
    fn cleanup_removed_instructions(&self, function: &mut MirFunction) {
        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                !matches!(&instr.kind, InstructionKind::Debug { message, .. } if message.starts_with("removed:"))
            });
        }
    }
}

/// Helper struct to hold Load instruction data
struct LoadData {
    val_id: crate::ValueId,
    address: Value,
    addr_id: crate::ValueId,
}

impl LoadData {
    fn from_instruction(instr: &Instruction) -> Option<Self> {
        if let InstructionKind::Load {
            dest: val_id,
            address: load_addr,
        } = &instr.kind
        {
            let addr_id = load_addr.as_operand()?;
            Some(Self {
                val_id: *val_id,
                address: *load_addr,
                addr_id,
            })
        } else {
            None
        }
    }
}

/// Helper struct to hold BinaryOp instruction data
struct BinopData {
    op: cairo_m_compiler_parser::parser::BinaryOp,
    tmp_id: crate::ValueId,
    left: Value,
    right: Value,
}

impl BinopData {
    const fn from_instruction(instr: &Instruction) -> Option<Self> {
        if let InstructionKind::BinaryOp {
            op,
            dest: tmp_id,
            left,
            right,
            in_place_target: None,
        } = &instr.kind
        {
            Some(Self {
                op: *op,
                tmp_id: *tmp_id,
                left: *left,
                right: *right,
            })
        } else {
            None
        }
    }
}

/// Helper struct to hold Store instruction data
struct StoreData {
    address: Value,
    value: Value,
    addr_id: Option<crate::ValueId>,
}

impl StoreData {
    const fn from_instruction(instr: &Instruction) -> Option<Self> {
        if let InstructionKind::Store {
            address: store_addr,
            value: store_val,
        } = &instr.kind
        {
            Some(Self {
                address: *store_addr,
                value: *store_val,
                addr_id: store_addr.as_operand(),
            })
        } else {
            None
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
            .add_pass(InPlaceOptimizationPass::new())
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
        assert!(
            !function
                .get_basic_block(unreachable_block)
                .unwrap()
                .instructions
                .is_empty()
        );

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
