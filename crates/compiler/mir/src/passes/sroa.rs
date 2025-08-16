//! # Scalar Replacement of Aggregates (SROA)
//!
//! This pass breaks up aggregate allocations (structs/tuples) into individual scalar allocations
//! for each field when all uses go through constant field paths. This enables mem2reg to promote
//! each field independently to SSA form.
//!
//! ## Transformations
//!
//! 1. **Alloca Splitting**: Transforms aggregate allocations into per-field allocations
//!    ```ignore
//!    %s = framealloc {felt, u32}
//!    %p = getelementptr_typed %s, .field1
//!    store %p, %v
//!    ```
//!    Becomes:
//!    ```ignore
//!    %s_field0 = framealloc felt
//!    %s_field1 = framealloc u32
//!    store %s_field1, %v
//!    ```
//!
//! 2. **SSA Aggregate Scalarization**: Eliminates Build*/Extract* patterns
//!    ```ignore
//!    %t = buildtuple (%a, %b)
//!    %x = extractvalue %t, [0]
//!    ```
//!    Becomes:
//!    ```ignore
//!    %x = %a
//!    ```

use std::collections::{HashMap, HashSet};

use crate::layout::DataLayout;
use crate::{
    AccessPath, BasicBlockId, FieldPath, Instruction, InstructionKind, MirFunction, MirType,
    Terminator, Value, ValueId,
};

/// SROA optimization pass
pub struct SroaPass {
    /// Statistics for reporting
    stats: SroaStats,
}

#[derive(Debug, Default)]
struct SroaStats {
    allocas_analyzed: usize,
    allocas_split: usize,
    loads_eliminated: usize,
    stores_eliminated: usize,
}

impl Default for SroaPass {
    fn default() -> Self {
        Self::new()
    }
}

impl SroaPass {
    /// Create a new SROA pass
    pub fn new() -> Self {
        Self {
            stats: SroaStats::default(),
        }
    }

    /// Run the SROA optimization
    pub fn optimize(&mut self, function: &mut MirFunction) -> bool {
        // Handle alloca splitting - focus exclusively on memory-based aggregate splitting
        self.process_allocas(function)
    }
}

/// Information about an aggregate allocation candidate
#[derive(Debug, Clone)]
struct AllocaCandidate {
    /// The allocation instruction's destination
    alloc_id: ValueId,
    /// The aggregate type being allocated
    aggregate_type: MirType,
    /// All GetElementPtr instructions with constant offsets derived from this alloca
    /// Maps the GEP result to its field path (when offset can be resolved to a field)
    constant_geps: HashMap<ValueId, FieldPath>,
    /// Whether this allocation escapes (passed to call, address taken, etc.)
    escapes: bool,
    /// Blocks containing uses of this allocation
    use_blocks: HashSet<BasicBlockId>,
}

impl SroaPass {
    /// Phase 1: Process and split aggregate allocations
    fn process_allocas(&mut self, function: &mut MirFunction) -> bool {
        // Step 1: Identify candidate allocations
        let candidates = self.identify_alloca_candidates(function);
        if candidates.is_empty() {
            return false;
        }

        // Step 2: Split eligible allocations
        let mut any_changed = false;
        for candidate in candidates {
            if !candidate.escapes {
                self.split_allocation(function, candidate);
                any_changed = true;
            }
        }

        any_changed
    }

    /// Identify aggregate allocations that are candidates for splitting
    fn identify_alloca_candidates(&mut self, function: &MirFunction) -> Vec<AllocaCandidate> {
        let mut candidates = HashMap::new();
        let mut escaping = HashSet::new();

        // First pass: Find all aggregate allocations and typed GEPs
        for (_block_id, block) in function.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::FrameAlloc { dest, ty } => {
                        // Only consider struct and tuple types
                        if matches!(ty, MirType::Struct { .. } | MirType::Tuple(_)) {
                            self.stats.allocas_analyzed += 1;
                            candidates.insert(
                                *dest,
                                AllocaCandidate {
                                    alloc_id: *dest,
                                    aggregate_type: ty.clone(),
                                    constant_geps: HashMap::new(),
                                    escapes: false,
                                    use_blocks: HashSet::new(),
                                },
                            );
                        }
                    }
                    // Note: GetElementPtrTyped has been removed. We now analyze regular GetElementPtr
                    // instructions with constant offsets to determine field access patterns.
                    InstructionKind::GetElementPtr { base, .. } => {
                        // Regular (untyped) GEP - mark allocation as escaping
                        if let Value::Operand(base_id) = base {
                            escaping.insert(*base_id);
                            // Also check chained GEPs
                            for candidate in candidates.values() {
                                if candidate.constant_geps.contains_key(base_id) {
                                    escaping.insert(candidate.alloc_id);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Second pass: Check for escaping uses
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                        // Any allocation or GEP passed to a call escapes
                        for arg in args {
                            if let Value::Operand(arg_id) = arg {
                                if candidates.contains_key(arg_id) {
                                    escaping.insert(*arg_id);
                                } else {
                                    // Check if it's a GEP from an allocation
                                    for candidate in candidates.values() {
                                        if candidate.constant_geps.contains_key(arg_id) {
                                            escaping.insert(candidate.alloc_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    InstructionKind::AddressOf { operand, .. } => {
                        // Taking address of allocation means it escapes
                        if candidates.contains_key(operand) {
                            escaping.insert(*operand);
                        }
                    }
                    InstructionKind::Store { address, value, .. } => {
                        // Check if storing the allocation itself (not TO it)
                        if let Value::Operand(val_id) = value {
                            if candidates.contains_key(val_id) {
                                escaping.insert(*val_id);
                            }
                        }
                        // Record use blocks for stores TO the allocation
                        if let Value::Operand(addr_id) = address {
                            for candidate in candidates.values_mut() {
                                if candidate.constant_geps.contains_key(addr_id) {
                                    candidate.use_blocks.insert(block_id);
                                }
                            }
                        }
                    }
                    InstructionKind::Load { address, .. } => {
                        // Record use blocks for loads FROM the allocation
                        if let Value::Operand(addr_id) = address {
                            for candidate in candidates.values_mut() {
                                if candidate.constant_geps.contains_key(addr_id) {
                                    candidate.use_blocks.insert(block_id);
                                }
                            }
                        }
                    }
                    InstructionKind::Assign { source, .. } => {
                        // Check if assigning the allocation address itself
                        if let Value::Operand(src_id) = source {
                            if candidates.contains_key(src_id) {
                                escaping.insert(*src_id);
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
                        if candidates.contains_key(val_id) {
                            escaping.insert(*val_id);
                        }
                    }
                }
            }
        }

        // Mark escaping allocations
        for escaped_id in escaping {
            if let Some(candidate) = candidates.get_mut(&escaped_id) {
                candidate.escapes = true;
            }
        }

        candidates.into_values().collect()
    }

    /// Split an aggregate allocation into per-field allocations
    fn split_allocation(&mut self, function: &mut MirFunction, candidate: AllocaCandidate) {
        self.stats.allocas_split += 1;

        // Generate scalar allocations for each field
        let field_allocations = self.create_field_allocations(function, &candidate);

        // Create a mapping from GEP results to their corresponding field allocations
        let mut gep_replacements: HashMap<ValueId, ValueId> = HashMap::new();
        for (gep_id, path) in &candidate.constant_geps {
            if let Some(&field_alloc) = self.resolve_path_to_field(&field_allocations, path) {
                gep_replacements.insert(*gep_id, field_alloc);
            }
        }

        // Rewrite all uses of the original allocation and its GEPs
        self.rewrite_aggregate_uses(function, &candidate, &gep_replacements);

        // Remove the original allocation and constant GEPs
        self.remove_original_instructions(function, &candidate);
    }

    /// Create individual allocations for each field of an aggregate
    fn create_field_allocations(
        &self,
        function: &mut MirFunction,
        candidate: &AllocaCandidate,
    ) -> Vec<(FieldPath, ValueId)> {
        let mut field_allocations = Vec::new();
        let _layout = DataLayout::new();

        // Helper to recursively create allocations for nested aggregates
        fn create_allocations_recursive(
            function: &mut MirFunction,
            ty: &MirType,
            current_path: &mut FieldPath,
            allocations: &mut Vec<(FieldPath, ValueId)>,
        ) {
            match ty {
                MirType::Struct { fields, .. } => {
                    for (field_name, field_type) in fields {
                        current_path.push(AccessPath::Field(field_name.clone()));

                        if matches!(field_type, MirType::Struct { .. } | MirType::Tuple(_)) {
                            // Recursively handle nested aggregates
                            create_allocations_recursive(
                                function,
                                field_type,
                                current_path,
                                allocations,
                            );
                        } else {
                            // Create allocation for scalar field
                            let field_alloc_id = function.new_typed_value_id(field_type.clone());
                            let field_alloc =
                                Instruction::frame_alloc(field_alloc_id, field_type.clone());

                            // Insert at the beginning of the entry block
                            function.basic_blocks[function.entry_block]
                                .instructions
                                .insert(0, field_alloc);

                            allocations.push((current_path.clone(), field_alloc_id));
                        }

                        current_path.pop();
                    }
                }
                MirType::Tuple(elements) => {
                    for (index, elem_type) in elements.iter().enumerate() {
                        current_path.push(AccessPath::TupleIndex(index));

                        if matches!(elem_type, MirType::Struct { .. } | MirType::Tuple(_)) {
                            // Recursively handle nested aggregates
                            create_allocations_recursive(
                                function,
                                elem_type,
                                current_path,
                                allocations,
                            );
                        } else {
                            // Create allocation for scalar element
                            let elem_alloc_id = function.new_typed_value_id(elem_type.clone());
                            let elem_alloc =
                                Instruction::frame_alloc(elem_alloc_id, elem_type.clone());

                            // Insert at the beginning of the entry block
                            function.basic_blocks[function.entry_block]
                                .instructions
                                .insert(0, elem_alloc);

                            allocations.push((current_path.clone(), elem_alloc_id));
                        }

                        current_path.pop();
                    }
                }
                _ => {
                    // Non-aggregate type - create single allocation
                    let alloc_id = function.new_typed_value_id(ty.clone());
                    let alloc = Instruction::frame_alloc(alloc_id, ty.clone());

                    function.basic_blocks[function.entry_block]
                        .instructions
                        .insert(0, alloc);

                    allocations.push((current_path.clone(), alloc_id));
                }
            }
        }

        let mut current_path = Vec::new();
        create_allocations_recursive(
            function,
            &candidate.aggregate_type,
            &mut current_path,
            &mut field_allocations,
        );

        field_allocations
    }

    /// Resolve a field path to its corresponding scalar allocation
    fn resolve_path_to_field<'a>(
        &self,
        field_allocations: &'a [(FieldPath, ValueId)],
        path: &FieldPath,
    ) -> Option<&'a ValueId> {
        field_allocations
            .iter()
            .find(|(alloc_path, _)| alloc_path == path)
            .map(|(_, id)| id)
    }

    /// Rewrite all uses of the aggregate allocation to use scalar allocations
    fn rewrite_aggregate_uses(
        &mut self,
        function: &mut MirFunction,
        _candidate: &AllocaCandidate,
        gep_replacements: &HashMap<ValueId, ValueId>,
    ) {
        for block in function.basic_blocks.iter_mut() {
            for instruction in &mut block.instructions {
                match &mut instruction.kind {
                    InstructionKind::Load { address, .. } => {
                        if let Value::Operand(addr_id) = address {
                            if let Some(&replacement) = gep_replacements.get(addr_id) {
                                *address = Value::Operand(replacement);
                                self.stats.loads_eliminated += 1;
                            }
                        }
                    }
                    InstructionKind::Store { address, .. } => {
                        if let Value::Operand(addr_id) = address {
                            if let Some(&replacement) = gep_replacements.get(addr_id) {
                                *address = Value::Operand(replacement);
                                self.stats.stores_eliminated += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Remove the original aggregate allocation and typed GEP instructions
    fn remove_original_instructions(
        &self,
        function: &mut MirFunction,
        candidate: &AllocaCandidate,
    ) {
        for block in function.basic_blocks.iter_mut() {
            block
                .instructions
                .retain(|instruction| match &instruction.kind {
                    InstructionKind::FrameAlloc { dest, .. } => *dest != candidate.alloc_id,
                    // Remove constant GEPs that were replaced with field allocations
                    InstructionKind::GetElementPtr { dest, .. } => {
                        !candidate.constant_geps.contains_key(dest)
                    }
                    _ => true,
                });
        }
    }
}

impl crate::passes::MirPass for SroaPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        self.optimize(function)
    }

    fn name(&self) -> &'static str {
        "SroaPass"
    }
}

#[cfg(test)]
#[path = "./sroa_tests.rs"]
mod tests;
