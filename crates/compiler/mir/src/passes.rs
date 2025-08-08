//! # MIR Optimization Passes
//!
//! This module implements various optimization passes that can be applied to MIR functions
//! to improve code quality and remove dead code.

pub mod mem2reg_ssa;
pub mod pre_opt;
pub mod sroa;
pub mod ssa_destruction;

pub use sroa::SroaPass;

use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BinaryOp, InstructionKind, Literal, MirFunction, MirType, Terminator, Value};

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
        matches!(
            op,
            BinaryOp::Eq | BinaryOp::Neq | BinaryOp::U32Eq | BinaryOp::U32Neq
        )
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
                                    (
                                        BinaryOp::Eq | BinaryOp::U32Eq,
                                        Value::Literal(Literal::Integer(0)),
                                        cond,
                                    )
                                    | (
                                        BinaryOp::Eq | BinaryOp::U32Eq,
                                        cond,
                                        Value::Literal(Literal::Integer(0)),
                                    ) => {
                                        // Checking x == 0 is equivalent to !x, so we switch the targets
                                        block.terminator =
                                            Terminator::branch(cond, else_target, then_target);
                                    }
                                    (
                                        BinaryOp::Neq | BinaryOp::U32Neq,
                                        Value::Literal(Literal::Integer(0)),
                                        cond,
                                    )
                                    | (
                                        BinaryOp::Neq | BinaryOp::U32Neq,
                                        cond,
                                        Value::Literal(Literal::Integer(0)),
                                    ) => {
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
                        } else if let InstructionKind::UnaryOp { op, source, .. } = &last_instr.kind
                        {
                            if matches!(op, UnaryOp::Not) {
                                // If the condition is a not, we switch the targets
                                // For simplicity, we assume dumb conditions such as !42 will never appear in the source code
                                block.terminator =
                                    Terminator::branch(*source, else_target, then_target);

                                // Remove the now-redundant UnaryOp instruction.
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
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!(
                    "[ERROR] MIR Validation failed for function '{}': {}",
                    function.name, err
                );
            }
            // Validation passes don't modify the function
            return false;
        }

        // Check for additional invariants
        self.validate_value_usage(function);
        self.validate_pointer_types(function);
        self.validate_store_types(function);
        self.validate_gep_usage(function);
        self.validate_cfg_structure(function);
        self.validate_single_definition(function);

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

    /// Validate that Load instructions only use pointer-typed addresses
    fn validate_pointer_types(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let InstructionKind::Load {
                    address: Value::Operand(addr_id),
                    ..
                } = &instruction.kind
                {
                    // Check that the address operand is a pointer
                    if let Some(addr_type) = function.get_value_type(*addr_id) {
                        if !matches!(addr_type, MirType::Pointer(_)) {
                            if std::env::var("RUST_LOG").is_ok() {
                                eprintln!(
                                    "[ERROR] Block {block_id:?}, instruction {instr_idx}: Load instruction uses non-pointer address {addr_id:?} with type {addr_type:?}"
                                );
                            }
                        }
                    } else {
                        if std::env::var("RUST_LOG").is_ok() {
                            eprintln!(
                                "[WARN] Block {block_id:?}, instruction {instr_idx}: Load instruction uses address {addr_id:?} with unknown type"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate that Store instructions only use pointer-typed addresses
    fn validate_store_types(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let InstructionKind::Store { address, .. } = &instruction.kind {
                    if let Value::Operand(addr_id) = address {
                        // Check that the address operand is a pointer
                        if let Some(addr_type) = function.get_value_type(*addr_id) {
                            if !matches!(addr_type, MirType::Pointer(_))
                                && std::env::var("RUST_LOG").is_ok()
                            {
                                eprintln!(
                                    "[ERROR] Block {block_id:?}, instruction {instr_idx}: Store instruction uses non-pointer address {addr_id:?} with type {addr_type:?}"
                                );
                            }
                        } else if std::env::var("RUST_LOG").is_ok() {
                            eprintln!(
                                "[WARN] Block {block_id:?}, instruction {instr_idx}: Store instruction uses address {addr_id:?} with unknown type"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate GEP usage (warn about raw offset GEPs)
    fn validate_gep_usage(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let InstructionKind::GetElementPtr { offset, .. } = &instruction.kind {
                    // Warn if using raw integer offsets (not typed indexing)
                    // This is a temporary warning until typed GEP is fully implemented
                    if let Value::Literal(Literal::Integer(offset_val)) = offset {
                        if *offset_val != 0 && std::env::var("RUST_LOG").is_ok() {
                            eprintln!(
                                "[WARN] Block {block_id:?}, instruction {instr_idx}: GEP uses raw offset {offset_val}. Consider using typed GEP once available."
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate CFG structure (check for critical edges, unreachable blocks, etc.)
    fn validate_cfg_structure(&self, function: &MirFunction) {
        use crate::cfg::{get_predecessors, is_critical_edge};

        // Check for unreachable blocks
        let unreachable = function.unreachable_blocks();
        if !unreachable.is_empty() && std::env::var("RUST_LOG").is_ok() {
            eprintln!(
                "[WARN] Function '{}' contains {} unreachable blocks: {:?}",
                function.name,
                unreachable.len(),
                unreachable
            );
        }

        // Warn about critical edges (these should be split for correct SSA destruction)
        for (pred_id, pred_block) in function.basic_blocks.iter_enumerated() {
            for succ_id in pred_block.terminator.target_blocks() {
                if is_critical_edge(function, pred_id, succ_id)
                    && std::env::var("RUST_LOG").is_ok()
                    && std::env::var("RUST_LOG").unwrap().contains("debug")
                {
                    eprintln!(
                        "[DEBUG] Critical edge detected: {pred_id:?} -> {succ_id:?} in function '{}'",
                        function.name
                    );
                }
            }
        }

        // Check that entry block has no predecessors
        let entry_preds = get_predecessors(function, function.entry_block);
        if !entry_preds.is_empty() && std::env::var("RUST_LOG").is_ok() {
            eprintln!(
                "[ERROR] Entry block {:?} has predecessors: {:?} in function '{}'",
                function.entry_block, entry_preds, function.name
            );
        }
    }

    /// Validate that each value is defined exactly once
    fn validate_single_definition(&self, function: &MirFunction) {
        let mut defined_values = std::collections::HashSet::new();

        // Check parameters
        for &param_id in &function.parameters {
            if !defined_values.insert(param_id) && std::env::var("RUST_LOG").is_ok() {
                eprintln!(
                    "[ERROR] Value {param_id:?} is defined multiple times as a parameter in function '{}'",
                    function.name
                );
            }
        }

        // Check instructions
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let Some(dest) = instruction.destination() {
                    if !defined_values.insert(dest) && std::env::var("RUST_LOG").is_ok() {
                        eprintln!(
                            "[ERROR] Value {dest:?} is defined multiple times (block {block_id:?}, instruction {instr_idx}) in function '{}'",
                            function.name
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "./passes/validation_tests.rs"]
mod validation_tests;

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
            .add_pass(pre_opt::PreOptimizationPass::new())
            .add_pass(sroa::SroaPass::new()) // Split aggregates before mem2reg
            .add_pass(mem2reg_ssa::Mem2RegSsaPass::new()) // Run SSA mem2reg early for true SSA form
            .add_pass(ssa_destruction::SsaDestructionPass::new()) // Eliminate Phi nodes before codegen
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new())
    }
}

#[cfg(test)]
#[path = "passes_tests.rs"]
mod tests;
