//! # SSA Destruction Pass
//!
//! This pass eliminates Phi nodes from the SSA form by converting them into
//! explicit move operations at the end of predecessor blocks.
//!
//! ## Algorithm
//!
//! For each Phi node `%dest = phi [%val1, block1], [%val2, block2], ...`:
//! 1. Create a temporary for the phi result if needed
//! 2. Handle critical edges by splitting them (when a predecessor has multiple successors)
//! 3. Insert assignments either at the end of predecessor blocks or in edge split blocks
//! 4. Replace the phi with an assignment from the temporary to the destination
//!
//! ## Critical Edge Handling
//!
//! A critical edge exists when a predecessor block has multiple successors and
//! the successor has multiple predecessors. These edges need to be split to
//! ensure phi copies don't interfere with each other.
//!
//! This ensures the code is in a form that can be directly lowered to assembly.

use super::MirPass;
use crate::{BasicBlock, BasicBlockId, Instruction, InstructionKind, MirFunction, Terminator};

/// Pass that eliminates Phi nodes by converting them to explicit assignments
pub struct SsaDestructionPass;

impl Default for SsaDestructionPass {
    fn default() -> Self {
        Self::new()
    }
}

impl SsaDestructionPass {
    pub const fn new() -> Self {
        Self
    }
}

impl MirPass for SsaDestructionPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        eliminate_phi_nodes(function)
    }

    fn name(&self) -> &'static str {
        "ssa_destruction"
    }
}

/// Check if an edge is critical (predecessor has multiple successors or successor has multiple predecessors)
fn is_critical_edge(function: &MirFunction, pred_id: BasicBlockId, succ_id: BasicBlockId) -> bool {
    // Check how many successors the predecessor has
    let pred_successors = get_successors(function, pred_id);
    if pred_successors.len() > 1 {
        // Check how many predecessors the successor has
        let succ_predecessors = get_predecessors(function, succ_id);
        if succ_predecessors.len() > 1 {
            return true;
        }
    }
    false
}

/// Get all successor blocks of a given block
fn get_successors(function: &MirFunction, block_id: BasicBlockId) -> Vec<BasicBlockId> {
    if let Some(block) = function.basic_blocks.get(block_id) {
        block.terminator.target_blocks()
    } else {
        vec![]
    }
}

/// Get all predecessor blocks of a given block
fn get_predecessors(function: &MirFunction, target_id: BasicBlockId) -> Vec<BasicBlockId> {
    let mut predecessors = Vec::new();
    for (block_id, block) in function.basic_blocks.iter_enumerated() {
        if block.terminator.target_blocks().contains(&target_id) {
            predecessors.push(block_id);
        }
    }
    predecessors
}

/// Split a critical edge by inserting a new block between predecessor and successor
fn split_critical_edge(
    function: &mut MirFunction,
    pred_id: BasicBlockId,
    succ_id: BasicBlockId,
) -> BasicBlockId {
    // Create a new edge block
    let edge_block = BasicBlock {
        instructions: Vec::new(),
        terminator: Terminator::Jump { target: succ_id },
    };
    let edge_block_id = function.basic_blocks.push(edge_block);

    // Update the predecessor's terminator to point to the edge block
    if let Some(pred_block) = function.basic_blocks.get_mut(pred_id) {
        pred_block.terminator.replace_target(succ_id, edge_block_id);
    }

    edge_block_id
}

/// Eliminates all Phi nodes in a function
fn eliminate_phi_nodes(function: &mut MirFunction) -> bool {
    let mut modified = false;
    let mut phi_replacements = Vec::new();
    let mut edge_splits = std::collections::HashMap::new();

    // First pass: collect all phi nodes and their information
    for (block_id, block) in function.basic_blocks.iter_enumerated() {
        for (inst_idx, instruction) in block.instructions.iter().enumerate() {
            if let InstructionKind::Phi { dest, ty, sources } = &instruction.kind {
                // The type is already in the Phi instruction
                let phi_type = ty.clone();

                // Create assignments for each predecessor
                let mut predecessor_assignments = Vec::new();
                for (pred_block_id, value) in sources {
                    predecessor_assignments.push((
                        *pred_block_id,
                        block_id, // successor block
                        *dest,
                        *value,
                        phi_type.clone(),
                    ));
                }

                phi_replacements.push((block_id, inst_idx, predecessor_assignments));
                modified = true;
            }
        }
    }

    // Second pass: insert assignments, splitting critical edges if needed
    for (phi_block_id, phi_inst_idx, assignments) in phi_replacements {
        for (pred_block_id, succ_block_id, dest, value, ty) in assignments {
            // Check if this is a critical edge
            let insert_block_id = if is_critical_edge(function, pred_block_id, succ_block_id) {
                // Check if we've already split this edge
                let edge_key = (pred_block_id, succ_block_id);
                *edge_splits
                    .entry(edge_key)
                    .or_insert_with(|| split_critical_edge(function, pred_block_id, succ_block_id))
            } else {
                pred_block_id
            };

            // Insert assignment at the end of the appropriate block (before terminator)
            if let Some(insert_block) = function.basic_blocks.get_mut(insert_block_id) {
                // Create the assignment instruction
                let assign_inst = Instruction::assign(dest, value, ty);
                insert_block.instructions.push(assign_inst);
            }
        }

        // Remove the phi instruction (mark it as a nop for now to avoid index issues)
        if let Some(block) = function.basic_blocks.get_mut(phi_block_id) {
            if phi_inst_idx < block.instructions.len() {
                // Replace with a nop (we'll clean these up in a final pass)
                block.instructions[phi_inst_idx] = Instruction::nop();
            }
        }
    }

    // Final pass: remove all nop instructions
    if modified {
        for block in function.basic_blocks.iter_mut() {
            block
                .instructions
                .retain(|inst| !matches!(inst.kind, InstructionKind::Nop));
        }
    }

    modified
}

#[cfg(test)]
#[path = "./ssa_destruction_tests.rs"]
mod tests;
