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
    aggregates_scalarized: usize,
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
        // Phase 1: Handle alloca splitting
        let alloca_changes = self.process_allocas(function);

        // Phase 2: Handle SSA aggregate scalarization
        let ssa_changes = self.process_ssa_aggregates(function);

        alloca_changes || ssa_changes
    }
}

/// Information about an aggregate allocation candidate
#[derive(Debug, Clone)]
struct AllocaCandidate {
    /// The allocation instruction's destination
    alloc_id: ValueId,
    /// The aggregate type being allocated
    aggregate_type: MirType,
    /// All GetElementPtrTyped instructions derived from this alloca
    /// Maps the GEP result to its constant path
    typed_geps: HashMap<ValueId, FieldPath>,
    /// Whether this allocation escapes (passed to call, address taken, etc.)
    escapes: bool,
    /// Blocks containing uses of this allocation
    use_blocks: HashSet<BasicBlockId>,
}

/// Information about an SSA aggregate value
#[derive(Debug, Clone)]
struct SsaAggregateInfo {
    /// The aggregate value ID
    value_id: ValueId,
    /// The aggregate type
    aggregate_type: MirType,
    /// For BuildStruct/BuildTuple: the source values
    source_values: Vec<(String, Value)>, // field name/index as string, value
    /// All ExtractValue uses with their paths
    extracts: HashMap<ValueId, FieldPath>,
    /// All InsertValue uses
    inserts: Vec<(ValueId, FieldPath, Value)>,
    /// Whether this aggregate escapes (returned, stored, passed to call)
    escapes: bool,
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
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
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
                                    typed_geps: HashMap::new(),
                                    escapes: false,
                                    use_blocks: HashSet::new(),
                                },
                            );
                        }
                    }
                    InstructionKind::GetElementPtrTyped {
                        dest, base, path, ..
                    } => {
                        if let Value::Operand(base_id) = base {
                            // Check if this is a GEP from a candidate allocation
                            if let Some(candidate) = candidates.get_mut(base_id) {
                                candidate.typed_geps.insert(*dest, path.clone());
                                candidate.use_blocks.insert(block_id);
                            }
                            // Also check for chained GEPs
                            for candidate in candidates.values_mut() {
                                if candidate.typed_geps.contains_key(base_id) {
                                    // Combine paths for chained GEPs
                                    if let Some(base_path) = candidate.typed_geps.get(base_id) {
                                        let mut combined_path = base_path.clone();
                                        combined_path.extend(path.clone());
                                        candidate.typed_geps.insert(*dest, combined_path);
                                        candidate.use_blocks.insert(block_id);
                                    }
                                }
                            }
                        }
                    }
                    InstructionKind::GetElementPtr { base, .. } => {
                        // Regular (untyped) GEP - mark allocation as escaping
                        if let Value::Operand(base_id) = base {
                            escaping.insert(*base_id);
                            // Also check chained GEPs
                            for candidate in candidates.values() {
                                if candidate.typed_geps.contains_key(base_id) {
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
                                        if candidate.typed_geps.contains_key(arg_id) {
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
                                if candidate.typed_geps.contains_key(addr_id) {
                                    candidate.use_blocks.insert(block_id);
                                }
                            }
                        }
                    }
                    InstructionKind::Load { address, .. } => {
                        // Record use blocks for loads FROM the allocation
                        if let Value::Operand(addr_id) = address {
                            for candidate in candidates.values_mut() {
                                if candidate.typed_geps.contains_key(addr_id) {
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
        for (gep_id, path) in &candidate.typed_geps {
            if let Some(&field_alloc) = self.resolve_path_to_field(&field_allocations, path) {
                gep_replacements.insert(*gep_id, field_alloc);
            }
        }

        // Rewrite all uses of the original allocation and its GEPs
        self.rewrite_aggregate_uses(function, &candidate, &gep_replacements);

        // Remove the original allocation and typed GEPs
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
                            function.basic_blocks[BasicBlockId::from_raw(0)]
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
                            function.basic_blocks[BasicBlockId::from_raw(0)]
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

                    function.basic_blocks[BasicBlockId::from_raw(0)]
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
        candidate: &AllocaCandidate,
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
                    InstructionKind::GetElementPtrTyped { dest, .. } => {
                        !candidate.typed_geps.contains_key(dest)
                    }
                    _ => true,
                });
        }
    }

    /// Phase 2: Process SSA aggregates (Build*/Extract* patterns)
    fn process_ssa_aggregates(&mut self, function: &mut MirFunction) -> bool {
        // Step 1: Identify SSA aggregate patterns
        let aggregates = self.identify_ssa_aggregates(function);
        if aggregates.is_empty() {
            return false;
        }

        // Step 2: Scalarize eligible aggregates
        let mut any_changed = false;
        for aggregate in aggregates {
            if !aggregate.escapes && !aggregate.inserts.is_empty() {
                // For now, only handle pure Extract patterns (no InsertValue)
                // InsertValue requires more complex rewriting
                continue;
            }

            if !aggregate.escapes {
                self.scalarize_ssa_aggregate(function, aggregate);
                any_changed = true;
            }
        }

        any_changed
    }

    /// Identify SSA aggregates built with Build* instructions
    fn identify_ssa_aggregates(&mut self, function: &MirFunction) -> Vec<SsaAggregateInfo> {
        let mut aggregates = HashMap::new();
        let mut escaping = HashSet::new();

        // First pass: Find Build* instructions
        for (_block_id, block) in function.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::BuildStruct {
                        dest,
                        struct_type,
                        fields,
                    } => {
                        let source_values: Vec<(String, Value)> = fields.clone();
                        aggregates.insert(
                            *dest,
                            SsaAggregateInfo {
                                value_id: *dest,
                                aggregate_type: struct_type.clone(),
                                source_values,
                                extracts: HashMap::new(),
                                inserts: Vec::new(),
                                escapes: false,
                            },
                        );
                    }
                    InstructionKind::BuildTuple {
                        dest,
                        elements,
                        tuple_type,
                    } => {
                        let source_values: Vec<(String, Value)> = elements
                            .iter()
                            .enumerate()
                            .map(|(i, v)| (i.to_string(), *v))
                            .collect();
                        aggregates.insert(
                            *dest,
                            SsaAggregateInfo {
                                value_id: *dest,
                                aggregate_type: tuple_type.clone(),
                                source_values,
                                extracts: HashMap::new(),
                                inserts: Vec::new(),
                                escapes: false,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        // Second pass: Find uses of these aggregates
        for (_block_id, block) in function.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    InstructionKind::ExtractValue {
                        dest,
                        aggregate,
                        path,
                        ..
                    } => {
                        if let Value::Operand(agg_id) = aggregate {
                            if let Some(info) = aggregates.get_mut(agg_id) {
                                info.extracts.insert(*dest, path.clone());
                            }
                        }
                    }
                    InstructionKind::InsertValue {
                        dest,
                        aggregate,
                        value,
                        path,
                        ..
                    } => {
                        if let Value::Operand(agg_id) = aggregate {
                            if let Some(info) = aggregates.get_mut(agg_id) {
                                info.inserts.push((*dest, path.clone(), *value));
                            }
                        }
                    }
                    InstructionKind::Store { value, .. } => {
                        // Storing an aggregate means it escapes
                        if let Value::Operand(val_id) = value {
                            if aggregates.contains_key(val_id) {
                                escaping.insert(*val_id);
                            }
                        }
                    }
                    InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                        // Passing aggregate to call means it escapes
                        for arg in args {
                            if let Value::Operand(arg_id) = arg {
                                if aggregates.contains_key(arg_id) {
                                    escaping.insert(*arg_id);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Check terminator
            if let Terminator::Return { values } = &block.terminator {
                for value in values {
                    if let Value::Operand(val_id) = value {
                        if aggregates.contains_key(val_id) {
                            escaping.insert(*val_id);
                        }
                    }
                }
            }
        }

        // Mark escaping aggregates
        for escaped_id in escaping {
            if let Some(info) = aggregates.get_mut(&escaped_id) {
                info.escapes = true;
            }
        }

        aggregates.into_values().collect()
    }

    /// Scalarize an SSA aggregate by replacing ExtractValue with direct value references
    fn scalarize_ssa_aggregate(&mut self, function: &mut MirFunction, aggregate: SsaAggregateInfo) {
        self.stats.aggregates_scalarized += 1;

        // Create a mapping from ExtractValue results to their source values
        let mut extract_replacements = HashMap::new();
        for (extract_dest, path) in &aggregate.extracts {
            if let Some(source_value) = self.resolve_path_to_source(&aggregate, path) {
                extract_replacements.insert(*extract_dest, source_value);
            }
        }

        // Rewrite all uses of ExtractValue results
        for block in function.basic_blocks.iter_mut() {
            for instruction in &mut block.instructions {
                // Replace uses of extracted values
                match &mut instruction.kind {
                    InstructionKind::Assign { source, .. } => {
                        if let Value::Operand(src_id) = source {
                            if let Some(&replacement) = extract_replacements.get(src_id) {
                                *source = replacement;
                            }
                        }
                    }
                    InstructionKind::BinaryOp { left, right, .. } => {
                        if let Value::Operand(left_id) = left {
                            if let Some(&replacement) = extract_replacements.get(left_id) {
                                *left = replacement;
                            }
                        }
                        if let Value::Operand(right_id) = right {
                            if let Some(&replacement) = extract_replacements.get(right_id) {
                                *right = replacement;
                            }
                        }
                    }
                    InstructionKind::UnaryOp { source, .. } => {
                        if let Value::Operand(src_id) = source {
                            if let Some(&replacement) = extract_replacements.get(src_id) {
                                *source = replacement;
                            }
                        }
                    }
                    InstructionKind::Store { value, .. } => {
                        if let Value::Operand(val_id) = value {
                            if let Some(&replacement) = extract_replacements.get(val_id) {
                                *value = replacement;
                            }
                        }
                    }
                    InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                        for arg in args {
                            if let Value::Operand(arg_id) = arg {
                                if let Some(&replacement) = extract_replacements.get(arg_id) {
                                    *arg = replacement;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Update terminator
            if let Terminator::Return { values } = &mut block.terminator {
                for value in values {
                    if let Value::Operand(val_id) = value {
                        if let Some(&replacement) = extract_replacements.get(val_id) {
                            *value = replacement;
                        }
                    }
                }
            }
        }

        // Remove Build* and ExtractValue instructions
        for block in function.basic_blocks.iter_mut() {
            block
                .instructions
                .retain(|instruction| match &instruction.kind {
                    InstructionKind::BuildStruct { dest, .. }
                    | InstructionKind::BuildTuple { dest, .. } => *dest != aggregate.value_id,
                    InstructionKind::ExtractValue { dest, .. } => {
                        !aggregate.extracts.contains_key(dest)
                    }
                    _ => true,
                });
        }
    }

    /// Resolve a path through an aggregate to find the source value
    fn resolve_path_to_source(
        &self,
        aggregate: &SsaAggregateInfo,
        path: &FieldPath,
    ) -> Option<Value> {
        if path.is_empty() {
            return Some(Value::Operand(aggregate.value_id));
        }

        // For single-level paths, directly look up the source value
        if path.len() == 1 {
            match &path[0] {
                AccessPath::Field(field_name) => aggregate
                    .source_values
                    .iter()
                    .find(|(name, _)| name == field_name)
                    .map(|(_, value)| *value),
                AccessPath::TupleIndex(index) => {
                    aggregate.source_values.get(*index).map(|(_, value)| *value)
                }
            }
        } else {
            // For nested paths, we would need to recursively resolve
            // This requires tracking nested Build* instructions
            // For now, return None for nested paths
            None
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
