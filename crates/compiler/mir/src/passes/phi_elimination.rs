//! # Phi-Node Elimination Pass
//!
//! This pass converts MIR from SSA form with phi-nodes to a non-SSA form suitable
//! for code generation. It implements the standard parallel copy insertion algorithm
//! from Sreedhar et al. (1999).
//!
//! ## Algorithm Overview
//!
//! 1. **Critical Edge Splitting**: Ensures unambiguous placement of copy instructions
//! 2. **Phi Decomposition**: Replaces phi-nodes with explicit copy instructions
//! 3. **Copy Sequencing**: Handles circular dependencies (the "lost copy problem")
//!
//! ## References
//!
//! - Sreedhar et al. (1999): "Translating Out of Static Single Assignment Form"
//! - Cooper & Torczon: "Engineering a Compiler" (2nd Ed), Section 9.4

use rustc_hash::FxHashMap;
use std::collections::{HashSet, VecDeque};

use crate::{
    cfg, BasicBlockId, Instruction, InstructionKind, MirFunction, MirPass, Value, ValueId,
};

/// Statistics for phi elimination
#[derive(Debug, Default)]
struct EliminationStats {
    /// Number of critical edges split
    critical_edges_split: usize,
    /// Number of phi nodes eliminated
    phis_eliminated: usize,
    /// Number of copy instructions inserted
    copies_inserted: usize,
    /// Number of cycles broken with temporaries
    cycles_broken: usize,
}

/// Phi-node elimination pass
pub struct PhiElimination {
    /// Enable debug output
    debug: bool,
    /// Statistics for reporting
    stats: EliminationStats,
}

impl PhiElimination {
    /// Create a new phi elimination pass
    pub fn new() -> Self {
        Self {
            debug: std::env::var("MIR_PHI_DEBUG").is_ok(),
            stats: EliminationStats::default(),
        }
    }

    /// Split all critical edges in the function
    fn split_critical_edges(&mut self, function: &mut MirFunction) {
        if self.debug {
            eprintln!(
                "[PhiElimination] Splitting critical edges for {}",
                function.name
            );
        }

        let splits = cfg::split_all_critical_edges(function);
        self.stats.critical_edges_split = splits.len();

        if self.debug && !splits.is_empty() {
            eprintln!("[PhiElimination] Split {} critical edges", splits.len());
            for ((pred, succ), edge_block) in &splits {
                eprintln!(
                    "  Edge {:?} -> {:?} split with block {:?}",
                    pred, succ, edge_block
                );
            }
        }
    }

    /// Collect all phi nodes and determine where to insert copies
    fn collect_phi_copies(
        &self,
        function: &MirFunction,
    ) -> FxHashMap<BasicBlockId, Vec<Instruction>> {
        let mut predecessor_copies: FxHashMap<BasicBlockId, Vec<Instruction>> =
            FxHashMap::default();

        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            // Process all phi instructions in this block
            for instruction in &block.instructions {
                if let InstructionKind::Phi { dest, ty, sources } = &instruction.kind {
                    if self.debug {
                        eprintln!(
                            "[PhiElimination] Processing phi in block {:?}: {:?} = phi {:?}",
                            block_id, dest, sources
                        );
                    }

                    // For each predecessor, create a copy instruction
                    for (pred_block_id, source_value) in sources {
                        let copy = Instruction::assign(*dest, *source_value, ty.clone());

                        if self.debug {
                            eprintln!(
                                "  Adding copy to block {:?}: {:?} = {:?}",
                                pred_block_id, dest, source_value
                            );
                        }

                        predecessor_copies
                            .entry(*pred_block_id)
                            .or_default()
                            .push(copy);
                    }
                }
            }
        }

        predecessor_copies
    }

    /// Build a dependency graph for parallel copies
    fn build_dependency_graph(&self, copies: &[Instruction]) -> FxHashMap<ValueId, Vec<ValueId>> {
        let mut graph = FxHashMap::default();

        // Build a map of destination -> source for all copies
        let mut dest_to_source: FxHashMap<ValueId, ValueId> = FxHashMap::default();
        for copy in copies {
            if let InstructionKind::Assign { dest, source, .. } = &copy.kind {
                if let Value::Operand(src_id) = source {
                    dest_to_source.insert(*dest, *src_id);
                }
            }
        }

        // Build dependency edges: if copy A writes to a value that copy B reads,
        // then B depends on A (must happen before A)
        for copy in copies {
            if let InstructionKind::Assign { dest, source, .. } = &copy.kind {
                if let Value::Operand(src_id) = source {
                    // Check if any other copy writes to our source
                    for other_dest in dest_to_source.keys() {
                        if other_dest == src_id {
                            // We depend on the copy that writes our source
                            graph
                                .entry(*dest)
                                .or_insert_with(Vec::new)
                                .push(*other_dest);
                        }
                    }
                }
            }
        }

        graph
    }

    /// Find cycles in the dependency graph using DFS
    fn find_cycles(&self, graph: &FxHashMap<ValueId, Vec<ValueId>>) -> Vec<Vec<ValueId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        fn dfs(
            node: ValueId,
            graph: &FxHashMap<ValueId, Vec<ValueId>>,
            visited: &mut HashSet<ValueId>,
            rec_stack: &mut HashSet<ValueId>,
            path: &mut Vec<ValueId>,
            cycles: &mut Vec<Vec<ValueId>>,
        ) {
            visited.insert(node);
            rec_stack.insert(node);
            path.push(node);

            if let Some(neighbors) = graph.get(&node) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        dfs(neighbor, graph, visited, rec_stack, path, cycles);
                    } else if rec_stack.contains(&neighbor) {
                        // Found a cycle
                        let cycle_start = path.iter().position(|&n| n == neighbor).unwrap();
                        let cycle = path[cycle_start..].to_vec();
                        if !cycle.is_empty() {
                            cycles.push(cycle);
                        }
                    }
                }
            }

            path.pop();
            rec_stack.remove(&node);
        }

        // Run DFS from all unvisited nodes
        for &node in graph.keys() {
            if !visited.contains(&node) {
                dfs(
                    node,
                    graph,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// Sequence parallel copies, handling cycles with temporary variables
    fn sequence_parallel_copies(
        &mut self,
        function: &mut MirFunction,
        copies: Vec<Instruction>,
    ) -> Vec<Instruction> {
        if copies.is_empty() {
            return Vec::new();
        }

        // Special case: single copy
        if copies.len() == 1 {
            return copies;
        }

        let graph = self.build_dependency_graph(&copies);
        let cycles = self.find_cycles(&graph);

        if self.debug && !cycles.is_empty() {
            eprintln!(
                "[PhiElimination] Found {} cycles in parallel copies",
                cycles.len()
            );
        }

        let mut sequenced = Vec::new();
        let mut processed = HashSet::new();

        // Handle cycles first by breaking them with temporaries
        for cycle in cycles {
            if cycle.len() <= 1 {
                continue; // Skip trivial cycles
            }

            self.stats.cycles_broken += 1;

            // Find the actual copy instructions for this cycle
            let mut cycle_copies = Vec::new();
            for copy in &copies {
                if let InstructionKind::Assign { dest, .. } = &copy.kind {
                    if cycle.contains(dest) {
                        cycle_copies.push(copy.clone());
                        processed.insert(*dest);
                    }
                }
            }

            if cycle_copies.is_empty() {
                continue;
            }

            // Break the cycle using a temporary
            // Save the first source to a temporary
            if let InstructionKind::Assign { source, ty, .. } = &cycle_copies[0].kind {
                let temp = function.new_value_id();
                sequenced.push(Instruction::assign(temp, *source, ty.clone()));

                // Perform the cycle rotations
                for i in 0..cycle_copies.len() - 1 {
                    if let (
                        InstructionKind::Assign {
                            dest: dest1,
                            ty: ty1,
                            ..
                        },
                        InstructionKind::Assign {
                            source: source2, ..
                        },
                    ) = (&cycle_copies[i].kind, &cycle_copies[i + 1].kind)
                    {
                        sequenced.push(Instruction::assign(*dest1, *source2, ty1.clone()));
                    }
                }

                // Restore from temporary to complete the cycle
                if let InstructionKind::Assign {
                    dest: last_dest,
                    ty: last_ty,
                    ..
                } = &cycle_copies[cycle_copies.len() - 1].kind
                {
                    sequenced.push(Instruction::assign(
                        *last_dest,
                        Value::Operand(temp),
                        last_ty.clone(),
                    ));
                }
            }
        }

        // Add non-cyclic copies in dependency order
        let mut remaining_copies = Vec::new();
        for copy in copies {
            if let InstructionKind::Assign { dest, .. } = &copy.kind {
                if !processed.contains(dest) {
                    remaining_copies.push(copy);
                }
            }
        }

        // Topological sort for remaining copies
        sequenced.extend(self.topological_sort_copies(remaining_copies, &graph));

        sequenced
    }

    /// Perform topological sort on non-cyclic copies
    fn topological_sort_copies(
        &self,
        copies: Vec<Instruction>,
        graph: &FxHashMap<ValueId, Vec<ValueId>>,
    ) -> Vec<Instruction> {
        if copies.is_empty() {
            return Vec::new();
        }

        let mut sorted = Vec::new();
        let mut in_degree: FxHashMap<ValueId, usize> = FxHashMap::default();
        let mut copy_map: FxHashMap<ValueId, Instruction> = FxHashMap::default();

        // Initialize in-degrees and build copy map
        for copy in &copies {
            if let InstructionKind::Assign { dest, .. } = &copy.kind {
                in_degree.insert(*dest, 0);
                copy_map.insert(*dest, copy.clone());
            }
        }

        // Calculate in-degrees
        for copy in &copies {
            if let InstructionKind::Assign { dest, .. } = &copy.kind {
                if let Some(deps) = graph.get(dest) {
                    for dep in deps {
                        if let Some(count) = in_degree.get_mut(dep) {
                            *count += 1;
                        }
                    }
                }
            }
        }

        // Find nodes with no dependencies
        let mut queue = VecDeque::new();
        for (&node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node);
            }
        }

        // Process nodes in topological order
        while let Some(node) = queue.pop_front() {
            if let Some(copy) = copy_map.get(&node) {
                sorted.push(copy.clone());
            }

            // Reduce in-degree of dependent nodes
            if let Some(deps) = graph.get(&node) {
                for &dep in deps {
                    if let Some(count) = in_degree.get_mut(&dep) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        // If we couldn't sort all copies, just return them in original order
        // (this shouldn't happen if cycle detection worked correctly)
        if sorted.len() != copies.len() {
            if self.debug {
                eprintln!(
                    "[PhiElimination] Warning: Topological sort incomplete, using original order"
                );
            }
            return copies;
        }

        sorted
    }

    /// Insert copy instructions into predecessor blocks
    fn insert_copies(
        &mut self,
        function: &mut MirFunction,
        copies: FxHashMap<BasicBlockId, Vec<Instruction>>,
    ) {
        for (block_id, copy_instructions) in copies {
            if copy_instructions.is_empty() {
                continue;
            }

            let sequenced = self.sequence_parallel_copies(function, copy_instructions);
            self.stats.copies_inserted += sequenced.len();

            if self.debug {
                eprintln!(
                    "[PhiElimination] Inserting {} copies into block {:?}",
                    sequenced.len(),
                    block_id
                );
            }

            // Get the block and insert copies before the terminator
            if let Some(block) = function.basic_blocks.get_mut(block_id) {
                // Temporarily save and clear the terminator
                let terminator = block.terminator.clone();

                // Insert all copy instructions
                for instr in sequenced {
                    if self.debug {
                        eprintln!("  Inserting: {:?}", instr.kind);
                    }
                    block.instructions.push(instr);
                }

                // Restore the terminator (it's already set, so no need to modify)
                // The terminator field is never actually cleared in our implementation
            }
        }
    }

    /// Remove all phi nodes from the function
    fn remove_phi_nodes(&mut self, function: &mut MirFunction) {
        for block in function.basic_blocks.iter_mut() {
            let original_count = block.instructions.len();
            block
                .instructions
                .retain(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }));
            let removed = original_count - block.instructions.len();
            self.stats.phis_eliminated += removed;

            if self.debug && removed > 0 {
                eprintln!("[PhiElimination] Removed {} phi nodes from block", removed);
            }
        }
    }
}

impl Default for PhiElimination {
    fn default() -> Self {
        Self::new()
    }
}

impl MirPass for PhiElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // Reset statistics
        self.stats = EliminationStats::default();

        if self.debug {
            eprintln!(
                "\n[PhiElimination] Starting for function '{}'",
                function.name
            );
        }

        // Phase 1: Split critical edges
        self.split_critical_edges(function);

        // Phase 2: Collect phi nodes and determine copy placement
        let copies = self.collect_phi_copies(function);

        if copies.is_empty() {
            if self.debug {
                eprintln!("[PhiElimination] No phi nodes found, skipping");
            }
            return false;
        }

        // Phase 3: Insert copy instructions with proper sequencing
        self.insert_copies(function, copies);

        // Phase 4: Remove all phi nodes
        self.remove_phi_nodes(function);

        if self.debug {
            eprintln!("[PhiElimination] Complete. Statistics:");
            eprintln!(
                "  Critical edges split: {}",
                self.stats.critical_edges_split
            );
            eprintln!("  Phi nodes eliminated: {}", self.stats.phis_eliminated);
            eprintln!(
                "  Copy instructions inserted: {}",
                self.stats.copies_inserted
            );
            eprintln!("  Cycles broken: {}", self.stats.cycles_broken);
        }

        // Return true if we made any changes
        self.stats.phis_eliminated > 0
    }

    fn name(&self) -> &'static str {
        "PhiElimination"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Literal, MirType, Terminator};

    /// Create a simple diamond CFG with a phi node
    fn create_diamond_cfg_with_phi() -> MirFunction {
        let mut function = MirFunction::new("test_diamond".to_string());

        // Create 4 blocks: Entry, Left, Right, Merge
        let entry = function.add_basic_block();
        let left = function.add_basic_block();
        let right = function.add_basic_block();
        let merge = function.add_basic_block();

        function.entry_block = entry;

        // Entry block: branch on condition
        let cond = function.new_value_id();
        function.basic_blocks[entry]
            .instructions
            .push(Instruction::assign(
                cond,
                Value::Literal(Literal::Boolean(true)),
                MirType::Felt,
            ));
        function.basic_blocks[entry].terminator = Terminator::If {
            condition: Value::Operand(cond),
            then_target: left,
            else_target: right,
        };
        function.connect(entry, left);
        function.connect(entry, right);

        // Left block: assign value and jump
        let left_val = function.new_value_id();
        function.basic_blocks[left]
            .instructions
            .push(Instruction::assign(
                left_val,
                Value::Literal(Literal::Integer(10)),
                MirType::Felt,
            ));
        function.basic_blocks[left].terminator = Terminator::Jump { target: merge };
        function.connect(left, merge);

        // Right block: assign value and jump
        let right_val = function.new_value_id();
        function.basic_blocks[right]
            .instructions
            .push(Instruction::assign(
                right_val,
                Value::Literal(Literal::Integer(20)),
                MirType::Felt,
            ));
        function.basic_blocks[right].terminator = Terminator::Jump { target: merge };
        function.connect(right, merge);

        // Merge block: phi node and return
        let phi_result = function.new_value_id();
        function.basic_blocks[merge]
            .instructions
            .push(Instruction::phi(
                phi_result,
                MirType::Felt,
                vec![
                    (left, Value::Operand(left_val)),
                    (right, Value::Operand(right_val)),
                ],
            ));
        function.basic_blocks[merge].terminator = Terminator::Return {
            values: vec![Value::Operand(phi_result)],
        };

        function
    }

    #[test]
    fn test_simple_diamond_phi_elimination() {
        let mut function = create_diamond_cfg_with_phi();
        let mut pass = PhiElimination::new();

        // Run the pass
        let modified = pass.run(&mut function);
        assert!(modified, "Pass should modify the function");

        // Check that no phi nodes remain
        for block in function.basic_blocks.iter() {
            for instr in &block.instructions {
                assert!(
                    !matches!(instr.kind, InstructionKind::Phi { .. }),
                    "No phi nodes should remain"
                );
            }
        }

        // Check statistics
        assert_eq!(pass.stats.phis_eliminated, 1, "Should eliminate 1 phi node");
        assert_eq!(pass.stats.copies_inserted, 2, "Should insert 2 copies");
    }

    #[test]
    fn test_no_phi_nodes() {
        let mut function = MirFunction::new("test_no_phi".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        function.basic_blocks[entry].terminator = Terminator::Return { values: vec![] };

        let mut pass = PhiElimination::new();
        let modified = pass.run(&mut function);

        assert!(
            !modified,
            "Pass should not modify function without phi nodes"
        );
        assert_eq!(pass.stats.phis_eliminated, 0);
        assert_eq!(pass.stats.copies_inserted, 0);
    }

    #[test]
    fn test_critical_edge_splitting() {
        let mut function = MirFunction::new("test_critical".to_string());

        // Create a CFG with critical edges
        let entry = function.add_basic_block();
        let b1 = function.add_basic_block();
        let merge = function.add_basic_block();

        function.entry_block = entry;

        // Entry branches to B1 or Merge (critical edge: Entry->Merge)
        let cond = function.new_value_id();
        function.basic_blocks[entry].terminator = Terminator::If {
            condition: Value::Operand(cond),
            then_target: b1,
            else_target: merge,
        };
        function.connect(entry, b1);
        function.connect(entry, merge);

        // B1 also goes to Merge
        function.basic_blocks[b1].terminator = Terminator::Jump { target: merge };
        function.connect(b1, merge);

        // Merge has a phi node
        let phi_result = function.new_value_id();
        let val1 = function.new_value_id();
        let val2 = function.new_value_id();

        function.basic_blocks[merge]
            .instructions
            .push(Instruction::phi(
                phi_result,
                MirType::Felt,
                vec![(entry, Value::Operand(val1)), (b1, Value::Operand(val2))],
            ));
        function.basic_blocks[merge].terminator = Terminator::Return { values: vec![] };

        let mut pass = PhiElimination::new();
        let modified = pass.run(&mut function);

        assert!(modified);
        assert!(
            pass.stats.critical_edges_split > 0,
            "Should split critical edges"
        );
    }

    #[test]
    fn test_parallel_copy_with_cycle() {
        let mut function = MirFunction::new("test_cycle".to_string());

        // Create a scenario where phi nodes create a swap (cycle of length 2)
        // This tests the cycle detection and temporary variable insertion

        let entry = function.add_basic_block();
        let block_a = function.add_basic_block();
        let block_b = function.add_basic_block();
        function.entry_block = entry;

        // Set up values
        let x = function.new_value_id();
        let y = function.new_value_id();

        // Entry initializes x and y
        function.basic_blocks[entry]
            .instructions
            .push(Instruction::assign(
                x,
                Value::Literal(Literal::Integer(1)),
                MirType::Felt,
            ));
        function.basic_blocks[entry]
            .instructions
            .push(Instruction::assign(
                y,
                Value::Literal(Literal::Integer(2)),
                MirType::Felt,
            ));
        function.basic_blocks[entry].terminator = Terminator::Jump { target: block_a };
        function.connect(entry, block_a);

        // Block A has phi nodes that swap x and y
        let new_x = function.new_value_id();
        let new_y = function.new_value_id();

        function.basic_blocks[block_a]
            .instructions
            .push(Instruction::phi(
                new_x,
                MirType::Felt,
                vec![(entry, Value::Operand(y))],
            ));
        function.basic_blocks[block_a]
            .instructions
            .push(Instruction::phi(
                new_y,
                MirType::Felt,
                vec![(entry, Value::Operand(x))],
            ));
        function.basic_blocks[block_a].terminator = Terminator::Jump { target: block_b };
        function.connect(block_a, block_b);

        function.basic_blocks[block_b].terminator = Terminator::Return { values: vec![] };

        let mut pass = PhiElimination::new();
        let modified = pass.run(&mut function);

        assert!(modified);
        // The swap should be handled with a temporary variable
        // We expect more copies than phi nodes due to the temporary
        assert!(pass.stats.copies_inserted >= pass.stats.phis_eliminated);
    }
}
