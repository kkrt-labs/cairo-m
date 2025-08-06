//! # Memory to Register Promotion Pass
//!
//! This optimization pass promotes memory operations (load/store) to direct register operations
//! when possible, eliminating redundant memory accesses for non-escaping allocations.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    BasicBlock, BasicBlockId, Instruction, InstructionKind, Literal, MirFunction, Terminator,
    Value, ValueId,
};

/// Memory to Register promotion pass configuration
#[derive(Debug, Clone)]
pub struct Mem2RegConfig {
    /// Enable full SSA-based promotion (more aggressive)
    pub enable_full_promotion: bool,
    /// Enable partial promotion for individual fields
    pub enable_partial_promotion: bool,
    /// Enable simple store-to-load forwarding within blocks
    pub enable_store_forwarding: bool,
    /// Maximum allocation size in slots to consider for optimization
    pub max_allocation_size: usize,
    /// Maximum iterations for fixed-point algorithms
    pub max_iterations: usize,
}

impl Default for Mem2RegConfig {
    fn default() -> Self {
        Self {
            enable_full_promotion: true,
            enable_partial_promotion: true,
            enable_store_forwarding: true,
            max_allocation_size: 16, // Don't optimize huge structs
            max_iterations: 10,      // Prevent infinite loops
        }
    }
}

/// Memory to Register promotion pass
pub struct Mem2RegPass {
    config: Mem2RegConfig,
    stats: OptimizationStats,
}

#[derive(Debug, Default)]
struct OptimizationStats {
    allocations_analyzed: usize,
    allocations_promoted: usize,
    loads_eliminated: usize,
    stores_eliminated: usize,
    geps_eliminated: usize,
}

impl Default for Mem2RegPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Mem2RegPass {
    /// Create a new mem2reg pass with default configuration
    pub fn new() -> Self {
        Self {
            config: Mem2RegConfig::default(),
            stats: OptimizationStats::default(),
        }
    }

    /// Create a new mem2reg pass with custom configuration
    pub fn with_config(config: Mem2RegConfig) -> Self {
        Self {
            config,
            stats: OptimizationStats::default(),
        }
    }

    /// Run the complete mem2reg optimization pipeline
    pub fn optimize(&mut self, function: &mut MirFunction) -> bool {
        let mut changed = false;

        // Phase 1: Simple store-to-load forwarding within blocks
        if self.config.enable_store_forwarding {
            changed |= self.run_store_forwarding(function);
        }

        // Phase 2: Full promotion with escape analysis
        if self.config.enable_full_promotion {
            let allocations = self.collect_allocations(function);
            let promotable = self.analyze_escapes(function, allocations);
            changed |= self.promote_allocations(function, promotable);
        }

        // Phase 3: Dead code elimination for removed instructions
        if changed {
            self.cleanup_dead_instructions(function);
        }

        eprintln!("Mem2Reg Stats: {:?}", self.stats);
        changed
    }
}

/// Information about a stack allocation
#[derive(Debug, Clone)]
struct AllocationInfo {
    /// The ValueId of the allocation
    alloc_id: ValueId,
    /// Size of the allocation in slots
    #[allow(dead_code)]
    size: usize,
    /// All access points to this allocation
    access_points: Vec<AccessPoint>,
    /// Whether this allocation escapes (address taken, passed to call, etc.)
    escapes: bool,
    /// Blocks that contain accesses to this allocation
    accessed_blocks: FxHashSet<BasicBlockId>,
}

/// An access point to an allocation
#[derive(Debug, Clone)]
struct AccessPoint {
    /// Block containing the access
    block_id: BasicBlockId,
    /// Index of the instruction within the block
    instruction_index: usize,
    /// Type of access
    kind: AccessKind,
    /// Offset from the base allocation (must be constant)
    offset: Option<i32>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum AccessKind {
    /// store address, value
    Store { address: Value, value: Value },
    /// dest = load address
    Load { dest: ValueId, address: Value },
    /// dest = getelementptr base, offset
    GetElementPtr {
        dest: ValueId,
        base: Value,
        offset: Value,
    },
    /// Address escapes (passed to call, stored, etc.)
    Escape,
}

impl Mem2RegPass {
    /// Phase 1: Simple store-to-load forwarding within blocks
    fn run_store_forwarding(&mut self, function: &mut MirFunction) -> bool {
        let mut changed = false;
        let mut replacements = FxHashMap::default();

        for block in function.basic_blocks.iter_mut() {
            let forwarded = self.forward_stores_in_block(block, &mut replacements);
            if forwarded > 0 {
                changed = true;
                self.stats.loads_eliminated += forwarded;
            }
        }

        // Apply replacements throughout the function
        if !replacements.is_empty() {
            self.apply_replacements(function, &replacements);
        }

        changed
    }

    /// Forward stores to loads within a single block
    fn forward_stores_in_block(
        &self,
        block: &mut BasicBlock,
        replacements: &mut FxHashMap<ValueId, Value>,
    ) -> usize {
        let mut available_values: FxHashMap<(ValueId, i32), Value> = FxHashMap::default();
        let mut forwarded_count = 0;

        for instruction in &mut block.instructions {
            match &instruction.kind {
                InstructionKind::Store { address, value } => {
                    // Track the stored value at this memory location
                    if let Some((base, offset)) = self.analyze_address(address) {
                        available_values.insert((base, offset), *value);
                    }
                }
                InstructionKind::Load { dest, address } => {
                    // Try to forward a previously stored value
                    if let Some((base, offset)) = self.analyze_address(address) {
                        if let Some(value) = available_values.get(&(base, offset)) {
                            // We can forward the store to this load!
                            replacements.insert(*dest, *value);
                            // Mark instruction for removal
                            instruction.kind = InstructionKind::Debug {
                                message: "mem2reg: forwarded load".to_string(),
                                values: vec![],
                            };
                            forwarded_count += 1;
                        }
                    }
                }
                InstructionKind::Call { .. } | InstructionKind::VoidCall { .. } => {
                    // Conservative: clear available values on function calls
                    // as they might have side effects
                    available_values.clear();
                }
                _ => {}
            }
        }

        forwarded_count
    }

    /// Analyze an address to extract base allocation and constant offset
    const fn analyze_address(&self, address: &Value) -> Option<(ValueId, i32)> {
        match address {
            Value::Operand(id) => {
                // This could be either a direct allocation or a getelementptr result
                // For now, treat it as offset 0 from itself
                Some((*id, 0))
            }
            _ => None,
        }
    }

    /// Collect all stack allocations in the function
    fn collect_allocations(&mut self, function: &MirFunction) -> Vec<AllocationInfo> {
        let mut allocations = Vec::new();

        for (_block_id, block) in function.basic_blocks() {
            for instruction in block.instructions.iter() {
                if let InstructionKind::StackAlloc { dest, size } = &instruction.kind {
                    if *size <= self.config.max_allocation_size {
                        allocations.push(AllocationInfo {
                            alloc_id: *dest,
                            size: *size,
                            access_points: Vec::new(),
                            escapes: false,
                            accessed_blocks: FxHashSet::default(),
                        });
                        self.stats.allocations_analyzed += 1;
                    }
                }
            }
        }

        allocations
    }

    /// Analyze which allocations escape and collect their access points
    #[allow(clippy::cognitive_complexity)]
    fn analyze_escapes(
        &self,
        function: &MirFunction,
        mut allocations: Vec<AllocationInfo>,
    ) -> Vec<AllocationInfo> {
        // Build a map for quick lookup
        let mut alloc_map: FxHashMap<ValueId, usize> = FxHashMap::default();
        for (idx, alloc) in allocations.iter().enumerate() {
            alloc_map.insert(alloc.alloc_id, idx);
        }

        // Track getelementptr results and their base allocations
        let mut gep_bases: FxHashMap<ValueId, (ValueId, i32)> = FxHashMap::default();

        // Analyze all instructions
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    InstructionKind::GetElementPtr { dest, base, offset } => {
                        // Track GEP relationships
                        if let Value::Operand(base_id) = base {
                            if let Some(&alloc_idx) = alloc_map.get(base_id) {
                                // Direct GEP from allocation
                                if let Value::Literal(Literal::Integer(off)) = offset {
                                    gep_bases.insert(*dest, (*base_id, *off));
                                    allocations[alloc_idx].access_points.push(AccessPoint {
                                        block_id,
                                        instruction_index: instr_idx,
                                        kind: AccessKind::GetElementPtr {
                                            dest: *dest,
                                            base: *base,
                                            offset: *offset,
                                        },
                                        offset: Some(*off),
                                    });
                                    allocations[alloc_idx].accessed_blocks.insert(block_id);
                                } else {
                                    // Non-constant offset - allocation escapes
                                    allocations[alloc_idx].escapes = true;
                                }
                            } else if let Some(&(base_alloc, base_off)) = gep_bases.get(base_id) {
                                // Chained GEP
                                if let Some(&alloc_idx) = alloc_map.get(&base_alloc) {
                                    if let Value::Literal(Literal::Integer(off)) = offset {
                                        let total_offset = base_off + off;
                                        gep_bases.insert(*dest, (base_alloc, total_offset));
                                        allocations[alloc_idx].access_points.push(AccessPoint {
                                            block_id,
                                            instruction_index: instr_idx,
                                            kind: AccessKind::GetElementPtr {
                                                dest: *dest,
                                                base: *base,
                                                offset: *offset,
                                            },
                                            offset: Some(total_offset),
                                        });
                                        allocations[alloc_idx].accessed_blocks.insert(block_id);
                                    } else {
                                        allocations[alloc_idx].escapes = true;
                                    }
                                }
                            }
                        }
                    }
                    InstructionKind::Load { dest, address } => {
                        if let Value::Operand(addr_id) = address {
                            // Check if this is a load from a tracked allocation or GEP
                            if let Some(&alloc_idx) = alloc_map.get(addr_id) {
                                allocations[alloc_idx].access_points.push(AccessPoint {
                                    block_id,
                                    instruction_index: instr_idx,
                                    kind: AccessKind::Load {
                                        dest: *dest,
                                        address: *address,
                                    },
                                    offset: Some(0),
                                });
                                allocations[alloc_idx].accessed_blocks.insert(block_id);
                            } else if let Some(&(base_alloc, offset)) = gep_bases.get(addr_id) {
                                if let Some(&alloc_idx) = alloc_map.get(&base_alloc) {
                                    allocations[alloc_idx].access_points.push(AccessPoint {
                                        block_id,
                                        instruction_index: instr_idx,
                                        kind: AccessKind::Load {
                                            dest: *dest,
                                            address: *address,
                                        },
                                        offset: Some(offset),
                                    });
                                    allocations[alloc_idx].accessed_blocks.insert(block_id);
                                }
                            }
                        }
                    }
                    InstructionKind::Store { address, value } => {
                        if let Value::Operand(addr_id) = address {
                            // Check if this is a store to a tracked allocation or GEP
                            if let Some(&alloc_idx) = alloc_map.get(addr_id) {
                                allocations[alloc_idx].access_points.push(AccessPoint {
                                    block_id,
                                    instruction_index: instr_idx,
                                    kind: AccessKind::Store {
                                        address: *address,
                                        value: *value,
                                    },
                                    offset: Some(0),
                                });
                                allocations[alloc_idx].accessed_blocks.insert(block_id);
                            } else if let Some(&(base_alloc, offset)) = gep_bases.get(addr_id) {
                                if let Some(&alloc_idx) = alloc_map.get(&base_alloc) {
                                    allocations[alloc_idx].access_points.push(AccessPoint {
                                        block_id,
                                        instruction_index: instr_idx,
                                        kind: AccessKind::Store {
                                            address: *address,
                                            value: *value,
                                        },
                                        offset: Some(offset),
                                    });
                                    allocations[alloc_idx].accessed_blocks.insert(block_id);
                                }
                            }
                        }

                        // Check if the value being stored is an allocation address
                        if let Value::Operand(val_id) = value {
                            if let Some(&alloc_idx) = alloc_map.get(val_id) {
                                // Allocation address is being stored - it escapes
                                allocations[alloc_idx].escapes = true;
                            }
                        }
                    }
                    InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                        // Check if any allocation addresses are passed to calls
                        for arg in args {
                            if let Value::Operand(arg_id) = arg {
                                if let Some(&alloc_idx) = alloc_map.get(arg_id) {
                                    allocations[alloc_idx].escapes = true;
                                } else if let Some(&(base_alloc, _)) = gep_bases.get(arg_id) {
                                    if let Some(&alloc_idx) = alloc_map.get(&base_alloc) {
                                        allocations[alloc_idx].escapes = true;
                                    }
                                }
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
                        if let Some(&alloc_idx) = alloc_map.get(val_id) {
                            allocations[alloc_idx].escapes = true;
                        }
                    }
                }
            }
        }

        // Filter to only non-escaping allocations
        allocations
            .into_iter()
            .filter(|alloc| !alloc.escapes)
            .collect()
    }

    /// Promote non-escaping allocations to registers
    fn promote_allocations(
        &mut self,
        function: &mut MirFunction,
        allocations: Vec<AllocationInfo>,
    ) -> bool {
        if allocations.is_empty() {
            return false;
        }

        let mut changed = false;
        let mut replacements = FxHashMap::default();

        for allocation in allocations {
            if self.promote_single_allocation(function, &allocation, &mut replacements) {
                changed = true;
                self.stats.allocations_promoted += 1;
            }
        }

        // Apply all replacements
        if !replacements.is_empty() {
            self.apply_replacements(function, &replacements);
        }

        changed
    }

    /// Promote a single allocation to registers
    fn promote_single_allocation(
        &mut self,
        function: &mut MirFunction,
        allocation: &AllocationInfo,
        replacements: &mut FxHashMap<ValueId, Value>,
    ) -> bool {
        // For simple cases with single-block access, use direct forwarding
        if allocation.accessed_blocks.len() == 1 {
            return self.promote_single_block_allocation(function, allocation, replacements);
        }

        // For multi-block allocations, we need more sophisticated analysis
        self.promote_multi_block_allocation(function, allocation, replacements)
    }

    /// Promote an allocation that's only accessed in a single block
    fn promote_single_block_allocation(
        &mut self,
        function: &mut MirFunction,
        allocation: &AllocationInfo,
        replacements: &mut FxHashMap<ValueId, Value>,
    ) -> bool {
        let block_id = *allocation.accessed_blocks.iter().next().unwrap();
        let block = function.get_basic_block_mut(block_id).unwrap();

        // Track values stored at each offset
        let mut slot_values: FxHashMap<i32, Value> = FxHashMap::default();
        let mut instructions_to_remove = Vec::new();

        // Process access points in order
        let mut access_points = allocation.access_points.clone();
        access_points.sort_by_key(|ap| ap.instruction_index);

        for access_point in access_points {
            match access_point.kind {
                AccessKind::Store { value, .. } => {
                    if let Some(offset) = access_point.offset {
                        slot_values.insert(offset, value);
                        instructions_to_remove.push(access_point.instruction_index);
                        self.stats.stores_eliminated += 1;
                    }
                }
                AccessKind::Load { dest, .. } => {
                    if let Some(offset) = access_point.offset {
                        if let Some(&value) = slot_values.get(&offset) {
                            replacements.insert(dest, value);
                            instructions_to_remove.push(access_point.instruction_index);
                            self.stats.loads_eliminated += 1;
                        }
                    }
                }
                AccessKind::GetElementPtr { .. } => {
                    // GEPs become no-ops when allocation is promoted
                    instructions_to_remove.push(access_point.instruction_index);
                    self.stats.geps_eliminated += 1;
                }
                _ => {}
            }
        }

        // Mark instructions for removal
        for idx in instructions_to_remove {
            if idx < block.instructions.len() {
                block.instructions[idx].kind = InstructionKind::Debug {
                    message: "mem2reg: removed".to_string(),
                    values: vec![],
                };
            }
        }

        // Mark the allocation itself for removal
        for instruction in block.instructions.iter_mut() {
            if let InstructionKind::StackAlloc { dest, .. } = &instruction.kind {
                if *dest == allocation.alloc_id {
                    instruction.kind = InstructionKind::Debug {
                        message: "mem2reg: removed allocation".to_string(),
                        values: vec![],
                    };
                    break;
                }
            }
        }

        true
    }

    /// Promote an allocation accessed across multiple blocks
    fn promote_multi_block_allocation(
        &mut self,
        function: &mut MirFunction,
        allocation: &AllocationInfo,
        replacements: &mut FxHashMap<ValueId, Value>,
    ) -> bool {
        // For now, use a simplified approach:
        // Track the last stored value for each offset across all blocks
        let mut last_stored: FxHashMap<i32, Value> = FxHashMap::default();
        let mut access_map: FxHashMap<(BasicBlockId, usize), AccessPoint> = FxHashMap::default();

        // Build access map
        for access_point in &allocation.access_points {
            access_map.insert(
                (access_point.block_id, access_point.instruction_index),
                access_point.clone(),
            );
        }

        // Process blocks in a simple forward pass
        // This is not optimal but works for simple cases
        for (block_id, block) in function.basic_blocks.iter_mut().enumerate() {
            let block_id = BasicBlockId::from_usize(block_id);
            let mut instructions_to_remove = Vec::new();

            for (idx, instruction) in block.instructions.iter_mut().enumerate() {
                if let Some(access_point) = access_map.get(&(block_id, idx)) {
                    match &access_point.kind {
                        AccessKind::Store { value, .. } => {
                            if let Some(offset) = access_point.offset {
                                last_stored.insert(offset, *value);
                                instructions_to_remove.push(idx);
                                self.stats.stores_eliminated += 1;
                            }
                        }
                        AccessKind::Load { dest, .. } => {
                            if let Some(offset) = access_point.offset {
                                if let Some(&value) = last_stored.get(&offset) {
                                    replacements.insert(*dest, value);
                                    instructions_to_remove.push(idx);
                                    self.stats.loads_eliminated += 1;
                                }
                            }
                        }
                        AccessKind::GetElementPtr { .. } => {
                            instructions_to_remove.push(idx);
                            self.stats.geps_eliminated += 1;
                        }
                        _ => {}
                    }
                }

                // Remove the allocation itself
                if let InstructionKind::StackAlloc { dest, .. } = &instruction.kind {
                    if *dest == allocation.alloc_id {
                        instructions_to_remove.push(idx);
                    }
                }
            }

            // Mark instructions for removal
            for idx in instructions_to_remove {
                block.instructions[idx].kind = InstructionKind::Debug {
                    message: "mem2reg: removed".to_string(),
                    values: vec![],
                };
            }
        }

        true
    }

    /// Apply value replacements throughout the function
    fn apply_replacements(
        &self,
        function: &mut MirFunction,
        replacements: &FxHashMap<ValueId, Value>,
    ) {
        for block in function.basic_blocks.iter_mut() {
            for instruction in &mut block.instructions {
                instruction.replace_uses(replacements);
            }
            block.terminator.replace_uses(replacements);
        }
    }

    /// Remove instructions marked as dead
    fn cleanup_dead_instructions(&self, function: &mut MirFunction) {
        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                !matches!(
                    &instr.kind,
                    InstructionKind::Debug { message, .. } if message.starts_with("mem2reg:")
                )
            });
        }
    }
}

impl crate::passes::MirPass for Mem2RegPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        self.optimize(function)
    }

    fn name(&self) -> &'static str {
        "Mem2RegPass"
    }
}

/// Extension trait for instructions to replace uses
trait ReplaceUses {
    fn replace_uses(&mut self, replacements: &FxHashMap<ValueId, Value>);
}

impl ReplaceUses for Instruction {
    fn replace_uses(&mut self, replacements: &FxHashMap<ValueId, Value>) {
        match &mut self.kind {
            InstructionKind::Assign { source, .. } => {
                if let Value::Operand(id) = source {
                    if let Some(new_value) = replacements.get(id) {
                        *source = *new_value;
                    }
                }
            }
            InstructionKind::BinaryOp { left, right, .. } => {
                if let Value::Operand(id) = left {
                    if let Some(new_value) = replacements.get(id) {
                        *left = *new_value;
                    }
                }
                if let Value::Operand(id) = right {
                    if let Some(new_value) = replacements.get(id) {
                        *right = *new_value;
                    }
                }
            }
            InstructionKind::UnaryOp { source, .. } => {
                if let Value::Operand(id) = source {
                    if let Some(new_value) = replacements.get(id) {
                        *source = *new_value;
                    }
                }
            }
            InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                for arg in args {
                    if let Value::Operand(id) = arg {
                        if let Some(new_value) = replacements.get(id) {
                            *arg = *new_value;
                        }
                    }
                }
            }
            InstructionKind::Store { value, .. } => {
                if let Value::Operand(id) = value {
                    if let Some(new_value) = replacements.get(id) {
                        *value = *new_value;
                    }
                }
            }
            _ => {}
        }
    }
}

impl ReplaceUses for Terminator {
    fn replace_uses(&mut self, replacements: &FxHashMap<ValueId, Value>) {
        match self {
            Self::If { condition, .. } => {
                if let Value::Operand(id) = condition {
                    if let Some(new_value) = replacements.get(id) {
                        *condition = *new_value;
                    }
                }
            }
            Self::BranchCmp { left, right, .. } => {
                if let Value::Operand(id) = left {
                    if let Some(new_value) = replacements.get(id) {
                        *left = *new_value;
                    }
                }
                if let Value::Operand(id) = right {
                    if let Some(new_value) = replacements.get(id) {
                        *right = *new_value;
                    }
                }
            }
            Self::Return { values } => {
                for value in values {
                    if let Value::Operand(id) = value {
                        if let Some(new_value) = replacements.get(id) {
                            *value = *new_value;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
