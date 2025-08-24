//! # Scalar Replacement of Aggregates (SROA) Pass
//!
//! This optimization pass decomposes aggregate types (structs and tuples) into their
//! constituent scalar fields, eliminating unnecessary aggregate construction and
//! extraction operations. This enables better downstream optimizations by exposing
//! more opportunities for constant folding, copy propagation, and dead code elimination.
//!
//! ## Algorithm Overview
//!
//! The SROA pass operates in a single forward pass through each basic block, using
//! a recursive forward-looking analysis to determine which aggregates can be safely
//! scalarized.
//!
//! ### Scalarization Rules
//!
//! An aggregate (struct or tuple) can be scalarized if ALL of the following conditions are met:
//!
//! 1. **Type is scalarizable**: The aggregate type must be eligible for scalarization
//!    (currently limited by size constraints, default max 8 fields)
//!
//! 2. **Not used across blocks**: The aggregate must not be used in different basic blocks
//!    (phase 1 limitation to avoid complex phi node handling)
//!
//! 3. **Valid field initialization**: Must be able to create an `AggState` from the
//!    aggregate's fields (all fields must be properly initialized)
//!
//! 4. **Recursive parent check**: If the aggregate is used as a field in another
//!    aggregate OR passed as an argument to a function call, it can only be scalarized
//!    if the parent aggregate can also be scalarized
//!
//! ### Arrays (FixedArray)
//!
//! - A `FixedArray` is treated like a tuple for SROA only if all array indices
//!   involved are compile-time constants for the array and its SSA family.
//!   - No `ArrayIndex`/`ArrayInsert` with non-constant `index` in the family
//!
//! If any non-constant indexing exists in the SSA family (the original array and all
//! values derived from it via `Assign` and `ArrayInsert`), the array is not scalarized.
//! Fixed-index `ArrayIndex`/`ArrayInsert` remain eligible for forwarding like tuples.
//!
//! ### Implementation Strategy
//!
//! 1. **Analysis Phase**:
//!    - Identify aggregates used across block boundaries
//!    - Build instruction list for forward-looking analysis
//!
//! 2. **Transformation Phase** (per instruction):
//!    - **MakeStruct/MakeTuple/MakeFixedArray**: Check scalarization conditions recursively
//!      - If scalarizable: capture field values in `AggState`, drop instruction
//!      - If not: keep instruction unchanged
//!
//!    - **ExtractField/ExtractTuple/ArrayIndex (const)**: Replace with direct field access
//!      - Rewrite to simple assignment of the tracked scalar value
//!
//!    - **InsertField/InsertTuple/ArrayInsert (const)**: Forward partial updates
//!      - Create new `AggState` with updated field/element
//!
//!    - **Assign**: Propagate aggregate states between values
//!
//!    - **Call/Store/Return**: Materialize aggregates as needed
//!      - Reconstruct full aggregates from scalar fields at ABI boundaries
//!
//! ### Recursive Forward-Looking Analysis
//!
//! The key innovation is the recursive check for nested aggregate dependencies:
//! ```text
//! struct Point { x, y }
//! struct Line { start: Point, end: Point }
//!
//! %0 = MakeStruct Point { x: 1, y: 2 }  // Can this be scalarized?
//! %1 = MakeStruct Line { start: %0, ... }  // Depends on whether Line can be scalarized
//! %2 = Call foo(%1)  // Line cannot be scalarized (used in call)
//! // Therefore, Point %0 also cannot be scalarized
//! ```
//!
//! ## Limitations (Phase 1)
//!
//! - No scalarization across basic blocks (requires phi node handling)
//! - Arrays are not scalarized if any dynamic indexing occurs
//! - Recursive aggregates not supported
//! - Maximum aggregate size limit (configurable, default 8 fields)

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    instruction::InstructionKind, value::Value, BasicBlockId, Instruction, MirFunction, MirType,
    ValueId,
};

use super::MirPass;

/// Phase-1 SROA: tuples & structs, optional arrays (no dynamic array indexing), no aggregate PHIs.
///
/// Strategy:
///  - Track aggregates built by MakeTuple/MakeStruct/MakeFixedArray (and copies via Assign)
///  - Model partial updates (InsertTuple/InsertField/InsertArrayElement) as per-component SSA
///  - Rewrite Extract* → Assign of the scalar value
///  - At uses that REQUIRE a true aggregate (call param typed as aggregate, Store with aggregate ty),
///    materialize right before the use from the latest per-field values
///  - Keep PHIs unchanged in v1 (skip blocks that would need per-field PHIs)
#[derive(Debug, Default)]
pub struct ScalarReplacementOfAggregates {
    stats: Stats,
    config: Config,
}

#[derive(Debug, Default)]
struct Stats {
    scalarized_builds: usize,
    extracts_rewritten: usize,
    inserts_forwarded: usize,
    assigns_forwarded: usize,
    materializations: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub enable_tuples: bool,
    pub enable_structs: bool,
    pub max_aggregate_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_tuples: true,
            enable_structs: true,
            max_aggregate_size: 8, // Conservative default
        }
    }
}

impl ScalarReplacementOfAggregates {
    pub const fn new() -> Self {
        Self {
            stats: Stats {
                scalarized_builds: 0,
                extracts_rewritten: 0,
                inserts_forwarded: 0,
                assigns_forwarded: 0,
                materializations: 0,
            },
            config: Config {
                enable_tuples: true,
                enable_structs: true,
                max_aggregate_size: 8,
            },
        }
    }

    pub const fn with_config(config: Config) -> Self {
        Self {
            stats: Stats {
                scalarized_builds: 0,
                extracts_rewritten: 0,
                inserts_forwarded: 0,
                assigns_forwarded: 0,
                materializations: 0,
            },
            config,
        }
    }

    /// Conservative configuration for testing
    pub const fn conservative() -> Self {
        Self::with_config(Config {
            enable_tuples: true,
            enable_structs: true,
            max_aggregate_size: 4,
        })
    }

    /// Check if a type is eligible for scalarization
    const fn is_scalarizable(&self, ty: &MirType) -> bool {
        match ty {
            MirType::Tuple(elems) if self.config.enable_tuples => {
                elems.len() <= self.config.max_aggregate_size
            }
            MirType::Struct { fields, .. } if self.config.enable_structs => {
                fields.len() <= self.config.max_aggregate_size
            }
            MirType::FixedArray { size, .. } if self.config.enable_tuples => {
                // Treat arrays like tuples for SROA purposes
                *size <= self.config.max_aggregate_size
            }
            _ => false,
        }
    }

    /// Check if an aggregate can be scalarized using recursive forward-looking analysis.
    ///
    /// This implements the core scalarization decision logic:
    /// 1. Type must be scalarizable
    /// 2. Not used across blocks
    /// 3. Valid field initialization (can create AggState)
    /// 4. Recursive check for parent aggregates and function calls
    fn can_scalarize_aggregate(
        &self,
        dest: &ValueId,
        instructions: &[Instruction],
        function: &MirFunction,
        cross_block_aggregates: &FxHashSet<ValueId>,
        visited: &mut FxHashSet<ValueId>,
    ) -> bool {
        // Avoid infinite recursion on cyclic dependencies
        if visited.contains(dest) {
            return false;
        }
        visited.insert(*dest);

        // Find the MakeStruct/MakeTuple/MakeFixedArray instruction for this dest
        let Some(make_inst) = instructions.iter().find(|inst| {
            match &inst.kind {
                InstructionKind::MakeStruct { dest: d, .. } => d == dest,
                InstructionKind::MakeTuple { dest: d, .. } => d == dest,
                InstructionKind::MakeFixedArray { dest: d, .. } => d == dest,
                _ => false,
            }
        }) else {
            return false;
        };

        // Check basic conditions based on instruction type
        match &make_inst.kind {
            InstructionKind::MakeStruct {
                fields, struct_ty, ..
            } => {
                // Condition 1: Type must be scalarizable
                if !self.is_scalarizable(struct_ty) {
                    return false;
                }

                // Condition 2: Not used across blocks
                if cross_block_aggregates.contains(dest) {
                    return false;
                }

                // Condition 3: Can create AggState
                let Some(MirType::Struct { fields: ty_fields, .. }) = function
                    .get_value_type(*dest)
                    .or(Some(struct_ty))
                    .filter(|t| matches!(t, MirType::Struct { .. }))
                else {
                    return false;
                };

                if AggState::from_struct_fields(fields, ty_fields).is_none() {
                    return false;
                }
            }
            InstructionKind::MakeTuple { .. } => {
                // Get tuple type
                let Some(ty @ MirType::Tuple(_)) = function.get_value_type(*dest) else {
                    return false;
                };

                // Check basic conditions
                if !self.is_scalarizable(ty) || cross_block_aggregates.contains(dest) {
                    return false;
                }
            }
            InstructionKind::MakeFixedArray { .. } => {
                // Get array type
                let Some(ty @ MirType::FixedArray { .. }) = function.get_value_type(*dest) else {
                    return false;
                };

                // Check basic conditions (arrays behave like tuples for SROA)
                if !self.is_scalarizable(ty) || cross_block_aggregates.contains(dest) {
                    return false;
                }
            }
            _ => return false,
        }

        // Condition 4: Check recursive dependencies (calls and parent aggregates)
        for inst in instructions {
            match &inst.kind {
                // Check if used in a function call - cannot scalarize
                InstructionKind::Call { args, .. } => {
                    if args
                        .iter()
                        .any(|arg| arg.is_operand() && arg.as_operand() == Some(*dest))
                    {
                        return false;
                    }
                }
                // Check if used as a field in another struct
                InstructionKind::MakeStruct {
                    dest: parent_dest,
                    fields: parent_fields,
                    ..
                } => {
                    // Skip self-reference
                    if parent_dest == dest {
                        continue;
                    }

                    for (_, field_val) in parent_fields {
                        if let Value::Operand(field_id) = field_val {
                            if field_id == dest {
                                // Our aggregate is used in parent - recursively check if parent can be scalarized
                                if !self.can_scalarize_aggregate(
                                    parent_dest,
                                    instructions,
                                    function,
                                    cross_block_aggregates,
                                    visited,
                                ) {
                                    return false;
                                }
                            }
                        }
                    }
                }
                // Check if used as an element in a tuple
                InstructionKind::MakeTuple {
                    dest: parent_dest,
                    elements,
                    ..
                } => {
                    // Skip self-reference
                    if parent_dest == dest {
                        continue;
                    }

                    for elem in elements {
                        if elem.is_operand() && elem.as_operand() == Some(*dest) {
                            // Our aggregate is used in parent tuple - recursively check
                            if !self.can_scalarize_aggregate(
                                parent_dest,
                                instructions,
                                function,
                                cross_block_aggregates,
                                visited,
                            ) {
                                return false;
                            }
                        }
                    }
                }
                // Check if used in array indexing with non-constant index - cannot scalarize
                InstructionKind::ArrayIndex { array, index, .. } => {
                    if array.is_operand()
                        && array.as_operand() == Some(*dest)
                        && !matches!(index, Value::Literal(crate::value::Literal::Integer(_)))
                    {
                        return false;
                    }
                }
                // Check if used in array insert with non-constant index - cannot scalarize
                InstructionKind::ArrayInsert {
                    array_val, index, ..
                } => {
                    if array_val.is_operand()
                        && array_val.as_operand() == Some(*dest)
                        && !matches!(index, Value::Literal(crate::value::Literal::Integer(_)))
                    {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// Build the SSA family of an aggregate value.
    ///
    /// A family consists of the root value and all values derived from it
    /// via Assign and Insert* operations within the same instruction sequence.
    fn build_aggregate_family(
        &self,
        root: ValueId,
        instructions: &[Instruction],
        function: &MirFunction,
    ) -> FxHashSet<ValueId> {
        let mut family = FxHashSet::default();
        family.insert(root);

        // Determine the aggregate type to match the right operations
        let root_ty = function.get_value_type(root);

        // Iteratively expand the family
        let mut changed = true;
        while changed {
            changed = false;
            for inst in instructions {
                match &inst.kind {
                    // Assign propagates the family membership
                    InstructionKind::Assign { dest, source, ty } => {
                        // Check if types match (same aggregate type)
                        if let (Some(root_ty), Value::Operand(src_id)) = (root_ty.as_ref(), source)
                        {
                            let types_match = matches!(
                                (root_ty, ty),
                                (MirType::FixedArray { .. }, MirType::FixedArray { .. })
                                    | (MirType::Tuple(_), MirType::Tuple(_))
                                    | (MirType::Struct { .. }, MirType::Struct { .. })
                            );

                            if types_match && family.contains(src_id) && !family.contains(dest) {
                                family.insert(*dest);
                                changed = true;
                            }
                        }
                    }
                    // Insert operations create new family members
                    InstructionKind::ArrayInsert {
                        dest,
                        array_val: Value::Operand(src_id),
                        ..
                    } => {
                        if family.contains(src_id) && !family.contains(dest) {
                            family.insert(*dest);
                            changed = true;
                        }
                    }
                    InstructionKind::InsertTuple {
                        dest,
                        tuple_val: Value::Operand(src_id),
                        ..
                    } => {
                        if family.contains(src_id) && !family.contains(dest) {
                            family.insert(*dest);
                            changed = true;
                        }
                    }
                    InstructionKind::InsertField {
                        dest,
                        struct_val: Value::Operand(src_id),
                        ..
                    } => {
                        if family.contains(src_id) && !family.contains(dest) {
                            family.insert(*dest);
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        family
    }

    /// Check if any value in an array's SSA family has non-constant index usage.
    fn array_family_has_dynamic_index_use(
        &self,
        root: ValueId,
        instructions: &[Instruction],
        function: &MirFunction,
    ) -> bool {
        let family = self.build_aggregate_family(root, instructions, function);

        // Check for non-constant indexing on any member of the family
        for inst in instructions {
            match &inst.kind {
                InstructionKind::ArrayIndex {
                    array: Value::Operand(id),
                    index,
                    ..
                } => {
                    if family.contains(id)
                        && !matches!(index, Value::Literal(crate::value::Literal::Integer(_)))
                    {
                        return true;
                    }
                }
                InstructionKind::ArrayInsert {
                    array_val: Value::Operand(id),
                    index,
                    ..
                } => {
                    if family.contains(id)
                        && !matches!(index, Value::Literal(crate::value::Literal::Integer(_)))
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Find aggregates that are used across block boundaries.
    /// In phase 1, we skip scalarizing these to maintain correctness.
    fn find_cross_block_aggregates(&self, function: &MirFunction) -> FxHashSet<ValueId> {
        let mut cross_block = FxHashSet::default();

        // Map each aggregate value to its root and defining block
        // root -> (block_id, family_members)
        let mut root_definitions: FxHashMap<ValueId, (BasicBlockId, FxHashSet<ValueId>)> =
            FxHashMap::default();

        // First pass: build families for each block
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            // Find all root aggregates (Make* instructions) in this block
            let mut block_roots = Vec::new();
            for inst in &block.instructions {
                match &inst.kind {
                    InstructionKind::MakeTuple { dest, .. }
                    | InstructionKind::MakeStruct { dest, .. }
                    | InstructionKind::MakeFixedArray { dest, .. } => {
                        if let Some(ty) = function.get_value_type(*dest) {
                            if self.is_scalarizable(ty) {
                                block_roots.push(*dest);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Build families for each root in this block
            let block_instructions: Vec<_> = block.instructions.clone();
            for root in block_roots {
                let family = self.build_aggregate_family(root, &block_instructions, function);
                root_definitions.insert(root, (block_id, family));
            }
        }

        // Second pass: check if any family member is used in a different block
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            // Check all value uses in instructions
            for inst in &block.instructions {
                self.collect_value_uses_in_instruction(&inst.kind, |used_value| {
                    // Find if this value belongs to any family
                    for (def_block, family) in root_definitions.values() {
                        if family.contains(&used_value) && *def_block != block_id {
                            // This aggregate family is used across blocks
                            // Mark all family members as cross-block
                            for member in family {
                                cross_block.insert(*member);
                            }
                        }
                    }
                });
            }

            // Check terminator uses
            self.collect_value_uses_in_terminator(&block.terminator, |used_value| {
                // Find if this value belongs to any family
                for (def_block, family) in root_definitions.values() {
                    if family.contains(&used_value) && *def_block != block_id {
                        // This aggregate family is used across blocks
                        // Mark all family members as cross-block
                        for member in family {
                            cross_block.insert(*member);
                        }
                    }
                }
            });
        }

        cross_block
    }

    /// Helper to collect all value uses in an instruction
    #[allow(clippy::cognitive_complexity)]
    fn collect_value_uses_in_instruction<F>(&self, inst: &InstructionKind, mut callback: F)
    where
        F: FnMut(ValueId),
    {
        match inst {
            InstructionKind::Assign { source, .. } => {
                if let Value::Operand(id) = source {
                    callback(*id);
                }
            }
            InstructionKind::UnaryOp { source, .. } => {
                if let Value::Operand(id) = source {
                    callback(*id);
                }
            }
            InstructionKind::BinaryOp { left, right, .. } => {
                if let Value::Operand(id) = left {
                    callback(*id);
                }
                if let Value::Operand(id) = right {
                    callback(*id);
                }
            }
            InstructionKind::Call { args, .. } => {
                for arg in args {
                    if let Value::Operand(id) = arg {
                        callback(*id);
                    }
                }
            }
            InstructionKind::Load { address, .. } => {
                if let Value::Operand(id) = address {
                    callback(*id);
                }
            }
            InstructionKind::Store { address, value, .. } => {
                if let Value::Operand(id) = address {
                    callback(*id);
                }
                if let Value::Operand(id) = value {
                    callback(*id);
                }
            }
            InstructionKind::ExtractTupleElement { tuple, .. } => {
                if let Value::Operand(id) = tuple {
                    callback(*id);
                }
            }
            InstructionKind::ExtractStructField { struct_val, .. } => {
                if let Value::Operand(id) = struct_val {
                    callback(*id);
                }
            }
            InstructionKind::InsertTuple {
                tuple_val,
                new_value,
                ..
            } => {
                if let Value::Operand(id) = tuple_val {
                    callback(*id);
                }
                if let Value::Operand(id) = new_value {
                    callback(*id);
                }
            }
            InstructionKind::InsertField {
                struct_val,
                new_value,
                ..
            } => {
                if let Value::Operand(id) = struct_val {
                    callback(*id);
                }
                if let Value::Operand(id) = new_value {
                    callback(*id);
                }
            }
            InstructionKind::MakeTuple { elements, .. } => {
                for elem in elements {
                    if let Value::Operand(id) = elem {
                        callback(*id);
                    }
                }
            }
            InstructionKind::MakeStruct { fields, .. } => {
                for (_, val) in fields {
                    if let Value::Operand(id) = val {
                        callback(*id);
                    }
                }
            }
            InstructionKind::Phi { sources, .. } => {
                for (_, val) in sources {
                    if let Value::Operand(id) = val {
                        callback(*id);
                    }
                }
            }
            InstructionKind::AddressOf { operand, .. } => {
                callback(*operand);
            }
            InstructionKind::FrameAlloc { .. } => {}
            InstructionKind::GetElementPtr { base, offset, .. } => {
                if let Value::Operand(id) = base {
                    callback(*id);
                }
                if let Value::Operand(id) = offset {
                    callback(*id);
                }
            }
            InstructionKind::Cast { source, .. } => {
                if let Value::Operand(id) = source {
                    callback(*id);
                }
            }
            InstructionKind::Debug { .. } => {}
            InstructionKind::Nop => {}
            // Array operations
            InstructionKind::MakeFixedArray { elements, .. } => {
                for elem in elements {
                    if let Value::Operand(id) = elem {
                        callback(*id);
                    }
                }
            }
            InstructionKind::ArrayIndex { array, index, .. } => {
                if let Value::Operand(id) = array {
                    callback(*id);
                }
                if let Value::Operand(id) = index {
                    callback(*id);
                }
            }
            InstructionKind::ArrayInsert {
                array_val,
                index,
                new_value,
                ..
            } => {
                if let Value::Operand(id) = array_val {
                    callback(*id);
                }
                if let Value::Operand(id) = index {
                    callback(*id);
                }
                if let Value::Operand(id) = new_value {
                    callback(*id);
                }
            }
        }
    }

    /// Helper to collect all value uses in a terminator
    fn collect_value_uses_in_terminator<F>(&self, term: &crate::Terminator, mut callback: F)
    where
        F: FnMut(ValueId),
    {
        use crate::Terminator;
        match term {
            Terminator::Return { values } => {
                for val in values {
                    if let Value::Operand(id) = val {
                        callback(*id);
                    }
                }
            }
            Terminator::Jump { .. } => {}
            Terminator::If { condition, .. } => {
                if let Value::Operand(id) = condition {
                    callback(*id);
                }
            }
            Terminator::BranchCmp { left, right, .. } => {
                if let Value::Operand(id) = left {
                    callback(*id);
                }
                if let Value::Operand(id) = right {
                    callback(*id);
                }
            }
            Terminator::Unreachable => {}
        }
    }
}

impl MirPass for ScalarReplacementOfAggregates {
    #[allow(clippy::cognitive_complexity)]
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified_any = false;

        // Phase 1: Analyze which aggregates are used across block boundaries
        let cross_block_aggregates = self.find_cross_block_aggregates(function);

        // Process blocks one by one; preserve phi prefix ordering
        let block_count = function.block_count();
        for raw in 0..block_count {
            let bb = BasicBlockId::from_raw(raw);
            let Some(block) = function.get_basic_block(bb).cloned() else {
                continue;
            };

            // v1: if block has aggregate PHIs, skip to keep invariants simple
            let has_aggregate_phi = block.phi_instructions().any(|inst| {
                if let InstructionKind::Phi { dest, .. } = &inst.kind {
                    if let Some(ty) = function.get_value_type(*dest) {
                        matches!(ty, MirType::Tuple(_) | MirType::Struct { .. })
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            if has_aggregate_phi {
                // Skip this block in Phase 1
                continue;
            }

            let phi_end = block.phi_range().end;
            let mut new_instrs: Vec<Instruction> = Vec::with_capacity(block.instructions.len());

            // Copy phi prefix untouched
            for i in 0..phi_end {
                new_instrs.push(block.instructions[i].clone());
            }

            // Map: aggregate ValueId → AggState
            let mut agg_states: FxHashMap<ValueId, AggState> = FxHashMap::default();

            let mut block_modified = false;

            // Collect all instructions for forward-looking
            let all_instructions: Vec<_> = block.non_phi_instructions().cloned().collect();

            // Walk non-phi instructions
            for inst in block.non_phi_instructions().cloned() {
                match &inst.kind {
                    // 1) Build aggregate → capture state, drop instruction
                    InstructionKind::MakeTuple { dest, elements } if self.config.enable_tuples => {
                        let Some(ty @ MirType::Tuple(_)) = function.get_value_type(*dest) else {
                            new_instrs.push(inst);
                            continue;
                        };

                        if !self.is_scalarizable(ty) {
                            new_instrs.push(inst);
                            continue;
                        }

                        // Skip scalarization if used across blocks in phase 1
                        if cross_block_aggregates.contains(dest) {
                            new_instrs.push(inst);
                            continue;
                        }

                        // For tuples, we scalarize even if used in calls (will materialize later)
                        // Only check for nested aggregate dependencies
                        let mut can_scalarize = true;
                        for future_inst in &all_instructions {
                            if let InstructionKind::MakeTuple {
                                dest: parent_dest,
                                elements: parent_elems,
                                ..
                            } = &future_inst.kind
                            {
                                if parent_dest != dest
                                    && parent_elems
                                        .iter()
                                        .any(|e| e.is_operand() && e.as_operand() == Some(*dest))
                                {
                                    // Check if parent tuple can be scalarized
                                    let mut visited = FxHashSet::default();
                                    if !self.can_scalarize_aggregate(
                                        parent_dest,
                                        &all_instructions,
                                        function,
                                        &cross_block_aggregates,
                                        &mut visited,
                                    ) {
                                        can_scalarize = false;
                                        break;
                                    }
                                }
                            }
                        }

                        if !can_scalarize {
                            new_instrs.push(inst);
                            continue;
                        }

                        agg_states.insert(*dest, AggState::tuple(elements.clone()));
                        self.stats.scalarized_builds += 1;
                        block_modified = true;
                    }

                    InstructionKind::MakeStruct {
                        dest,
                        fields,
                        struct_ty,
                    } if self.config.enable_structs => {
                        // Check all conditions including recursive forward-looking
                        let mut visited = FxHashSet::default();
                        let can_scalarize = self.can_scalarize_aggregate(
                            dest,
                            &all_instructions,
                            function,
                            &cross_block_aggregates,
                            &mut visited,
                        );

                        if !can_scalarize {
                            new_instrs.push(inst);
                            continue;
                        }

                        // If we can scalarize, get the type info and create AggState
                        let Some(MirType::Struct {
                            fields: ty_fields, ..
                        }) = function
                            .get_value_type(*dest)
                            .or(Some(struct_ty))
                            .filter(|t| matches!(t, MirType::Struct { .. }))
                        else {
                            new_instrs.push(inst);
                            continue;
                        };

                        if let Some(state) = AggState::from_struct_fields(fields, ty_fields) {
                            agg_states.insert(*dest, state);
                            self.stats.scalarized_builds += 1;
                            block_modified = true;
                        } else {
                            new_instrs.push(inst);
                        }
                    }

                    // Handle MakeFixedArray like Tuple, but forbid SROA if any dynamic indexing
                    InstructionKind::MakeFixedArray { dest, elements, .. }
                        if self.config.enable_tuples =>
                    {
                        let Some(ty @ MirType::FixedArray { .. }) = function.get_value_type(*dest) else {
                            new_instrs.push(inst);
                            continue;
                        };

                        if !self.is_scalarizable(ty) {
                            new_instrs.push(inst);
                            continue;
                        }

                        // Skip scalarization if used across blocks in phase 1
                        if cross_block_aggregates.contains(dest) {
                            new_instrs.push(inst);
                            continue;
                        }

                        // Full decision: general recursive checks AND no dynamic index in the array family
                        let mut visited = FxHashSet::default();
                        let can_scalarize_general = self.can_scalarize_aggregate(
                            dest,
                            &all_instructions,
                            function,
                            &cross_block_aggregates,
                            &mut visited,
                        );

                        let family_has_dynamic_index = self.array_family_has_dynamic_index_use(
                            *dest,
                            &all_instructions,
                            function,
                        );

                        if !can_scalarize_general || family_has_dynamic_index {
                            // Keep as real array (do not SROA)
                            new_instrs.push(inst);
                            continue;
                        }

                        // Arrays can be scalarized like tuples
                        agg_states.insert(*dest, AggState::tuple(elements.clone()));
                        self.stats.scalarized_builds += 1;
                        block_modified = true;
                    }

                    // 2) Partial updates → produce new aggregate state; drop instruction
                    InstructionKind::InsertTuple {
                        dest,
                        tuple_val,
                        index,
                        new_value,
                        tuple_ty: _,
                    } if self.config.enable_tuples => {
                        let Some(src_id) = tuple_val.as_operand() else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(src_state) = agg_states.get(&src_id).cloned() else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let mut next = src_state;
                        if *index >= next.elems.len() {
                            new_instrs.push(inst);
                            continue;
                        }
                        next.elems[*index] = *new_value;
                        agg_states.insert(*dest, next);
                        self.stats.inserts_forwarded += 1;
                        block_modified = true;
                    }

                    InstructionKind::InsertField {
                        dest,
                        struct_val,
                        field_name,
                        new_value,
                        struct_ty,
                    } if self.config.enable_structs => {
                        let Some(src_id) = struct_val.as_operand() else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(src_state) = agg_states.get(&src_id).cloned() else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(field_index) = struct_field_index(struct_ty, field_name) else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let mut next = src_state;
                        if field_index >= next.elems.len() {
                            new_instrs.push(inst);
                            continue;
                        }
                        next.elems[field_index] = *new_value;
                        agg_states.insert(*dest, next);
                        self.stats.inserts_forwarded += 1;
                        block_modified = true;
                    }

                    // Handle ArrayInsert with constant index like InsertTuple; dynamic index keeps instruction
                    InstructionKind::ArrayInsert {
                        dest,
                        array_val,
                        index,
                        new_value,
                        array_ty: _,
                    } if self.config.enable_tuples => {
                        // Only forward when index is a literal
                        if let Value::Literal(crate::value::Literal::Integer(idx)) = index {
                            let Some(src_id) = array_val.as_operand() else {
                                new_instrs.push(inst);
                                continue;
                            };
                            let Some(src_state) = agg_states.get(&src_id).cloned() else {
                                new_instrs.push(inst);
                                continue;
                            };
                            let mut next = src_state;
                            let i = *idx as usize;
                            if i >= next.elems.len() {
                                new_instrs.push(inst);
                                continue;
                            }
                            next.elems[i] = *new_value;
                            agg_states.insert(*dest, next);
                            self.stats.inserts_forwarded += 1;
                            block_modified = true;
                        } else {
                            // Dynamic index: keep as-is
                            new_instrs.push(inst);
                        }
                    }

                    // 3) Extracts → rewrite into Assign of the scalar; drop original
                    InstructionKind::ExtractTupleElement {
                        dest,
                        tuple,
                        index,
                        element_ty,
                    } if self.config.enable_tuples => {
                        let Some(src_id) = tuple.as_operand() else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(state) = agg_states.get(&src_id) else {
                            new_instrs.push(inst);
                            continue;
                        };
                        if *index >= state.elems.len() {
                            new_instrs.push(inst);
                            continue;
                        }
                        let scalar = state.elems[*index];

                        // Check if the extracted element is itself an aggregate that's been scalarized
                        if let Value::Operand(elem_val_id) = &scalar {
                            if let Some(elem_state) = agg_states.get(elem_val_id) {
                                // The extracted element is a scalarized aggregate - propagate its state
                                agg_states.insert(*dest, elem_state.clone());
                                self.stats.extracts_rewritten += 1;
                                block_modified = true;
                                continue;
                            }
                        }

                        // For non-aggregate elements or non-scalarized aggregates, do normal assignment
                        new_instrs.push(Instruction::assign(*dest, scalar, element_ty.clone()));
                        self.stats.extracts_rewritten += 1;
                        block_modified = true;
                    }

                    InstructionKind::ExtractStructField {
                        dest,
                        struct_val,
                        field_name,
                        field_ty,
                    } if self.config.enable_structs => {
                        let Some(src_id) = struct_val.as_operand() else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(MirType::Struct { fields, .. }) = function.get_value_type(src_id)
                        else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(field_index) = fields.iter().position(|(n, _)| n == field_name)
                        else {
                            new_instrs.push(inst);
                            continue;
                        };
                        let Some(state) = agg_states.get(&src_id) else {
                            new_instrs.push(inst);
                            continue;
                        };
                        if field_index >= state.elems.len() {
                            new_instrs.push(inst);
                            continue;
                        }
                        let scalar = state.elems[field_index];

                        // Check if the extracted field is itself an aggregate that's been scalarized
                        if let Value::Operand(field_val_id) = &scalar {
                            if let Some(field_state) = agg_states.get(field_val_id) {
                                // The extracted field is a scalarized aggregate - propagate its state
                                agg_states.insert(*dest, field_state.clone());
                                self.stats.extracts_rewritten += 1;
                                block_modified = true;
                                continue;
                            }
                        }

                        // For non-aggregate fields or non-scalarized aggregates, do normal assignment
                        new_instrs.push(Instruction::assign(*dest, scalar, field_ty.clone()));
                        self.stats.extracts_rewritten += 1;
                        block_modified = true;
                    }

                    // Handle ArrayIndex with constant index like ExtractTupleElement; dynamic index keeps instruction
                    InstructionKind::ArrayIndex {
                        dest,
                        array,
                        index,
                        element_ty,
                    } if self.config.enable_tuples => {
                        // Only forward when index is a literal
                        if let Value::Literal(crate::value::Literal::Integer(idx)) = index {
                            let Some(src_id) = array.as_operand() else {
                                new_instrs.push(inst);
                                continue;
                            };
                            let Some(state) = agg_states.get(&src_id) else {
                                new_instrs.push(inst);
                                continue;
                            };
                            let i = *idx as usize;
                            if i >= state.elems.len() {
                                new_instrs.push(inst);
                                continue;
                            }
                            let scalar = state.elems[i];

                            // Check if the extracted element is itself an aggregate that's been scalarized
                            if let Value::Operand(elem_val_id) = &scalar {
                                if let Some(elem_state) = agg_states.get(elem_val_id) {
                                    // The extracted element is a scalarized aggregate - propagate its state
                                    agg_states.insert(*dest, elem_state.clone());
                                    self.stats.extracts_rewritten += 1;
                                    block_modified = true;
                                    continue;
                                }
                            }

                            // For non-aggregate elements or non-scalarized aggregates, do normal assignment
                            new_instrs.push(Instruction::assign(*dest, scalar, element_ty.clone()));
                            self.stats.extracts_rewritten += 1;
                            block_modified = true;
                        } else {
                            // Dynamic index: keep as-is
                            new_instrs.push(inst);
                        }
                    }

                    // 4) Aggregate Assign forwarding (copy-prop for aggregates)
                    InstructionKind::Assign {
                        dest,
                        source,
                        ty: MirType::Tuple(_) | MirType::Struct { .. } | MirType::FixedArray { .. },
                    } => {
                        if let Value::Operand(src_id) = source {
                            if let Some(state) = agg_states.get(src_id).cloned() {
                                agg_states.insert(*dest, state);
                                self.stats.assigns_forwarded += 1;
                                block_modified = true;
                                // Drop the assign (no materialization yet)
                                continue;
                            }
                        }
                        new_instrs.push(inst)
                    }

                    // 5) Calls & Stores: materialize only the aggregate arguments that need it
                    InstructionKind::Call {
                        dests,
                        callee,
                        args,
                        signature,
                    } => {
                        let mut new_args = args.clone();
                        let mut touched = false;

                        for (i, arg) in args.iter().enumerate() {
                            let Some(Value::Operand(id)) = Some(arg).filter(|v| v.is_operand())
                            else {
                                continue;
                            };

                            let needs_agg = matches!(
                                signature.param_types.get(i),
                                Some(MirType::Tuple(_))
                                    | Some(MirType::Struct { .. })
                                    | Some(MirType::FixedArray { .. })
                            );
                            if !needs_agg {
                                continue;
                            }

                            if let Some(state) = agg_states.get(id) {
                                // Use the signature type for exact shape
                                let Some(param_ty) = signature.param_types.get(i).cloned() else {
                                    continue;
                                };
                                let mat_id =
                                    materialize(function, &mut new_instrs, state, &param_ty);
                                new_args[i] = Value::operand(mat_id);
                                self.stats.materializations += 1;
                                touched = true;
                                block_modified = true;
                            }
                        }

                        if touched {
                            new_instrs.push(Instruction::call(
                                dests.clone(),
                                *callee,
                                new_args,
                                signature.clone(),
                            ));
                        } else {
                            new_instrs.push(inst);
                        }
                    }

                    InstructionKind::Store { address, value, ty } => {
                        if matches!(
                            ty,
                            MirType::Tuple(_) | MirType::Struct { .. } | MirType::FixedArray { .. }
                        ) {
                            if let Value::Operand(src_id) = value {
                                if let Some(state) = agg_states.get(src_id) {
                                    let mat_id = materialize(function, &mut new_instrs, state, ty);
                                    new_instrs.push(Instruction::store(
                                        *address,
                                        Value::operand(mat_id),
                                        ty.clone(),
                                    ));
                                    self.stats.materializations += 1;
                                    block_modified = true;
                                    continue;
                                }
                            }
                        }
                        new_instrs.push(inst);
                    }

                    // Everything else: keep as-is
                    _ => new_instrs.push(inst),
                }
            }

            // Process terminator - materialize any scalarized aggregates in returns
            let mut new_return_values = None;
            let terminator_values = if let Some(block) = function.get_basic_block(bb) {
                if let crate::Terminator::Return { values } = &block.terminator {
                    Some(values.clone())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(values) = terminator_values {
                let mut updated_values = values.clone();
                let mut any_materialized = false;

                // Collect aggregates to materialize first
                let mut to_materialize = Vec::new();
                for (i, val) in values.iter().enumerate() {
                    if let Value::Operand(id) = val {
                        if let Some(state) = agg_states.get(id) {
                            if let Some(ty) = function.get_value_type(*id) {
                                to_materialize.push((i, state.clone(), ty.clone()));
                            }
                        }
                    }
                }

                // Now materialize them
                for (i, state, ty) in to_materialize {
                    let mat_id = materialize(function, &mut new_instrs, &state, &ty);
                    updated_values[i] = Value::operand(mat_id);
                    self.stats.materializations += 1;
                    any_materialized = true;
                    block_modified = true;
                }

                if any_materialized {
                    new_return_values = Some(updated_values);
                }
            }

            if block_modified {
                modified_any = true;
                if let Some(block_mut) = function.get_basic_block_mut(bb) {
                    // Reinstall updated instruction list: phi prefix + rewritten tail
                    block_mut.instructions.clear();
                    block_mut.instructions.extend(new_instrs);

                    // Update terminator if it was modified
                    if let Some(new_vals) = new_return_values {
                        block_mut.terminator = crate::Terminator::Return { values: new_vals };
                    }
                }
            }
        }

        if modified_any {
            log::debug!(
                "SROA pass stats: scalarized={}, extracts_rewritten={}, inserts={}, assigns={}, materializations={}",
                self.stats.scalarized_builds,
                self.stats.extracts_rewritten,
                self.stats.inserts_forwarded,
                self.stats.assigns_forwarded,
                self.stats.materializations
            );
        }

        modified_any
    }

    fn name(&self) -> &'static str {
        "ScalarReplacementOfAggregates"
    }
}

/// A tracked aggregate value decomposed into components
#[derive(Clone, Debug)]
struct AggState {
    /// Elements are in tuple order, or struct field declaration order
    elems: Vec<Value>,
}

impl AggState {
    const fn tuple(elements: Vec<Value>) -> Self {
        Self { elems: elements }
    }

    /// Build a state for a struct, aligning to the *type* field order
    fn from_struct_fields(
        provided: &[(String, Value)],
        ty_fields: &[(String, MirType)],
    ) -> Option<Self> {
        let mut elems: Vec<Value> = Vec::with_capacity(ty_fields.len());
        for (name, _ty) in ty_fields {
            if let Some((_, v)) = provided.iter().find(|(n, _)| n == name) {
                elems.push(*v);
            } else {
                return None; // missing field initialization (be conservative)
            }
        }
        Some(Self { elems })
    }
}

fn struct_field_index(ty: &MirType, name: &str) -> Option<usize> {
    match ty {
        MirType::Struct { fields, .. } => fields.iter().position(|(n, _)| n == name),
        _ => None,
    }
}

/// Create a concrete aggregate value (MakeTuple/MakeStruct) from components,
/// insert it *before* the current use by pushing into `sink`, and return the new ValueId
fn materialize(
    func: &mut MirFunction,
    sink: &mut Vec<Instruction>,
    state: &AggState,
    ty: &MirType,
) -> ValueId {
    match ty {
        MirType::Tuple(elem_tys) => {
            // Sanity check on arity; if mismatch, truncate/pad with unit (defensive)
            let mut elems = state.elems.clone();
            if elems.len() != elem_tys.len() {
                elems.truncate(elem_tys.len());
                while elems.len() < elem_tys.len() {
                    elems.push(Value::unit());
                }
            }
            let dest = func.new_typed_value_id(ty.clone());
            sink.push(Instruction::make_tuple(dest, elems));
            dest
        }
        MirType::Struct { fields, .. } => {
            // Rebuild in declared order with names
            let mut pairs: Vec<(String, Value)> = Vec::with_capacity(fields.len());
            for (i, (name, _fty)) in fields.iter().enumerate() {
                let v = state.elems.get(i).cloned().unwrap_or_else(Value::unit);
                pairs.push((name.clone(), v));
            }
            let dest = func.new_typed_value_id(ty.clone());
            sink.push(Instruction::make_struct(dest, pairs, ty.clone()));
            dest
        }
        MirType::FixedArray { element_type, size } => {
            // Rebuild array from elements. The tracked element count must match the array size.
            assert_eq!(
                state.elems.len(),
                *size,
                "SROA materialize: array element count mismatch (have {}, want {})",
                state.elems.len(),
                size
            );
            let dest = func.new_typed_value_id(ty.clone());
            sink.push(Instruction::make_fixed_array(
                dest,
                state.elems.clone(),
                (**element_type).clone(),
            ));
            dest
        }
        _ => {
            // Not an aggregate; should not be called
            func.new_typed_value_id(ty.clone())
        }
    }
}

#[cfg(test)]
#[path = "sroa_tests.rs"]
mod tests;
