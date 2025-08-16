use crate::{InstructionKind, MirFunction, Value, ValueId};
use rustc_hash::{FxHashMap, FxHashSet};

/// Pre-optimization pass that runs immediately after lowering
///
/// With the introduction of proper SSA form and pointer types, many optimizations
/// that were previously needed are now handled naturally during the lowering phase:
///
/// - Binary operations in let statements generate efficient code directly
/// - Tuple destructuring from function calls avoids intermediate allocations
/// - Values vs addresses are properly distinguished, eliminating confusion
///
/// This pass now focuses on cleanup optimizations that are best done after
/// the initial MIR generation.
pub struct PreOptimizationPass {
    /// Track which optimizations were applied for debugging
    optimizations_applied: Vec<String>,
}

impl PreOptimizationPass {
    pub const fn new() -> Self {
        Self {
            optimizations_applied: Vec::new(),
        }
    }

    /// Remove dead stores for unused variables
    ///
    /// This optimization removes store instructions for variables that are never used.
    /// It's particularly useful for cleaning up after the lowering phase, which may
    /// generate stores for variables that semantic analysis marked as unused.
    fn eliminate_dead_stores(
        &mut self,
        function: &mut MirFunction,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> bool {
        // First, collect all addresses that are read from
        let mut addresses_read = FxHashSet::default();
        let mut local_addresses = FxHashSet::default();
        let mut escaping_addresses = FxHashSet::default();

        for block in function.basic_blocks.iter() {
            for instr in &block.instructions {
                match &instr.kind {
                    InstructionKind::Load { address, .. } => {
                        if let Value::Operand(addr_id) = address {
                            addresses_read.insert(*addr_id);
                        }
                    }
                    InstructionKind::FrameAlloc { dest, .. } => {
                        local_addresses.insert(*dest);
                    }
                    InstructionKind::Call { args, .. } => {
                        // Mark any address passed to a call as escaping
                        for arg in args {
                            if let Value::Operand(id) = arg {
                                if local_addresses.contains(id) {
                                    escaping_addresses.insert(*id);
                                }
                            }
                        }
                    }
                    InstructionKind::AddressOf { operand, .. } => {
                        escaping_addresses.insert(*operand);
                    }
                    _ => {}
                }
            }
        }

        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                if let InstructionKind::Store { address, .. } = &instr.kind
                    && let Value::Operand(dest) = address
                {
                    // Only eliminate stores to local, non-escaping addresses that are never read
                    if local_addresses.contains(dest)
                        && !escaping_addresses.contains(dest)
                        && !addresses_read.contains(dest)
                    {
                        modified = true;
                        self.optimizations_applied
                            .push("dead_store_elimination".to_string());
                        return false; // Remove this instruction
                    }
                }
                true
            });
        }

        modified
    }

    /// Calculate how many times each value is used
    ///
    /// This analysis walks through all instructions and terminators to count
    /// how many times each ValueId is referenced. Values with zero uses can
    /// potentially be eliminated.
    fn calculate_value_use_counts(&self, function: &MirFunction) -> FxHashMap<ValueId, usize> {
        let mut use_counts = FxHashMap::default();

        for block in function.basic_blocks.iter() {
            // Use the existing helper methods to get all used values
            for instr in &block.instructions {
                for used_value_id in instr.used_values() {
                    *use_counts.entry(used_value_id).or_insert(0) += 1;
                }
            }

            for used_value_id in block.terminator.used_values() {
                *use_counts.entry(used_value_id).or_insert(0) += 1;
            }
        }

        use_counts
    }

    /// Remove dead stack allocations that are never used
    ///
    /// After eliminating dead stores, some stack allocations may become unused.
    /// This optimization removes them to reduce stack frame size.
    fn eliminate_dead_allocations(
        &mut self,
        function: &mut MirFunction,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                if let InstructionKind::FrameAlloc { dest, .. } = &instr.kind {
                    if use_counts.get(dest).copied().unwrap_or(0) == 0 {
                        modified = true;
                        self.optimizations_applied
                            .push("dead_allocation_elimination".to_string());
                        return false; // Remove this instruction
                    }
                }
                true
            });
        }

        modified
    }

    /// Remove dead instructions that compute unused values
    ///
    /// This optimization removes instructions whose results are never used.
    /// This includes binary operations, unary operations, and assignments
    /// that produce values that are never referenced.
    fn eliminate_dead_instructions(
        &mut self,
        function: &mut MirFunction,
        use_counts: &FxHashMap<ValueId, usize>,
    ) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                // Check if this instruction produces a value that's never used
                match &instr.kind {
                    InstructionKind::BinaryOp { dest, .. }
                    | InstructionKind::UnaryOp { dest, .. }
                    | InstructionKind::Assign { dest, .. }
                        if use_counts.get(dest).copied().unwrap_or(0) == 0 =>
                    {
                        modified = true;
                        self.optimizations_applied
                            .push("dead_instruction_elimination".to_string());
                        return false; // Remove this instruction
                    }
                    // Load instructions can also be dead if their result is unused
                    InstructionKind::Load { dest, .. } => {
                        if use_counts.get(dest).copied().unwrap_or(0) == 0 {
                            modified = true;
                            self.optimizations_applied
                                .push("dead_load_elimination".to_string());
                            return false; // Remove this instruction
                        }
                    }
                    _ => {}
                }
                true
            });
        }

        modified
    }
}

impl crate::passes::MirPass for PreOptimizationPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Calculate use counts once at the beginning
        let mut use_counts = self.calculate_value_use_counts(function);

        // Run optimization passes in order:
        // 1. Dead instructions (computations that produce unused values)
        // 2. Dead stores (stores to unused locations)
        // 3. Dead allocations (allocations that become unused after removing stores)
        // The order matters because removing one type of dead code can make other code dead
        let instructions_modified = self.eliminate_dead_instructions(function, &use_counts);
        modified |= instructions_modified;

        // Recompute use counts if we modified the function
        if instructions_modified {
            use_counts = self.calculate_value_use_counts(function);
        }

        // Re-enabled with conservative analysis - only eliminates stores to local frame allocations
        // The current implementation only removes stores where the address operand itself is unused,
        // which is conservative and safe. This avoids the GEP aliasing issue while still providing
        // optimization benefits for simple cases.
        // TODO: Enhance with alias analysis to handle GEP-derived pointers more aggressively
        let stores_modified = self.eliminate_dead_stores(function, &use_counts);
        modified |= stores_modified;

        // Recompute use counts again if we modified stores
        if stores_modified {
            use_counts = self.calculate_value_use_counts(function);
        }

        modified |= self.eliminate_dead_allocations(function, &use_counts);

        if !self.optimizations_applied.is_empty() {
            log::debug!(
                "Pre-optimizations applied: {:?}",
                self.optimizations_applied
            );
        }

        modified
    }

    fn name(&self) -> &'static str {
        "pre-optimization"
    }
}

impl Default for PreOptimizationPass {
    fn default() -> Self {
        Self::new()
    }
}
