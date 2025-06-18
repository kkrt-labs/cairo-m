//! # CASM Optimizations
//!
//! This module contains optimization passes for CASM instructions.

use crate::builder::SymbolicInstruction;
use crate::{CasmBuilder, CasmInstruction, CodegenResult, Opcode};
use std::collections::{HashMap, HashSet};

/// Trait for CASM optimization passes
pub trait CasmOptimization {
    /// Name of the optimization pass
    fn name(&self) -> &'static str;

    /// Run the optimization pass on a function's instructions
    fn optimize(&self, builder: &mut CasmBuilder) -> CodegenResult<()>;
}

/// Collection of optimization passes
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn CasmOptimization>>,
}

impl OptimizationPipeline {
    /// Create a new optimization pipeline
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add an optimization pass to the pipeline
    pub fn add_pass(&mut self, pass: Box<dyn CasmOptimization>) {
        self.passes.push(pass);
    }

    /// Run all optimization passes in sequence
    pub fn run(&self, builder: &mut CasmBuilder) -> CodegenResult<()> {
        for pass in &self.passes {
            pass.optimize(builder)?;
        }
        Ok(())
    }
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Neutral operations pass
///
/// This pass eliminates neutral operations (add 0, sub 0, mul 1, div 1)
/// by replacing them with simpler instructions.
///
/// Neutral operations are operations that do not change the value of the destination.
/// For example, add 0 does not change the value of the destination, so it can be replaced with a copy.
/// Most of these neutral operations are then cleaned up by the copy propagation pass.
pub struct NeutralOperationsPass;

impl NeutralOperationsPass {
    /// Create a new copy propagation pass
    pub fn new() -> Self {
        Self
    }
}

impl CasmOptimization for NeutralOperationsPass {
    fn name(&self) -> &'static str {
        "neutral-operations"
    }

    fn optimize(&self, builder: &mut CasmBuilder) -> CodegenResult<()> {
        let symbolic_instructions = builder.symbolic_instructions();

        let optimized_instructions: Vec<_> = symbolic_instructions
            .iter()
            .filter_map(|symbolic_inst| {
                match symbolic_inst {
                    SymbolicInstruction::Label(label) => {
                        // Keep labels as-is
                        Some(SymbolicInstruction::Label(label.clone()))
                    }
                    SymbolicInstruction::Instruction(inst) => {
                        // Check for neutral operations and return appropriate transformation
                        match inst.opcode {
                            // Add 0 or Sub 0 operations
                            opcode
                                if opcode == Opcode::StoreAddFpImm as u32
                                    || opcode == Opcode::StoreSubFpImm as u32 =>
                            {
                                if inst.imm() == Some(0) {
                                    // Convert to a simple copy (StoreDerefFp)
                                    if let (Some(src_off), Some(dst_off)) = (inst.off0, inst.off2) {
                                        Some(SymbolicInstruction::Instruction(
                                            CasmInstruction::new(Opcode::StoreDerefFp as u32)
                                                .with_off0(src_off)
                                                .with_off2(dst_off)
                                                .with_comment(format!(
                                                    "[fp + {}] = [fp + {}] (optimized)",
                                                    dst_off, src_off
                                                )),
                                        ))
                                    } else {
                                        Some(SymbolicInstruction::Instruction(inst.clone()))
                                    }
                                } else {
                                    Some(SymbolicInstruction::Instruction(inst.clone()))
                                }
                            }
                            // Multiply by 1 or Divide by 1 operations
                            opcode
                                if opcode == Opcode::StoreMulFpImm as u32
                                    || opcode == Opcode::StoreDivFpImm as u32 =>
                            {
                                if inst.imm() == Some(1) {
                                    // Convert to a simple copy (StoreDerefFp)
                                    if let (Some(src_off), Some(dst_off)) = (inst.off0, inst.off2) {
                                        Some(SymbolicInstruction::Instruction(
                                            CasmInstruction::new(Opcode::StoreDerefFp as u32)
                                                .with_off0(src_off)
                                                .with_off2(dst_off)
                                                .with_comment(format!(
                                                    "[fp + {}] = [fp + {}] (optimized)",
                                                    dst_off, src_off
                                                )),
                                        ))
                                    } else {
                                        Some(SymbolicInstruction::Instruction(inst.clone()))
                                    }
                                } else {
                                    Some(SymbolicInstruction::Instruction(inst.clone()))
                                }
                            }
                            // Add 0 with two fp operands (when one operand is known to be 0)
                            opcode if opcode == Opcode::StoreAddFpFp as u32 => {
                                // We can't easily determine if an fp offset contains 0 at compile time
                                // This would require constant propagation analysis
                                // For now, skip this case
                                Some(SymbolicInstruction::Instruction(inst.clone()))
                            }
                            // Sub with same operands (x - x = 0)
                            opcode if opcode == Opcode::StoreSubFpFp as u32 => {
                                if inst.off0 == inst.off1 && inst.off0.is_some() {
                                    // x - x = 0, replace with store immediate 0
                                    if let Some(dst_off) = inst.off2 {
                                        Some(SymbolicInstruction::Instruction(
                                            CasmInstruction::new(Opcode::StoreImm as u32)
                                                .with_off2(dst_off)
                                                .with_imm(0)
                                                .with_comment(format!(
                                                    "[fp + {}] = 0 (optimized)",
                                                    dst_off
                                                )),
                                        ))
                                    } else {
                                        Some(SymbolicInstruction::Instruction(inst.clone()))
                                    }
                                } else {
                                    Some(SymbolicInstruction::Instruction(inst.clone()))
                                }
                            }
                            // Multiply by 0 (result is always 0)
                            opcode if opcode == Opcode::StoreMulFpImm as u32 => {
                                if inst.imm() == Some(0) {
                                    // x * 0 = 0, replace with store immediate 0
                                    if let Some(dst_off) = inst.off2 {
                                        Some(SymbolicInstruction::Instruction(
                                            CasmInstruction::new(Opcode::StoreImm as u32)
                                                .with_off2(dst_off)
                                                .with_imm(0)
                                                .with_comment(format!(
                                                    "[fp + {}] = 0 (optimized)",
                                                    dst_off
                                                )),
                                        ))
                                    } else {
                                        Some(SymbolicInstruction::Instruction(inst.clone()))
                                    }
                                } else {
                                    Some(SymbolicInstruction::Instruction(inst.clone()))
                                }
                            }
                            _ => Some(SymbolicInstruction::Instruction(inst.clone())),
                        }
                    }
                }
            })
            .collect();

        // Update the builder with optimized symbolic instructions
        builder.set_symbolic_instructions(optimized_instructions);

        Ok(())
    }
}

/// Copy propagation optimization pass
///
/// This pass eliminates redundant copies by:
/// 1. Building a use-def chain for fp offsets
/// 2. Identifying offsets that are used exactly once
/// 3. Merging operations with their single use
///
/// This pass is applied after the neutral operations pass to ensure that all neutral operations are eliminated
///
///
/// TODO : rearrange local variables to free some space on the stack after useless temporaries are eliminated
pub struct CopyPropagationPass;

impl CopyPropagationPass {
    pub fn new() -> Self {
        Self
    }
}

impl CasmOptimization for CopyPropagationPass {
    fn name(&self) -> &'static str {
        "copy-propagation"
    }

    fn optimize(&self, builder: &mut CasmBuilder) -> CodegenResult<()> {
        let symbolic_instructions = builder.symbolic_instructions();
        let mut write_counts = HashMap::new();
        let mut read_counts = HashMap::new();
        let mut copy_sources = HashMap::new(); // dst_off -> src_off for StoreDerefFp

        // First pass: count reads, writes, and track StoreDerefFp sources
        // Only process actual instructions, skip labels
        for symbolic_inst in symbolic_instructions {
            if let SymbolicInstruction::Instruction(inst) = symbolic_inst {
                // Use match to only count FP offset reads/writes based on opcode
                match Opcode::from_u32(inst.opcode) {
                    // Two FP operand operations - count reads from off0 and off1, write to off2
                    Some(
                        Opcode::StoreAddFpFp
                        | Opcode::StoreSubFpFp
                        | Opcode::StoreMulFpFp
                        | Opcode::StoreDivFpFp,
                    ) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                        if let Some(off1) = inst.off1 {
                            *read_counts.entry(off1).or_insert(0) += 1;
                        }
                        if let Some(dst_off) = inst.off2 {
                            *write_counts.entry(dst_off).or_insert(0) += 1;
                        }
                    }

                    // FP + Immediate operations - count read from off0, write to off2
                    Some(
                        Opcode::StoreAddFpImm
                        | Opcode::StoreSubFpImm
                        | Opcode::StoreMulFpImm
                        | Opcode::StoreDivFpImm,
                    ) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                        if let Some(dst_off) = inst.off2 {
                            *write_counts.entry(dst_off).or_insert(0) += 1;
                        }
                    }

                    // Memory operations
                    Some(Opcode::StoreDerefFp) => {
                        // [fp + off2] = [fp + off0] - read off0, write off2
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                        if let Some(dst_off) = inst.off2 {
                            *write_counts.entry(dst_off).or_insert(0) += 1;

                            // Track StoreDerefFp copy sources
                            if let Some(src_off) = inst.off0 {
                                copy_sources.insert(dst_off, src_off);
                            }
                        }
                    }

                    Some(Opcode::StoreDoubleDerefFp) => {
                        // [fp + off2] = [[fp + off0] + off1] - read off0, write off2
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                        if let Some(dst_off) = inst.off2 {
                            *write_counts.entry(dst_off).or_insert(0) += 1;
                        }
                    }

                    Some(Opcode::StoreImm) => {
                        // [fp + off2] = imm - only write to off2
                        if let Some(dst_off) = inst.off2 {
                            *write_counts.entry(dst_off).or_insert(0) += 1;
                        }
                    }

                    // Call operations that read FP offsets
                    Some(Opcode::CallAbsFp | Opcode::CallRelFp) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                    }

                    // Call operations with immediate addresses
                    // TODO : check if this doesn't break anything
                    Some(Opcode::CallAbsImm | Opcode::CallRelImm) => {
                        if let Some(off0) = inst.off0 {
                            // The return value location is typically at off0 - K
                            // For single return value functions, K=1, so return value is at off0-1
                            let return_value_offset = off0 - 1;
                            if return_value_offset >= 0 {
                                *write_counts.entry(return_value_offset).or_insert(0) += 1;
                            }
                        }
                    }

                    // Jump operations that read FP offsets
                    Some(
                        Opcode::JmpAbsAddFpFp
                        | Opcode::JmpAbsMulFpFp
                        | Opcode::JmpRelAddFpFp
                        | Opcode::JmpRelMulFpFp,
                    ) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                        if let Some(off1) = inst.off1 {
                            *read_counts.entry(off1).or_insert(0) += 1;
                        }
                    }

                    Some(
                        Opcode::JmpAbsAddFpImm
                        | Opcode::JmpAbsMulFpImm
                        | Opcode::JmpRelAddFpImm
                        | Opcode::JmpRelMulFpImm
                        | Opcode::JmpAbsDerefFp
                        | Opcode::JmpRelDerefFp,
                    ) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                    }

                    Some(Opcode::JmpAbsDoubleDerefFp | Opcode::JmpRelDoubleDerefFp) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                    }

                    // Conditional jumps
                    Some(Opcode::JnzFpFp) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                        if let Some(off1) = inst.off1 {
                            *read_counts.entry(off1).or_insert(0) += 1;
                        }
                    }

                    Some(Opcode::JnzFpImm) => {
                        if let Some(off0) = inst.off0 {
                            *read_counts.entry(off0).or_insert(0) += 1;
                        }
                    }

                    // Operations that don't use FP offsets - ignore
                    Some(Opcode::JmpAbsImm | Opcode::JmpRelImm | Opcode::Ret) | None => {
                        // These operations don't read/write FP offsets, so ignore them
                    }
                }
            }
        }

        // Second pass: find StoreDerefFp instructions where both source and destination
        // are written and read exactly once
        let mut optimizable_copies = HashMap::new(); // dst_off -> src_off for copies we can eliminate

        for (&dst_off, &src_off) in &copy_sources {
            let dst_writes = write_counts.get(&dst_off).unwrap_or(&0);
            let dst_reads = read_counts.get(&dst_off).unwrap_or(&0);
            let src_writes = write_counts.get(&src_off).unwrap_or(&0);
            let src_reads = read_counts.get(&src_off).unwrap_or(&0);

            // Check if this copy can be optimized:
            // - Destination written exactly once (by this StoreDerefFp)
            // - Destination read at most once (by some other instruction)
            // - Source written exactly once (by some other instruction) OR is a function parameter
            // - Source read at most once (by this StoreDerefFp)
            // - Only optimize non-negative offsets (for safety reasons) TODO : check if this is needed

            // TODO : find less constraining conditions
            // eg : in the fibonacci example, the parameter at [fp - 4] is read twice so the optimisation isn't applied
            // the conditions should only be this strict in the interior nodes of a chain, as the origin fp offset can be read several times with no issue

            if *dst_writes == 1
                && *dst_reads <= 1
                && *src_writes <= 1
                && *src_reads == 1
                && dst_off >= 0
            {
                optimizable_copies.insert(dst_off, src_off);
            }
        }

        // Third pass: find chains in optimizable_copies and resolve to ultimate sources
        let mut ultimate_sources = HashMap::new(); // dst_off -> ultimate_src_off

        for &dst_off in optimizable_copies.keys() {
            let mut current = dst_off;
            let mut visited = HashSet::new();

            // Follow the chain to find the ultimate source
            while let Some(&next_src) = optimizable_copies.get(&current) {
                if visited.contains(&current) {
                    // Cycle detected - break to avoid infinite loop
                    break;
                }
                visited.insert(current);
                current = next_src;
            }

            // current is now the ultimate source (not in optimizable_copies as a destination)
            ultimate_sources.insert(dst_off, current);
        }

        // Fourth pass: transform symbolic instructions using ultimate sources
        let optimized_instructions: Vec<_> = symbolic_instructions
            .iter()
            .filter_map(|symbolic_inst| {
                match symbolic_inst {
                    SymbolicInstruction::Label(label) => {
                        // Keep labels as-is
                        Some(SymbolicInstruction::Label(label.clone()))
                    }
                    SymbolicInstruction::Instruction(inst) => {
                        // Skip StoreDerefFp instructions for optimizable copies
                        if inst.opcode == Opcode::StoreDerefFp as u32 {
                            if let Some(dst_off) = inst.off2 {
                                if optimizable_copies.contains_key(&dst_off) {
                                    return None; // Eliminate this copy instruction
                                }
                            }
                        }

                        // Replace reads from optimizable copy destinations with their ultimate sources
                        let mut new_inst = inst.clone();
                        if let Some(off0) = inst.off0 {
                            if let Some(&ultimate_src) = ultimate_sources.get(&off0) {
                                new_inst.off0 = Some(ultimate_src);
                            }
                        }
                        if let Some(off1) = inst.off1 {
                            if let Some(&ultimate_src) = ultimate_sources.get(&off1) {
                                new_inst.off1 = Some(ultimate_src);
                            }
                        }

                        Some(SymbolicInstruction::Instruction(new_inst))
                    }
                }
            })
            .collect();

        // Update the builder with optimized symbolic instructions
        builder.set_symbolic_instructions(optimized_instructions);

        Ok(())
    }
}
