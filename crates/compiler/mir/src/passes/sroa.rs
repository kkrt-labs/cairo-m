//! Scalar Replacement of Aggregates (SROA) Pass
//!
//! This pass decomposes tuples and structs into per-field SSA values, eliminating
//! unnecessary aggregate construction and enabling better downstream optimizations.
//! Aggregates are only materialized on-demand at ABI boundaries (calls, stores).

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    instruction::InstructionKind, value::Value, BasicBlockId, Instruction, MirFunction, MirType,
    ValueId,
};

use super::MirPass;

/// Phase-1 SROA: tuples & structs, no arrays, no aggregate PHIs.
///
/// Strategy:
///  - Track aggregates built by MakeTuple/MakeStruct (and copies via Assign)
///  - Model partial updates (InsertTuple/InsertField) as per-component SSA
///  - Rewrite Extract* → Assign of the scalar value
///  - At uses that REQUIRE a true aggregate (call param typed as aggregate, or Store with aggregate ty),
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
            _ => false,
        }
    }

    /// Find aggregates that are used across block boundaries
    /// In phase 1, we skip scalarizing these to maintain correctness
    fn find_cross_block_aggregates(&self, function: &MirFunction) -> FxHashSet<ValueId> {
        let mut cross_block = FxHashSet::default();
        let mut defined_in_block: FxHashMap<ValueId, BasicBlockId> = FxHashMap::default();

        // First pass: record where each aggregate is defined
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            for inst in &block.instructions {
                match &inst.kind {
                    InstructionKind::MakeTuple { dest, .. }
                    | InstructionKind::MakeStruct { dest, .. } => {
                        if let Some(ty) = function.get_value_type(*dest) {
                            if self.is_scalarizable(ty) {
                                defined_in_block.insert(*dest, block_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Second pass: check if aggregates are used in different blocks
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            // Check all value uses in instructions
            for inst in &block.instructions {
                self.collect_value_uses_in_instruction(&inst.kind, |used_value| {
                    if let Some(&def_block) = defined_in_block.get(&used_value) {
                        if def_block != block_id {
                            cross_block.insert(used_value);
                        }
                    }
                });
            }

            // Check terminator uses
            self.collect_value_uses_in_terminator(&block.terminator, |used_value| {
                if let Some(&def_block) = defined_in_block.get(&used_value) {
                    if def_block != block_id {
                        cross_block.insert(used_value);
                    }
                }
            });
        }

        cross_block
    }

    /// Helper to collect all value uses in an instruction
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

    /// Check if an instruction requires materialization of its aggregate operands
    fn requires_materialization<'a>(
        &self,
        inst: &'a InstructionKind,
        operand_idx: usize,
    ) -> Option<&'a MirType> {
        match inst {
            InstructionKind::Call { signature, .. } => {
                // Check if this argument position expects an aggregate
                signature
                    .param_types
                    .get(operand_idx)
                    .filter(|ty| matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }))
            }
            InstructionKind::Store { ty, .. } if operand_idx == 1 => {
                // The value being stored (second operand) needs materialization if aggregate
                if matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }) {
                    Some(ty)
                } else {
                    None
                }
            }
            InstructionKind::AddressOf { .. } => {
                // Taking address of aggregate requires materialization
                // We'd need to check the operand type, but for now be conservative
                None // Will be handled in full implementation
            }
            _ => None,
        }
    }
}

impl MirPass for ScalarReplacementOfAggregates {
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

                        agg_states.insert(*dest, AggState::tuple(elements.clone()));
                        self.stats.scalarized_builds += 1;
                        block_modified = true;
                    }

                    InstructionKind::MakeStruct {
                        dest,
                        fields,
                        struct_ty,
                    } if self.config.enable_structs => {
                        if !self.is_scalarizable(struct_ty) {
                            new_instrs.push(inst);
                            continue;
                        }

                        // Skip scalarization if used across blocks in phase 1
                        if cross_block_aggregates.contains(dest) {
                            new_instrs.push(inst);
                            continue;
                        }

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

                    // 4) Aggregate Assign forwarding (copy-prop for aggregates)
                    InstructionKind::Assign { dest, source, ty }
                        if matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }) =>
                    {
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
                                Some(MirType::Tuple(_)) | Some(MirType::Struct { .. })
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
                        if matches!(ty, MirType::Tuple(_) | MirType::Struct { .. }) {
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

            if block_modified {
                modified_any = true;
                if let Some(block_mut) = function.get_basic_block_mut(bb) {
                    // Reinstall updated instruction list: phi prefix + rewritten tail
                    block_mut.instructions.clear();
                    block_mut.instructions.extend(new_instrs);
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
        _ => {
            // Not an aggregate; should not be called
            func.new_typed_value_id(ty.clone())
        }
    }
}

#[cfg(test)]
#[path = "sroa_tests.rs"]
mod tests;
