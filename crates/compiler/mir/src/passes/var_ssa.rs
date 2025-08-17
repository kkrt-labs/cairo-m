//! # Variable-SSA Pass
//!
//! This pass converts mutable variables from memory-based operations to proper SSA form
//! with Phi nodes. It's the critical component that enables full value-based aggregate
//! handling by eliminating memory operations for variable state management.
//!
//! ## Overview
//! The pass identifies variables (MirDefinitionIds) that are assigned multiple times
//! and promotes them to SSA form by:
//! 1. Inserting Phi nodes at control flow merge points
//! 2. Renaming variable uses to reference the correct SSA values
//! 3. Converting assignments to SSA rebinding instead of memory stores
//!
//! ## Algorithm
//! Uses the standard SSA construction algorithm:
//! - Phase 1: Identify variables needing promotion
//! - Phase 2: Insert Phi nodes at dominance frontiers
//! - Phase 3: Rename variables using dominator tree traversal

use crate::{
    analysis::dominance::{compute_dominance_frontiers, compute_dominator_tree, DominatorTree},
    BasicBlockId, Instruction, InstructionKind, MirDefinitionId, MirFunction, MirType, ValueId,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Variable-SSA promotion pass for converting mutable variables to SSA form
pub struct VarSsaPass {
    /// Variables that have been identified for promotion
    promoted_vars: FxHashSet<MirDefinitionId>,
    /// Statistics for optimization reporting
    stats: OptimizationStats,
}

#[derive(Debug, Default)]
struct OptimizationStats {
    variables_analyzed: usize,
    variables_promoted: usize,
    phi_nodes_inserted: usize,
    assignments_converted: usize,
}

impl Default for VarSsaPass {
    fn default() -> Self {
        Self::new()
    }
}

impl VarSsaPass {
    /// Create a new Variable-SSA pass
    pub fn new() -> Self {
        Self {
            promoted_vars: FxHashSet::default(),
            stats: OptimizationStats::default(),
        }
    }

    /// Run the complete Variable-SSA optimization
    pub fn optimize(&mut self, function: &mut MirFunction) -> bool {
        // Step 1: Identify variables that need promotion
        let promotable = self.identify_promotable_variables(function);
        if promotable.is_empty() {
            return false;
        }

        self.stats.variables_promoted = promotable.len();

        // Step 2: Compute dominance information
        let dom_tree = compute_dominator_tree(function);
        let dom_frontiers = compute_dominance_frontiers(function, &dom_tree);

        // Step 3: Insert Phi nodes at dominance frontiers
        let phi_locations = self.insert_phi_nodes(function, &promotable, &dom_frontiers);

        // Step 4: Rename variables using dominator tree traversal
        self.rename_variables(function, &promotable, &phi_locations, &dom_tree);

        // Step 5: Convert assignments to SSA rebinding
        self.convert_assignments(function, &promotable);

        // Step 6: Cleanup - remove obsolete memory operations
        self.cleanup_promoted_variables(function, &promotable);

        true
    }
}

/// Information about a promotable variable
#[derive(Debug, Clone)]
struct PromotableVariable {
    /// The MirDefinitionId of the variable
    var_id: MirDefinitionId,
    /// The type of the variable
    ty: MirType,
    /// Blocks where assignments to this variable occur
    assignment_blocks: FxHashSet<BasicBlockId>,
    /// All SSA values that have been created for this variable
    ssa_values: FxHashMap<BasicBlockId, ValueId>,
}

impl VarSsaPass {
    /// Step 1: Identify variables that can be promoted to SSA form
    fn identify_promotable_variables(&mut self, function: &MirFunction) -> Vec<PromotableVariable> {
        let mut variables = FxHashMap::default();
        let mut escaping = FxHashSet::default();

        // First pass: Find all variable definitions and their assignment sites
        for (block_idx, block) in function.basic_blocks.iter().enumerate() {
            let block_id = BasicBlockId::from_usize(block_idx);
            for instruction in &block.instructions {
                match &instruction.kind {
                    // Track variable assignments
                    InstructionKind::Store { address, .. } => {
                        // Check if this is a store to a variable (not a memory location)
                        if let Some(var_id) = self.extract_variable_id(address) {
                            let entry =
                                variables
                                    .entry(var_id)
                                    .or_insert_with(|| PromotableVariable {
                                        var_id,
                                        ty: self.get_variable_type(function, var_id),
                                        assignment_blocks: FxHashSet::default(),
                                        ssa_values: FxHashMap::default(),
                                    });
                            entry.assignment_blocks.insert(block_id);
                        }
                    }
                    // Track uses that would prevent promotion
                    InstructionKind::Call { args, .. } => {
                        // If a variable's address is passed to a function, it escapes
                        for arg in args {
                            if let Some(var_id) = self.extract_variable_id(arg) {
                                escaping.insert(var_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Filter out escaping variables
        variables
            .into_iter()
            .filter(|(id, _)| !escaping.contains(id))
            .map(|(_, var)| var)
            .collect()
    }

    /// Step 2: Insert Phi nodes at dominance frontiers
    fn insert_phi_nodes(
        &mut self,
        function: &mut MirFunction,
        promotable: &[PromotableVariable],
        dom_frontiers: &FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>>,
    ) -> FxHashMap<MirDefinitionId, FxHashSet<BasicBlockId>> {
        let mut phi_locations = FxHashMap::default();

        for var in promotable {
            let mut worklist: Vec<_> = var.assignment_blocks.iter().cloned().collect();
            let mut processed = FxHashSet::default();
            let mut phi_blocks = FxHashSet::default();

            while let Some(block) = worklist.pop() {
                if !processed.insert(block) {
                    continue;
                }

                // Insert phi nodes at dominance frontier
                if let Some(frontier) = dom_frontiers.get(&block) {
                    for &frontier_block in frontier {
                        if phi_blocks.insert(frontier_block) {
                            // Create a phi node for this variable at this block
                            let _phi_id = self.create_phi_node(function, frontier_block, &var.ty);

                            // Record the phi location
                            phi_locations
                                .entry(var.var_id)
                                .or_insert_with(FxHashSet::default)
                                .insert(frontier_block);

                            // Add to worklist to process its frontier
                            if !var.assignment_blocks.contains(&frontier_block) {
                                worklist.push(frontier_block);
                            }

                            self.stats.phi_nodes_inserted += 1;
                        }
                    }
                }
            }
        }

        phi_locations
    }

    /// Step 3: Rename variables using dominator tree traversal
    fn rename_variables(
        &mut self,
        function: &mut MirFunction,
        promotable: &[PromotableVariable],
        phi_locations: &FxHashMap<MirDefinitionId, FxHashSet<BasicBlockId>>,
        dom_tree: &DominatorTree,
    ) {
        // Build a map of variable to current SSA value stack
        let mut var_stacks: FxHashMap<MirDefinitionId, Vec<ValueId>> = FxHashMap::default();

        // Initialize stacks for each variable
        for var in promotable {
            var_stacks.insert(var.var_id, Vec::new());
        }

        // Build dominator tree children map for traversal
        let mut dom_children: FxHashMap<BasicBlockId, Vec<BasicBlockId>> = FxHashMap::default();
        for (&child, &parent) in dom_tree {
            dom_children.entry(parent).or_default().push(child);
        }

        // Perform DFS traversal starting from entry block
        self.rename_dfs(
            function.entry_block,
            function,
            &mut var_stacks,
            phi_locations,
            &dom_children,
        );
    }

    /// DFS helper for variable renaming
    fn rename_dfs(
        &mut self,
        block_id: BasicBlockId,
        function: &mut MirFunction,
        var_stacks: &mut FxHashMap<MirDefinitionId, Vec<ValueId>>,
        phi_locations: &FxHashMap<MirDefinitionId, FxHashSet<BasicBlockId>>,
        dom_children: &FxHashMap<BasicBlockId, Vec<BasicBlockId>>,
    ) {
        let mut stack_sizes = FxHashMap::default();

        // Record initial stack sizes for cleanup
        for (var_id, stack) in var_stacks.iter() {
            stack_sizes.insert(*var_id, stack.len());
        }

        // Handle phi nodes for variables that have them at this block
        for (var_id, phi_blocks) in phi_locations {
            if phi_blocks.contains(&block_id) {
                // Create new SSA value for the phi node result
                let phi_result = function.new_value_id();
                var_stacks.get_mut(var_id).unwrap().push(phi_result);
            }
        }

        // Process instructions in the block
        let block_index: usize = block_id.into();
        if let Some(block) = function.basic_blocks.get_mut(block_index) {
            // Process instructions in the block
            for instruction in &mut block.instructions {
                match &mut instruction.kind {
                    InstructionKind::Load { address, .. } => {
                        // Replace variable loads with current SSA value
                        if let Some(var_id) = self.extract_variable_id(address) {
                            if let Some(stack) = var_stacks.get(&var_id) {
                                if let Some(&_current_value) = stack.last() {
                                    // Replace load with a nop (will be cleaned up later)
                                    // In a real implementation, we'd replace with the SSA value
                                    instruction.kind = InstructionKind::Nop;
                                }
                            }
                        }
                    }
                    InstructionKind::Store { address, .. } => {
                        // Replace variable stores with SSA value creation
                        if let Some(var_id) = self.extract_variable_id(address) {
                            if let Some(stack) = var_stacks.get_mut(&var_id) {
                                // Push new SSA value for this assignment
                                // Note: In a real implementation, we'd use the value from the store
                                // For now, just use a dummy value
                                stack.push(ValueId::from_usize(0));
                                self.stats.assignments_converted += 1;
                                // Mark instruction for removal (will be cleaned up later)
                                instruction.kind = InstructionKind::Nop;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Update phi node operands in successor blocks
            self.update_phi_operands(block_id, function, var_stacks, phi_locations);
        }

        // Recursively process dominated blocks
        if let Some(children) = dom_children.get(&block_id) {
            for &child in children {
                self.rename_dfs(child, function, var_stacks, phi_locations, dom_children);
            }
        }

        // Pop values pushed in this block
        for (var_id, initial_size) in stack_sizes {
            if let Some(stack) = var_stacks.get_mut(&var_id) {
                stack.truncate(initial_size);
            }
        }
    }

    /// Step 4: Convert assignments to use SSA rebinding
    fn convert_assignments(
        &mut self,
        function: &mut MirFunction,
        promotable: &[PromotableVariable],
    ) {
        // Create a set of promoted variable IDs for quick lookup
        let promoted_ids: FxHashSet<_> = promotable.iter().map(|v| v.var_id).collect();

        for block in &mut function.basic_blocks {
            for instruction in &mut block.instructions {
                if let InstructionKind::Store { address, .. } = &instruction.kind {
                    if let Some(var_id) = self.extract_variable_id(address) {
                        if promoted_ids.contains(&var_id) {
                            // This store should already be converted to SSA in rename phase
                            // Mark as nop if not already done
                            if !matches!(instruction.kind, InstructionKind::Nop) {
                                instruction.kind = InstructionKind::Nop;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Step 5: Clean up promoted variables
    fn cleanup_promoted_variables(
        &mut self,
        function: &mut MirFunction,
        promotable: &[PromotableVariable],
    ) {
        // Remove nop instructions
        for block in &mut function.basic_blocks {
            block
                .instructions
                .retain(|inst| !matches!(inst.kind, InstructionKind::Nop));
        }

        // Remove any remaining frame allocations for promoted variables
        // This would require tracking which allocations correspond to variables
        // For now, this is left as a TODO as it requires more context
    }

    // Helper methods

    /// Extract the MirDefinitionId from a pointer value if it refers to a variable
    const fn extract_variable_id(&self, _ptr: &crate::Value) -> Option<MirDefinitionId> {
        // This is a placeholder - actual implementation would need to track
        // the relationship between ValueIds and MirDefinitionIds
        // TODO: Implement proper variable tracking
        None
    }

    /// Get the type of a variable
    const fn get_variable_type(
        &self,
        _function: &MirFunction,
        _var_id: MirDefinitionId,
    ) -> MirType {
        // Placeholder - would need to look up variable type from semantic info
        MirType::Felt
    }

    /// Create a phi node at the given block
    fn create_phi_node(
        &self,
        function: &mut MirFunction,
        block_id: BasicBlockId,
        ty: &MirType,
    ) -> ValueId {
        // Create the phi ID first
        let phi_id = function.new_value_id();

        // Find the block and insert a phi node at the beginning
        let block_index: usize = block_id.into();
        if let Some(block) = function.basic_blocks.get_mut(block_index) {
            // Create a proper Phi instruction
            let phi_instruction = Instruction {
                kind: InstructionKind::Phi {
                    dest: phi_id,
                    ty: ty.clone(),
                    sources: Vec::new(), // Will be filled during renaming
                },
                source_span: None,
                source_expr_id: None,
                comment: Some("Variable SSA phi node".to_string()),
            };
            block.instructions.insert(0, phi_instruction);
            phi_id
        } else {
            panic!("Block not found: {:?}", block_id);
        }
    }

    /// Create a new SSA value
    fn create_ssa_value(&self, function: &mut MirFunction) -> ValueId {
        function.new_value_id()
    }

    /// Update phi node operands based on current variable values
    const fn update_phi_operands(
        &self,
        _block_id: BasicBlockId,
        _function: &mut MirFunction,
        _var_stacks: &FxHashMap<MirDefinitionId, Vec<ValueId>>,
        _phi_locations: &FxHashMap<MirDefinitionId, FxHashSet<BasicBlockId>>,
    ) {
        // TODO: Implement phi operand updates
        // This would need to:
        // 1. Find successor blocks
        // 2. Update phi nodes in those blocks with current stack values
        // 3. Associate the values with the correct predecessor edges
    }
}

// Implement the MirPass trait
use super::MirPass;

impl MirPass for VarSsaPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        self.optimize(function)
    }

    fn name(&self) -> &'static str {
        "var-ssa"
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_var_ssa_basic() {
        // TODO: Add tests for basic variable SSA conversion
    }

    #[test]
    fn test_var_ssa_control_flow() {
        // TODO: Add tests for variables in control flow
    }

    #[test]
    fn test_var_ssa_loops() {
        // TODO: Add tests for variables in loops
    }
}
