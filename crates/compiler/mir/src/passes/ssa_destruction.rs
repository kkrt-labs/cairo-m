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
//! ## Parallel Copy Semantics
//!
//! When multiple phi nodes share overlapping sources and destinations, we must
//! ensure assignments execute with parallel copy semantics. This means all sources
//! are read before any destinations are written. The implementation uses a
//! dependency graph to detect cycles and introduces temporaries to break them.
//!
//! This ensures the code is in a form that can be directly lowered to assembly.

use super::MirPass;
use crate::{BasicBlockId, Instruction, InstructionKind, MirFunction, MirType, Value, ValueId};
use rustc_hash::FxHashMap;

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

/// Represents a copy operation in the parallel copy graph
#[derive(Debug, Clone)]
struct CopyOperation {
    dest: ValueId,
    source: Value,
    ty: MirType,
}

/// Manages parallel copy operations for phi elimination
struct ParallelCopyGraph {
    copies: Vec<CopyOperation>,
    dependencies: FxHashMap<ValueId, Vec<usize>>, // Maps ValueId to indices of copies that depend on it
}

impl ParallelCopyGraph {
    fn new() -> Self {
        Self {
            copies: Vec::new(),
            dependencies: FxHashMap::default(),
        }
    }

    fn add_copy(&mut self, dest: ValueId, source: Value, ty: MirType) {
        let copy_idx = self.copies.len();
        self.copies.push(CopyOperation { dest, source, ty });

        // Track dependencies: if source is an operand, this copy depends on that value
        if let Value::Operand(src_id) = source {
            self.dependencies.entry(src_id).or_default().push(copy_idx);
        }
    }

    /// Detect cycles in the dependency graph using DFS
    fn find_cycles(&self) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        let mut visited = vec![false; self.copies.len()];
        let mut rec_stack = vec![false; self.copies.len()];
        let mut path = Vec::new();

        for i in 0..self.copies.len() {
            if !visited[i] {
                self.dfs_find_cycles(i, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    fn dfs_find_cycles(
        &self,
        idx: usize,
        visited: &mut [bool],
        rec_stack: &mut [bool],
        path: &mut Vec<usize>,
        cycles: &mut Vec<Vec<usize>>,
    ) {
        visited[idx] = true;
        rec_stack[idx] = true;
        path.push(idx);

        // Check if this copy's destination is used by another copy
        let dest = self.copies[idx].dest;
        if let Some(dependent_indices) = self.dependencies.get(&dest) {
            for &dep_idx in dependent_indices {
                if !visited[dep_idx] {
                    self.dfs_find_cycles(dep_idx, visited, rec_stack, path, cycles);
                } else if rec_stack[dep_idx] {
                    // Found a cycle - extract it from the path
                    if let Some(cycle_start) = path.iter().position(|&x| x == dep_idx) {
                        cycles.push(path[cycle_start..].to_vec());
                    }
                }
            }
        }

        rec_stack[idx] = false;
        path.pop();
    }

    /// Generate assignments in correct order, breaking cycles with temporaries
    fn generate_assignments(self, function: &mut MirFunction, block_id: BasicBlockId) {
        let cycles = self.find_cycles();
        let mut temp_assignments = Vec::new();
        let mut modified_copies = self.copies.clone();

        // Break cycles by introducing temporaries
        for cycle in cycles {
            if cycle.len() > 1 {
                // Get the first copy in the cycle
                let first_idx = cycle[0];
                let first_copy = &modified_copies[first_idx];

                // Create a temporary with the same type as the source
                let temp_type = first_copy.ty.clone();
                let temp = function.new_typed_value_id(temp_type.clone());

                // Save the source value to the temporary
                temp_assignments.push(CopyOperation {
                    dest: temp,
                    source: first_copy.source,
                    ty: temp_type,
                });

                // Modify the first copy to use the temporary as source
                modified_copies[first_idx].source = Value::Operand(temp);
            }
        }

        // Perform topological sort for correct assignment order
        let sorted_indices = self.topological_sort(&modified_copies);

        // Generate assignments
        let block = function.basic_blocks.get_mut(block_id).unwrap();

        // First, insert temporary assignments to break cycles
        for temp_op in temp_assignments {
            block.instructions.push(Instruction::assign(
                temp_op.dest,
                temp_op.source,
                temp_op.ty,
            ));
        }

        // Then, insert the main assignments in dependency order
        for idx in sorted_indices {
            let copy = &modified_copies[idx];
            block
                .instructions
                .push(Instruction::assign(copy.dest, copy.source, copy.ty.clone()));
        }
    }

    /// Topological sort using Kahn's algorithm
    fn topological_sort(&self, copies: &[CopyOperation]) -> Vec<usize> {
        let mut in_degree = vec![0; copies.len()];
        let mut adj_list: FxHashMap<usize, Vec<usize>> = FxHashMap::default();

        // Build adjacency list and calculate in-degrees
        for (idx, copy) in copies.iter().enumerate() {
            if let Value::Operand(src_id) = copy.source {
                // Find copies that produce this source value
                for (other_idx, other_copy) in copies.iter().enumerate() {
                    if other_copy.dest == src_id {
                        adj_list.entry(other_idx).or_default().push(idx);
                        in_degree[idx] += 1;
                    }
                }
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: Vec<usize> = in_degree
            .iter()
            .enumerate()
            .filter(|(_, &degree)| degree == 0)
            .map(|(idx, _)| idx)
            .collect();

        let mut sorted = Vec::new();

        while let Some(idx) = queue.pop() {
            sorted.push(idx);

            if let Some(neighbors) = adj_list.get(&idx) {
                for &neighbor in neighbors {
                    in_degree[neighbor] -= 1;
                    if in_degree[neighbor] == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }

        // If we couldn't sort all copies, there's an unbreakable cycle (shouldn't happen after breaking cycles)
        if sorted.len() != copies.len() {
            // Fall back to original order
            (0..copies.len()).collect()
        } else {
            sorted
        }
    }
}

/// Eliminates all Phi nodes in a function
fn eliminate_phi_nodes(function: &mut MirFunction) -> bool {
    use crate::cfg::{is_critical_edge, split_critical_edge};

    let mut modified = false;
    let mut phi_replacements = Vec::new();
    let mut edge_splits = FxHashMap::default();

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

    // Group phi assignments by insertion location for parallel copy
    let mut insertion_groups: FxHashMap<BasicBlockId, Vec<CopyOperation>> = FxHashMap::default();

    // Second pass: group assignments by insertion block
    for (_phi_block_id, _phi_inst_idx, assignments) in &phi_replacements {
        for (pred_block_id, succ_block_id, dest, value, ty) in assignments {
            // Check if this is a critical edge
            let insert_block_id = if is_critical_edge(function, *pred_block_id, *succ_block_id) {
                // Check if we've already split this edge
                let edge_key = (*pred_block_id, *succ_block_id);
                *edge_splits.entry(edge_key).or_insert_with(|| {
                    split_critical_edge(function, *pred_block_id, *succ_block_id)
                })
            } else {
                *pred_block_id
            };

            // Add to the group for this insertion block
            insertion_groups
                .entry(insert_block_id)
                .or_default()
                .push(CopyOperation {
                    dest: *dest,
                    source: *value,
                    ty: ty.clone(),
                });
        }
    }

    // Third pass: generate parallel copy operations for each insertion block
    for (insert_block_id, copies) in insertion_groups {
        let mut graph = ParallelCopyGraph::new();
        for copy in copies {
            graph.add_copy(copy.dest, copy.source, copy.ty);
        }
        graph.generate_assignments(function, insert_block_id);
    }

    // Fourth pass: remove phi instructions
    for (phi_block_id, phi_inst_idx, _) in phi_replacements {
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
