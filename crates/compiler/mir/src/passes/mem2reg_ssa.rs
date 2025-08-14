//! # Memory to Register Promotion Pass (SSA Version)
//!
//! This pass implements the classic Mem2Reg optimization using SSA construction.
//! It promotes stack-allocated variables to SSA virtual registers by:
//! 1. Identifying promotable allocations (non-escaping)
//! 2. Inserting Phi nodes at dominance frontiers
//! 3. Renaming variables using dominator tree traversal
//!
//! This transforms the MIR into true SSA form for promotable variables.

use crate::{
    analysis::dominance::{compute_dominance_frontiers, compute_dominator_tree, DominatorTree},
    layout::DataLayout,
    BasicBlockId, Instruction, InstructionKind, Literal, MirFunction, MirType, Terminator, Value,
    ValueId,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// SSA-based Memory to Register promotion pass
pub struct Mem2RegSsaPass {
    /// Statistics for optimization reporting
    stats: OptimizationStats,
}

#[derive(Debug, Default)]
struct OptimizationStats {
    allocations_analyzed: usize,
    allocations_promoted: usize,
    phi_nodes_inserted: usize,
    loads_eliminated: usize,
    stores_eliminated: usize,
}

impl Default for Mem2RegSsaPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Mem2RegSsaPass {
    /// Create a new SSA-based mem2reg pass
    pub fn new() -> Self {
        Self {
            stats: OptimizationStats::default(),
        }
    }

    /// Run the complete SSA-based mem2reg optimization
    pub fn optimize(&mut self, function: &mut MirFunction) -> bool {
        // Step 1: Identify promotable allocations
        let promotable = self.identify_promotable_allocations(function);
        if promotable.is_empty() {
            return false;
        }

        self.stats.allocations_promoted = promotable.len();

        // Step 2: Compute dominance information
        let dom_tree = compute_dominator_tree(function);
        let dom_frontiers = compute_dominance_frontiers(function, &dom_tree);

        // Step 3: Insert Phi nodes at dominance frontiers
        let phi_locations = self.insert_phi_nodes(function, &promotable, &dom_frontiers);

        // Step 4: Rename variables using dominator tree traversal
        self.rename_variables(function, &promotable, &phi_locations, &dom_tree);

        // Step 5: Remove promoted allocations and dead instructions
        self.cleanup_promoted_allocations(function, &promotable);

        true
    }
}

/// Information about a promotable allocation
#[derive(Debug, Clone)]
struct PromotableAllocation {
    /// The ValueId of the FrameAlloc instruction
    alloc_id: ValueId,
    /// The type of the allocated value
    ty: MirType,
    /// Blocks where stores to this allocation occur
    store_blocks: FxHashSet<BasicBlockId>,
    /// All GEP instructions derived from this allocation
    gep_values: FxHashMap<ValueId, i32>, // Maps GEP result to constant offset
}

impl Mem2RegSsaPass {
    /// Step 1: Identify allocations that can be promoted to registers
    fn identify_promotable_allocations(
        &mut self,
        function: &MirFunction,
    ) -> Vec<PromotableAllocation> {
        let mut allocations = FxHashMap::default();
        let mut escaping = FxHashSet::default();

        // IMPORTANT: Track allocations that are used as values (not just for loads/stores)
        // This is critical for handling tuple returns from function calls correctly.
        //
        // When a function returns a tuple, the MIR lowering creates an allocation to hold
        // the tuple values, but then returns the allocation address itself as the value
        // that represents the tuple. This allocation serves as a "value proxy" - other
        // code expects to receive this address and use it to access tuple elements.
        //
        // If we promote such allocations, we would eliminate the allocation instruction
        // but the code that expects to receive the allocation address would break.
        // For example, consider: let (a, b) = returns_tuple()
        // The lowering produces something like:
        //   %alloc = framealloc (felt, felt)
        //   %ret1, %ret2 = call returns_tuple
        //   store %alloc[0], %ret1
        //   store %alloc[1], %ret2
        //   %result = %alloc  // The allocation ADDRESS is the tuple value!
        //
        // If we promoted %alloc, the "%result = %alloc" would become invalid.
        let mut used_as_values = FxHashSet::default();

        // First pass: Find all allocations and GEPs
        for (_block_id, block) in function.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::FrameAlloc { dest, ty } => {
                        // IMPORTANT: Only track single-slot allocations for promotion
                        // Multi-slot allocations (u32, structs, tuples) require per-slot phi insertion
                        // which is not yet implemented. This is a temporary restriction until
                        // we implement either SROA (scalar replacement of aggregates) or
                        // per-slot phi insertion.
                        let layout = DataLayout::new();
                        if layout.is_promotable(ty) {
                            allocations.insert(
                                *dest,
                                PromotableAllocation {
                                    alloc_id: *dest,
                                    ty: ty.clone(),
                                    store_blocks: FxHashSet::default(),
                                    gep_values: FxHashMap::default(),
                                },
                            );
                        } else {
                            // TODO: this is currently inefficient, but we keep this for safety.
                            // Mark multi-slot allocations as escaping immediately
                            // This ensures they won't be considered for promotion
                            escaping.insert(*dest);
                        }
                        self.stats.allocations_analyzed += 1;
                    }
                    InstructionKind::GetElementPtr { dest, base, offset } => {
                        // Track GEPs with constant offsets
                        if let Value::Operand(base_id) = base {
                            if let Value::Literal(Literal::Integer(off)) = offset {
                                if let Some(alloc) = allocations.get_mut(base_id) {
                                    alloc.gep_values.insert(*dest, *off);
                                }
                                // Also check for chained GEPs
                                for alloc in allocations.values_mut() {
                                    if let Some(&base_off) = alloc.gep_values.get(base_id) {
                                        alloc.gep_values.insert(*dest, base_off + off);
                                    }
                                }
                            } else {
                                // Non-constant offset - mark as escaping
                                escaping.insert(*base_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Second pass: Check for escaping uses and collect store blocks
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::Store { address, value, ty } => {
                        if let Value::Operand(addr_id) = address {
                            // Check if storing to a tracked allocation
                            if let Some(alloc) = allocations.get_mut(addr_id) {
                                // CRITICAL BUG FIX: Check if we're storing a composite type
                                // The mem2reg pass cannot handle stores of entire tuples/structs
                                // because it tracks individual field values, not composite values.
                                // Example problematic IR:
                                //   %7 = framealloc (felt, felt, felt)
                                //   store %7, %3  // Storing entire tuple - cannot promote!
                                //   %10 = getelementptr %7, 1
                                //   %11 = load %10  // Expects individual value at offset 1
                                if matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }) {
                                    // Mark as escaping - cannot be promoted with composite stores
                                    escaping.insert(*addr_id);
                                }
                                alloc.store_blocks.insert(block_id);
                            } else {
                                // Check if it's a GEP from an allocation
                                for alloc in allocations.values_mut() {
                                    if alloc.gep_values.contains_key(addr_id) {
                                        alloc.store_blocks.insert(block_id);
                                    }
                                }
                            }

                            // Check if storing an allocation address (escape)
                            if let Value::Operand(val_id) = value {
                                if allocations.contains_key(val_id) {
                                    escaping.insert(*val_id);
                                }
                            }
                        }
                    }
                    InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                        // Any allocation passed to a call escapes
                        for arg in args {
                            if let Value::Operand(arg_id) = arg {
                                if allocations.contains_key(arg_id) {
                                    escaping.insert(*arg_id);
                                } else {
                                    // Check if it's a GEP from an allocation
                                    for alloc in allocations.values() {
                                        if alloc.gep_values.contains_key(arg_id) {
                                            escaping.insert(alloc.alloc_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    InstructionKind::AddressOf { operand, .. } => {
                        // Taking address of allocation means it escapes
                        if allocations.contains_key(operand) {
                            escaping.insert(*operand);
                        }
                    }
                    InstructionKind::Assign { source, .. } => {
                        // CRITICAL: Check if an allocation address is being used as a value
                        // This happens when tuple returns from function calls are assigned.
                        // Example: After calling a function that returns (felt, felt):
                        //   %alloc = framealloc (felt, felt)
                        //   %v1, %v2 = call func()
                        //   store %alloc[0], %v1
                        //   store %alloc[1], %v2
                        //   %tuple = %alloc      // <-- This Assign uses the allocation as a value!
                        //
                        // The %tuple = %alloc assignment means the allocation address itself
                        // has semantic meaning beyond just being storage. It represents the
                        // tuple value that other code will use to access the elements.
                        // We cannot promote such allocations without breaking this semantic.
                        if let Value::Operand(src_id) = source {
                            if allocations.contains_key(src_id) {
                                // Mark as used as value - these allocations cannot be promoted
                                used_as_values.insert(*src_id);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Check terminator for escaping values
            if let Terminator::Return { values } = &block.terminator {
                for value in values {
                    if let Value::Operand(val_id) = value {
                        if allocations.contains_key(val_id) {
                            escaping.insert(*val_id);
                        }
                    }
                }
            }
        }

        // Return only non-escaping allocations that are not used as values.
        //
        // WHY THIS FILTER IS NECESSARY:
        // Some allocations serve dual purposes - they are both storage AND values.
        // This happens with tuple returns from function calls where the MIR lowering
        // creates an allocation to hold the tuple elements, but then uses the allocation
        // address itself as the value representing the tuple.
        //
        // Without this filter, we would promote these allocations and eliminate them,
        // but then instructions that use the allocation address as a value would
        // reference non-existent values, creating invalid MIR like "%9 = %3" where
        // %3 was the removed allocation.
        //
        // This preserves the semantic meaning of allocations that represent compound
        // values in the MIR, ensuring the generated code remains valid after optimization.
        allocations
            .into_iter()
            .filter(|(id, _)| !escaping.contains(id) && !used_as_values.contains(id))
            .map(|(_, alloc)| alloc)
            .collect()
    }

    /// Step 2: Insert Phi nodes at dominance frontiers
    fn insert_phi_nodes(
        &mut self,
        function: &mut MirFunction,
        promotable: &[PromotableAllocation],
        dom_frontiers: &FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>>,
    ) -> FxHashMap<ValueId, FxHashMap<BasicBlockId, ValueId>> {
        let mut phi_locations = FxHashMap::default();

        for alloc in promotable {
            let mut phi_blocks = FxHashSet::default();
            let mut worklist = alloc.store_blocks.clone();
            let mut processed = FxHashSet::default();

            // Iteratively add Phi nodes at dominance frontiers
            while let Some(block) = worklist.iter().next().cloned() {
                worklist.remove(&block);
                if processed.insert(block) {
                    if let Some(frontier) = dom_frontiers.get(&block) {
                        for &frontier_block in frontier {
                            if phi_blocks.insert(frontier_block) {
                                // Add frontier block to worklist to find its frontiers
                                worklist.insert(frontier_block);
                            }
                        }
                    }
                }
            }

            // Create Phi nodes in identified blocks
            let mut alloc_phi_map = FxHashMap::default();
            for &phi_block_id in &phi_blocks {
                let phi_dest = function.new_typed_value_id(alloc.ty.clone());
                alloc_phi_map.insert(phi_block_id, phi_dest);

                // Insert empty Phi at the beginning of the block
                let phi_instr = Instruction::phi(phi_dest, alloc.ty.clone(), Vec::new());
                let block = &mut function.basic_blocks[phi_block_id];
                block.instructions.insert(0, phi_instr);

                self.stats.phi_nodes_inserted += 1;
            }

            phi_locations.insert(alloc.alloc_id, alloc_phi_map);
        }

        phi_locations
    }

    /// Step 3: Rename variables using dominator tree traversal
    fn rename_variables(
        &mut self,
        function: &mut MirFunction,
        promotable: &[PromotableAllocation],
        phi_locations: &FxHashMap<ValueId, FxHashMap<BasicBlockId, ValueId>>,
        dom_tree: &DominatorTree,
    ) {
        // Build promotable allocation map for quick lookup
        let mut alloc_map = FxHashMap::default();
        for alloc in promotable {
            alloc_map.insert(alloc.alloc_id, alloc);
        }

        // Build children map for dominator tree traversal
        let mut dom_children: FxHashMap<BasicBlockId, Vec<BasicBlockId>> = FxHashMap::default();
        for (&child, &parent) in dom_tree {
            dom_children.entry(parent).or_default().push(child);
        }

        // Initialize value stacks for each allocation and offset
        // Maps (alloc_id, offset) -> stack of values
        let mut value_stacks: FxHashMap<(ValueId, i32), Vec<Value>> = FxHashMap::default();
        for alloc in promotable {
            // Initialize stacks for all known offsets (from GEPs)
            value_stacks.insert((alloc.alloc_id, 0), Vec::new());
            for &offset in alloc.gep_values.values() {
                value_stacks.insert((alloc.alloc_id, offset), Vec::new());
            }
        }

        // Start renaming from the entry block
        let entry = function.entry_block;
        self.rename_block(
            function,
            entry,
            &alloc_map,
            &mut value_stacks,
            phi_locations,
            &dom_children,
        );
    }

    /// Recursively rename variables in a block and its dominator tree children
    fn rename_block(
        &mut self,
        function: &mut MirFunction,
        block_id: BasicBlockId,
        alloc_map: &FxHashMap<ValueId, &PromotableAllocation>,
        value_stacks: &mut FxHashMap<(ValueId, i32), Vec<Value>>,
        phi_locations: &FxHashMap<ValueId, FxHashMap<BasicBlockId, ValueId>>,
        dom_children: &FxHashMap<BasicBlockId, Vec<BasicBlockId>>,
    ) {
        let mut stack_pushes: Vec<((ValueId, i32), usize)> = Vec::new();
        let mut instructions_to_remove = Vec::new();

        // Process Phi nodes first - they define new values
        let block = &mut function.basic_blocks[block_id];
        for instruction in block.instructions.iter_mut() {
            if let InstructionKind::Phi { dest, .. } = &instruction.kind {
                // Find which allocation this Phi is for
                for (alloc_id, phi_map) in phi_locations {
                    if let Some(&phi_dest) = phi_map.get(&block_id) {
                        if *dest == phi_dest {
                            // Push this Phi's destination as the new current value
                            // Phi nodes are for offset 0 of the allocation
                            value_stacks
                                .get_mut(&(*alloc_id, 0))
                                .unwrap()
                                .push(Value::operand(*dest));
                            stack_pushes.push(((*alloc_id, 0), 1));
                            break;
                        }
                    }
                }
            }
        }

        // Process regular instructions
        for (idx, instruction) in block.instructions.iter_mut().enumerate() {
            match &mut instruction.kind {
                InstructionKind::Store { address, value, .. } => {
                    if let Value::Operand(addr_id) = address {
                        // Check if this is a store to a promotable allocation

                        // Direct store to allocation
                        if let Some(alloc) = alloc_map.get(addr_id) {
                            // Store to offset 0
                            value_stacks
                                .get_mut(&(alloc.alloc_id, 0))
                                .unwrap()
                                .push(*value);
                            stack_pushes.push(((alloc.alloc_id, 0), 1));
                            instructions_to_remove.push(idx);
                            self.stats.stores_eliminated += 1;
                        } else {
                            // Store through GEP - find the offset
                            for alloc in alloc_map.values() {
                                if let Some(&offset) = alloc.gep_values.get(addr_id) {
                                    // Store to the specific offset
                                    value_stacks
                                        .get_mut(&(alloc.alloc_id, offset))
                                        .unwrap()
                                        .push(*value);
                                    stack_pushes.push(((alloc.alloc_id, offset), 1));
                                    instructions_to_remove.push(idx);
                                    self.stats.stores_eliminated += 1;
                                    break;
                                }
                            }
                        }
                    }
                }
                InstructionKind::Load { dest, address, ty } => {
                    if let Value::Operand(addr_id) = address {
                        // Check if this is a load from a promotable allocation

                        // Direct load from allocation
                        if let Some(alloc) = alloc_map.get(addr_id) {
                            // Load from offset 0
                            if let Some(current_value) = value_stacks
                                .get(&(alloc.alloc_id, 0))
                                .and_then(|v| v.last())
                            {
                                // Replace all uses of the load's destination with the current value
                                // Use the load's type, not the allocation's type
                                *instruction =
                                    Instruction::assign(*dest, *current_value, ty.clone());
                                self.stats.loads_eliminated += 1;
                            }
                        } else {
                            // Load through GEP - find the offset
                            for alloc in alloc_map.values() {
                                if let Some(&offset) = alloc.gep_values.get(addr_id) {
                                    if let Some(current_value) = value_stacks
                                        .get(&(alloc.alloc_id, offset))
                                        .and_then(|v| v.last())
                                    {
                                        // Use the load's type (the element type), not the allocation's type
                                        *instruction =
                                            Instruction::assign(*dest, *current_value, ty.clone());
                                        self.stats.loads_eliminated += 1;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                InstructionKind::GetElementPtr { base, .. } => {
                    if let Value::Operand(base_id) = base {
                        // CRITICAL BUG FIX: Only remove GEPs from allocations that are being promoted
                        // alloc_map only contains allocations that passed all promotion checks
                        //
                        // We must NOT remove GEPs that:
                        // 1. Are based on non-promoted allocations
                        // 2. Are based on values that themselves are GEPs from non-promoted allocations
                        //
                        // Example: If %3 is not promoted (used as value), then:
                        //   %9 = %3  (assignment, %3 used as value)
                        //   %10 = getelementptr %9, 1  <- This GEP must NOT be removed!
                        //   %11 = load %10  <- This needs %10 to exist

                        let should_remove = if alloc_map.contains_key(base_id) {
                            // Direct GEP from a promoted allocation - remove it
                            true
                        } else {
                            // Check if this is a chained GEP from a promoted allocation
                            let mut remove = false;
                            for alloc in alloc_map.values() {
                                if alloc.gep_values.contains_key(base_id) {
                                    // This GEP is derived from a promoted allocation
                                    remove = true;
                                    break;
                                }
                            }
                            remove
                        };

                        if should_remove {
                            instructions_to_remove.push(idx);
                        }
                        // If not removing, the GEP stays because it's needed for accessing
                        // elements of non-promoted allocations
                    }
                }
                InstructionKind::FrameAlloc { dest, .. } => {
                    // Remove promoted allocations
                    if alloc_map.contains_key(dest) {
                        instructions_to_remove.push(idx);
                    }
                }
                InstructionKind::Assign { source, .. } => {
                    // Check if this assignment uses an allocation address as a value
                    if let Value::Operand(src_id) = source {
                        if alloc_map.contains_key(src_id) {
                            // This allocation is used as a value, not just for loads/stores
                            // We can't fully promote it - skip removing this instruction
                            // and don't remove the allocation itself
                            continue;
                        }
                    }
                }
                _ => {}
            }
        }

        // Update Phi nodes in successors
        let successors = function.basic_blocks[block_id].terminator.target_blocks();
        for succ_id in successors {
            let succ_block = &mut function.basic_blocks[succ_id];
            for instruction in &mut succ_block.instructions {
                // Get the destination if it's a Phi instruction
                let phi_dest = if let InstructionKind::Phi { dest, .. } = &instruction.kind {
                    Some(*dest)
                } else {
                    None
                };

                if let Some(dest) = phi_dest {
                    // Find which allocation this Phi is for
                    for (alloc_id, phi_map) in phi_locations {
                        if let Some(&expected_dest) = phi_map.get(&succ_id) {
                            if dest == expected_dest {
                                // Add entry for the current block
                                // Phi nodes track values at offset 0
                                if let Some(current_value) =
                                    value_stacks.get(&(*alloc_id, 0)).and_then(|v| v.last())
                                {
                                    if let InstructionKind::Phi { sources, .. } =
                                        &mut instruction.kind
                                    {
                                        sources.push((block_id, *current_value));
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Recursively process dominator tree children
        if let Some(children) = dom_children.get(&block_id) {
            for &child in children {
                self.rename_block(
                    function,
                    child,
                    alloc_map,
                    value_stacks,
                    phi_locations,
                    dom_children,
                );
            }
        }

        // Pop values pushed in this block
        for ((alloc_id, offset), count) in stack_pushes {
            let stack = value_stacks.get_mut(&(alloc_id, offset)).unwrap();
            for _ in 0..count {
                stack.pop();
            }
        }

        // Mark instructions for removal
        let block = &mut function.basic_blocks[block_id];
        for idx in instructions_to_remove.into_iter().rev() {
            block.instructions.remove(idx);
        }
    }

    /// Step 4: Clean up promoted allocations and dead instructions
    fn cleanup_promoted_allocations(
        &self,
        function: &mut MirFunction,
        _promotable: &[PromotableAllocation],
    ) {
        // Remove any remaining Phi nodes with no sources (dead code)
        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                if let InstructionKind::Phi { sources, .. } = &instr.kind {
                    !sources.is_empty()
                } else {
                    true
                }
            });
        }
    }
}

// Note: get_successors has been removed - using terminator.target_blocks() instead

impl crate::passes::MirPass for Mem2RegSsaPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        self.optimize(function)
    }

    fn name(&self) -> &'static str {
        "Mem2RegSsaPass"
    }
}

#[cfg(test)]
#[path = "./mem2reg_ssa_tests.rs"]
mod tests;
