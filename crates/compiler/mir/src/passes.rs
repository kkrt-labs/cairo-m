//! # MIR Optimization Passes
//!
//! This module implements various optimization passes that can be applied to MIR functions
//! to improve code quality and remove dead code.

use crate::{InstructionKind, MirFunction};

pub mod const_eval;

/// Analyzes a MIR function to determine if it uses memory operations
/// that require SROA/Mem2Reg optimization passes.
pub fn function_uses_memory(function: &MirFunction) -> bool {
    for block in function.basic_blocks.iter() {
        for instruction in &block.instructions {
            match &instruction.kind {
                InstructionKind::FrameAlloc { .. }
                | InstructionKind::Load { .. }
                | InstructionKind::Store { .. }
                | InstructionKind::GetElementPtr { .. }
                | InstructionKind::AddressOf { .. } => {
                    return true;
                }
                _ => continue,
            }
        }
    }
    false
}

/// A trait for MIR optimization passes
pub trait MirPass {
    /// Apply this pass to a MIR function
    /// Returns true if the function was modified
    fn run(&mut self, function: &mut MirFunction) -> bool;

    /// Get the name of this pass for debugging
    fn name(&self) -> &'static str;
}

/// A wrapper for conditional pass execution
///
/// This allows passes to be skipped based on function characteristics,
/// improving compilation performance for functions that don't need certain optimizations.
pub struct ConditionalPass {
    pass: Box<dyn MirPass>,
    condition: fn(&MirFunction) -> bool,
}

impl ConditionalPass {
    /// Create a new conditional pass
    pub fn new(pass: Box<dyn MirPass>, condition: fn(&MirFunction) -> bool) -> Self {
        Self { pass, condition }
    }
}

impl MirPass for ConditionalPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        if (self.condition)(function) {
            self.pass.run(function)
        } else {
            // Skip the pass - no changes needed
            false
        }
    }

    fn name(&self) -> &'static str {
        self.pass.name()
    }
}

pub mod arithmetic_simplify;
use arithmetic_simplify::ArithmeticSimplify;

pub mod constant_folding;
use constant_folding::ConstantFolding;

pub mod copy_propagation;
use copy_propagation::CopyPropagation;

pub mod local_cse;
use local_cse::LocalCSE;

pub mod simplify_branches;
use simplify_branches::SimplifyBranches;

pub mod fuse_cmp;
use fuse_cmp::FuseCmpBranch;

pub mod dead_code_elimination;
use dead_code_elimination::DeadCodeElimination;

pub mod sroa;
use sroa::ScalarReplacementOfAggregates;

pub mod phi_elimination;
use phi_elimination::PhiElimination;

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

    /// Add a conditional pass to the manager
    /// The pass will only run if the condition function returns true
    pub fn add_conditional_pass<P: MirPass + 'static>(
        mut self,
        pass: P,
        condition: fn(&MirFunction) -> bool,
    ) -> Self {
        self.passes
            .push(Box::new(ConditionalPass::new(Box::new(pass), condition)));
        self
    }

    /// Run all passes on the function
    /// Returns true if any pass modified the function
    pub fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        for pass in &mut self.passes {
            if pass.run(function) {
                modified = true;
            }
        }

        modified
    }

    /// Create a standard optimization pipeline (default)
    ///
    /// The pipeline implements a two-phase aggregate lowering strategy:
    /// 1. SelectiveLowerAggregatesPass: Preserves optimized value-based operations
    /// 2. LowerAggregatesPass: Final complete lowering for codegen compatibility
    ///
    /// This allows optimizations like constant folding to work on value-based aggregates
    /// while still ensuring all aggregates are memory-based for CASM generation.
    pub fn standard_pipeline() -> Self {
        Self::new()
            .add_pass(ScalarReplacementOfAggregates::new()) // Run SROA early to expose scalars
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(LocalCSE::new())
            .add_pass(SimplifyBranches::new())
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(PhiElimination::new()) // Convert from SSA to non-SSA form
    }
}

#[cfg(test)]
#[path = "passes_tests.rs"]
mod tests;
