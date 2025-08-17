//! # MIR Optimization Passes
//!
//! This module implements various optimization passes that can be applied to MIR functions
//! to improve code quality and remove dead code.

pub mod const_fold;
pub mod lower_aggregates;
pub mod mem2reg_ssa;
pub mod pre_opt;
pub mod ssa_destruction;
pub mod var_ssa;

pub use lower_aggregates::LowerAggregatesPass;
pub use var_ssa::VarSsaPass;

use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BinaryOp, InstructionKind, Literal, MirFunction, MirType, Terminator, Value};

/// Analyzes a MIR function to determine if it uses memory operations
/// that require SROA/Mem2Reg optimization passes.
pub fn function_uses_memory(function: &MirFunction) -> bool {
    for block in function.basic_blocks.iter() {
        for instruction in &block.instructions {
            match &instruction.kind {
                InstructionKind::FrameAlloc { .. }
                | InstructionKind::Load { .. }
                | InstructionKind::Store { .. }
                | InstructionKind::GetElementPtr { .. }
                | InstructionKind::AddressOf { .. } => {
                    return true;
                }
                _ => continue,
            }
        }
    }
    false
}

/// A trait for MIR optimization passes
pub trait MirPass {
    /// Apply this pass to a MIR function
    /// Returns true if the function was modified
    fn run(&mut self, function: &mut MirFunction) -> bool;

    /// Get the name of this pass for debugging
    fn name(&self) -> &'static str;
}

/// A wrapper for conditional pass execution
///
/// This allows passes to be skipped based on function characteristics,
/// improving compilation performance for functions that don't need certain optimizations.
pub struct ConditionalPass {
    pass: Box<dyn MirPass>,
    condition: fn(&MirFunction) -> bool,
}

impl ConditionalPass {
    /// Create a new conditional pass
    pub fn new(pass: Box<dyn MirPass>, condition: fn(&MirFunction) -> bool) -> Self {
        Self { pass, condition }
    }
}

impl MirPass for ConditionalPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        if (self.condition)(function) {
            self.pass.run(function)
        } else {
            // Skip the pass - no changes needed
            false
        }
    }

    fn name(&self) -> &'static str {
        self.pass.name()
    }
}

/// Fuse Compare and Branch Pass
///
/// This pass identifies a `BinaryOp` performing a comparison (e.g., `Eq`)
/// whose result is only used in a subsequent `If` terminator, and fuses them
/// into a single, more efficient `BranchCmp` terminator.
///
/// ### Before:
/// ```mir
/// block_N:
///   %1 = binary_op Eq, %a, %b
///   if %1 then jump then_block else jump else_block
/// ```
///
/// ### After:
/// ```mir
/// block_N:
///   if %a Eq %b then jump then_block else jump else_block
/// ```
#[derive(Debug, Default)]
pub struct FuseCmpBranch;

impl FuseCmpBranch {
    /// Create a new pass
    pub const fn new() -> Self {
        Self
    }

    /// Returns true if an op is a comparison that can be fused.
    const fn is_fusible_comparison(op: BinaryOp) -> bool {
        // For now, only Eq is guaranteed to work. In the future we might have Le / Ge opcodes
        matches!(
            op,
            BinaryOp::Eq | BinaryOp::Neq | BinaryOp::U32Eq | BinaryOp::U32Neq
        )
    }
}

impl MirPass for FuseCmpBranch {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;
        let use_counts = function.get_value_use_counts();

        for block in function.basic_blocks.iter_mut() {
            // We are looking for a block that ends in `If`.
            if let Terminator::If {
                condition: Value::Operand(cond_val_id),
                then_target,
                else_target,
            } = block.terminator
            {
                // The condition's result must be used exactly once (in this `If`).
                if use_counts.get(&cond_val_id).cloned() != Some(1) {
                    continue;
                }

                // The instruction defining the condition must be the last one in the block.
                if let Some(last_instr) = block.instructions.last() {
                    if last_instr.destination() == Some(cond_val_id) {
                        if let InstructionKind::BinaryOp {
                            op, left, right, ..
                        } = &last_instr.kind
                        {
                            if Self::is_fusible_comparison(*op) {
                                // We found the pattern! Perform the fusion.

                                // Helper to check if a value represents zero
                                let is_zero = |v: &Value| {
                                    matches!(
                                        v,
                                        Value::Literal(Literal::Integer(0))
                                            | Value::Literal(Literal::Boolean(false))
                                    )
                                };

                                // We first check for comparisons with 0 which can be optimized
                                match (*op, is_zero(left), is_zero(right)) {
                                    (BinaryOp::Eq | BinaryOp::U32Eq, true, false) => {
                                        // 0 == x is equivalent to !x, so we switch the targets
                                        block.terminator =
                                            Terminator::branch(*right, else_target, then_target);
                                    }
                                    (BinaryOp::Eq | BinaryOp::U32Eq, false, true) => {
                                        // x == 0 is equivalent to !x, so we switch the targets
                                        block.terminator =
                                            Terminator::branch(*left, else_target, then_target);
                                    }
                                    (BinaryOp::Neq | BinaryOp::U32Neq, true, false) => {
                                        // 0 != x is equivalent to x, so we use x as the condition
                                        block.terminator =
                                            Terminator::branch(*right, then_target, else_target);
                                    }
                                    (BinaryOp::Neq | BinaryOp::U32Neq, false, true) => {
                                        // x != 0 is equivalent to x, so we use x as the condition
                                        block.terminator =
                                            Terminator::branch(*left, then_target, else_target);
                                    }
                                    _ => {
                                        // For all other cases, we can fuse the comparison and branch
                                        block.terminator = Terminator::branch_cmp(
                                            *op,
                                            *left,
                                            *right,
                                            then_target,
                                            else_target,
                                        );
                                    }
                                }

                                // Remove the now-redundant BinaryOp instruction.
                                block.instructions.pop();

                                modified = true;
                            }
                        } else if let InstructionKind::UnaryOp { op, source, .. } = &last_instr.kind
                        {
                            if matches!(op, UnaryOp::Not) {
                                // If the condition is a not, we switch the targets
                                // For simplicity, we assume dumb conditions such as !42 will never appear in the source code
                                block.terminator =
                                    Terminator::branch(*source, else_target, then_target);

                                // Remove the now-redundant UnaryOp instruction.
                                block.instructions.pop();

                                modified = true;
                            }
                        }
                    }
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "FuseCmpBranch"
    }
}

/// Dead Code Elimination Pass
///
/// This pass identifies and removes unreachable basic blocks from the function.
/// It uses the function's built-in reachability analysis to find dead blocks.
#[derive(Debug, Default)]
pub struct DeadCodeElimination;

impl DeadCodeElimination {
    /// Create a new dead code elimination pass
    pub const fn new() -> Self {
        Self
    }
}

impl MirPass for DeadCodeElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let unreachable = function.unreachable_blocks();

        if unreachable.is_empty() {
            return false; // No changes made
        }

        // Sort in reverse order to avoid index invalidation when removing
        let mut unreachable = unreachable;
        unreachable.sort_by_key(|a| std::cmp::Reverse(a.index()));

        // Remove unreachable blocks
        for block_id in unreachable {
            // Note: We need to be careful about removing blocks because IndexVec doesn't
            // directly support removal. For now, we'll mark them as "dead" by replacing
            // with empty blocks. A more sophisticated implementation would compact the CFG.
            if let Some(block) = function.get_basic_block_mut(block_id) {
                block.instructions.clear();
                block.set_terminator(crate::Terminator::Unreachable);
            }
        }

        true // Modified the function
    }

    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
}

/// MIR Validation Pass
///
/// This pass validates the MIR function to ensure it meets all invariants.
/// It's useful to run after other passes to ensure correctness.
#[derive(Debug)]
pub struct Validation {
    /// Whether to check SSA invariants (single definition per value)
    /// Should be false after SSA destruction pass
    check_ssa_invariants: bool,
}

impl Default for Validation {
    fn default() -> Self {
        Self::new()
    }
}

impl Validation {
    /// Create a new validation pass that checks SSA invariants
    pub const fn new() -> Self {
        Self {
            check_ssa_invariants: true,
        }
    }

    /// Create a new validation pass for post-SSA form
    /// This skips SSA invariant checks since SSA destruction creates multiple assignments
    pub const fn new_post_ssa() -> Self {
        Self {
            check_ssa_invariants: false,
        }
    }
}

impl MirPass for Validation {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        if let Err(err) = function.validate() {
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!(
                    "[ERROR] MIR Validation failed for function '{}': {}",
                    function.name, err
                );
            }
            // Validation passes don't modify the function
            return false;
        }

        // Check for additional invariants
        self.validate_value_usage(function);
        self.validate_pointer_types(function);
        self.validate_store_types(function);
        self.validate_gep_usage(function);
        self.validate_aggregate_operations(function);
        self.validate_cfg_structure(function);
        self.validate_single_definition(function);

        false // Validation doesn't modify the function
    }

    fn name(&self) -> &'static str {
        "Validation"
    }
}

impl Validation {
    /// Validate that all used values are defined somewhere in the function
    ///
    /// In SSA form, values can be used from any dominating block, not just the same block.
    /// This validation ensures that every used value is either:
    /// - A function parameter
    /// - Defined by some instruction in the function
    fn validate_value_usage(&self, function: &MirFunction) {
        // Collect all defined values in the entire function
        let mut all_defined_values = std::collections::HashSet::new();

        // Add function parameters
        for param in &function.parameters {
            all_defined_values.insert(*param);
        }

        // Add all values defined by instructions
        for (_block_id, block) in function.basic_blocks() {
            for instruction in &block.instructions {
                if let Some(dest) = instruction.destination() {
                    all_defined_values.insert(dest);
                }
            }
        }

        // Now check that all used values are defined somewhere
        for (block_id, block) in function.basic_blocks() {
            let used_values = block.used_values();
            for used_value in used_values {
                if !all_defined_values.contains(&used_value) {
                    // This is a real error - value is not defined anywhere
                    eprintln!(
                        "Error: Block {block_id:?} uses value {used_value:?} that is not defined anywhere in the function"
                    );
                }
            }
        }
    }

    /// Validate that Load instructions only use pointer-typed addresses
    fn validate_pointer_types(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let InstructionKind::Load {
                    address: Value::Operand(addr_id),
                    ..
                } = &instruction.kind
                {
                    // Check that the address operand is a pointer
                    if let Some(addr_type) = function.get_value_type(*addr_id) {
                        if !matches!(addr_type, MirType::Pointer(_)) {
                            if std::env::var("RUST_LOG").is_ok() {
                                eprintln!(
                                    "[ERROR] Block {block_id:?}, instruction {instr_idx}: Load instruction uses non-pointer address {addr_id:?} with type {addr_type:?}"
                                );
                            }
                        }
                    } else {
                        if std::env::var("RUST_LOG").is_ok() {
                            eprintln!(
                                "[WARN] Block {block_id:?}, instruction {instr_idx}: Load instruction uses address {addr_id:?} with unknown type"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate that Store instructions only use pointer-typed addresses
    fn validate_store_types(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let InstructionKind::Store { address, .. } = &instruction.kind {
                    if let Value::Operand(addr_id) = address {
                        // Check that the address operand is a pointer
                        if let Some(addr_type) = function.get_value_type(*addr_id) {
                            if !matches!(addr_type, MirType::Pointer(_))
                                && std::env::var("RUST_LOG").is_ok()
                            {
                                eprintln!(
                                    "[ERROR] Block {block_id:?}, instruction {instr_idx}: Store instruction uses non-pointer address {addr_id:?} with type {addr_type:?}"
                                );
                            }
                        } else if std::env::var("RUST_LOG").is_ok() {
                            eprintln!(
                                "[WARN] Block {block_id:?}, instruction {instr_idx}: Store instruction uses address {addr_id:?} with unknown type"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate GEP usage (warn about raw offset GEPs)
    fn validate_gep_usage(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let InstructionKind::GetElementPtr { offset, .. } = &instruction.kind {
                    // Warn if using raw integer offsets (not typed indexing)
                    // This is a temporary warning until typed GEP is fully implemented
                    if let Value::Literal(Literal::Integer(offset_val)) = offset {
                        if *offset_val != 0 && std::env::var("RUST_LOG").is_ok() {
                            eprintln!(
                                "[WARN] Block {block_id:?}, instruction {instr_idx}: GEP uses raw offset {offset_val}. Consider using typed GEP once available."
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate CFG structure (check for critical edges, unreachable blocks, etc.)
    fn validate_cfg_structure(&self, function: &MirFunction) {
        use crate::cfg::{get_predecessors, is_critical_edge};

        // Check for unreachable blocks
        let unreachable = function.unreachable_blocks();
        if !unreachable.is_empty() && std::env::var("RUST_LOG").is_ok() {
            eprintln!(
                "[WARN] Function '{}' contains {} unreachable blocks: {:?}",
                function.name,
                unreachable.len(),
                unreachable
            );
        }

        // Warn about critical edges (these should be split for correct SSA destruction)
        for (pred_id, pred_block) in function.basic_blocks.iter_enumerated() {
            for succ_id in pred_block.terminator.target_blocks() {
                if is_critical_edge(function, pred_id, succ_id) {
                    log::debug!(
                        "Critical edge detected: {pred_id:?} -> {succ_id:?} in function '{}'",
                        function.name
                    );
                }
            }
        }

        // Check that entry block has no predecessors
        let entry_preds = get_predecessors(function, function.entry_block);
        if !entry_preds.is_empty() && std::env::var("RUST_LOG").is_ok() {
            eprintln!(
                "[ERROR] Entry block {:?} has predecessors: {:?} in function '{}'",
                function.entry_block, entry_preds, function.name
            );
        }
    }

    /// Validate that each value is defined exactly once (only in SSA form)
    fn validate_single_definition(&self, function: &MirFunction) {
        // Skip SSA invariant checks if we're in post-SSA form
        if !self.check_ssa_invariants {
            return;
        }

        let mut defined_values = std::collections::HashSet::new();

        // Check parameters
        for &param_id in &function.parameters {
            if !defined_values.insert(param_id) && std::env::var("RUST_LOG").is_ok() {
                eprintln!(
                    "[ERROR] Value {param_id:?} is defined multiple times as a parameter in function '{}'",
                    function.name
                );
            }
        }

        // Check instructions
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let Some(dest) = instruction.destination() {
                    if !defined_values.insert(dest) && std::env::var("RUST_LOG").is_ok() {
                        eprintln!(
                            "[ERROR] Value {dest:?} is defined multiple times (block {block_id:?}, instruction {instr_idx}) in function '{}'",
                            function.name
                        );
                    }
                }
            }
        }
    }

    /// Validate aggregate operations (MakeTuple, ExtractTuple, MakeStruct, etc.)
    fn validate_aggregate_operations(&self, function: &MirFunction) {
        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    // Validate ExtractTupleElement
                    InstructionKind::ExtractTupleElement {
                        tuple,
                        index,
                        element_ty,
                        ..
                    } => {
                        if let Value::Operand(tuple_id) = tuple {
                            if let Some(tuple_type) = function.get_value_type(*tuple_id) {
                                // Check that ExtractTupleElement is not used on arrays
                                if matches!(tuple_type, MirType::Array { .. })
                                    && std::env::var("RUST_LOG").is_ok()
                                {
                                    eprintln!(
                                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                        ExtractTupleElement used on array type - arrays should use memory operations (get_element_ptr + load)"
                                    );
                                }
                                if let MirType::Tuple(elements) = tuple_type {
                                    // Check index bounds
                                    if *index >= elements.len() && std::env::var("RUST_LOG").is_ok()
                                    {
                                        eprintln!(
                                            "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                            ExtractTupleElement index {} out of bounds for tuple with {} elements",
                                            index, elements.len()
                                        );
                                    }
                                    // Check type consistency
                                    if let Some(expected_ty) = elements.get(*index) {
                                        if expected_ty != element_ty
                                            && !matches!(expected_ty, MirType::Unknown)
                                            && !matches!(element_ty, MirType::Unknown)
                                            && std::env::var("RUST_LOG").is_ok()
                                        {
                                            eprintln!(
                                                "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                                ExtractTupleElement type mismatch: expected {expected_ty:?}, got {element_ty:?}"
                                            );
                                        }
                                    }
                                } else if std::env::var("RUST_LOG").is_ok() {
                                    eprintln!(
                                        "[WARN] Block {block_id:?}, instruction {instr_idx}: \
                                        ExtractTupleElement on non-tuple type {tuple_type:?}"
                                    );
                                }
                            }
                        }
                    }

                    // Validate ExtractStructField
                    InstructionKind::ExtractStructField {
                        struct_val,
                        field_name,
                        field_ty,
                        ..
                    } => {
                        if let Value::Operand(struct_id) = struct_val {
                            if let Some(struct_type) = function.get_value_type(*struct_id) {
                                // Check that ExtractStructField is not used on arrays
                                if matches!(struct_type, MirType::Array { .. })
                                    && std::env::var("RUST_LOG").is_ok()
                                {
                                    eprintln!(
                                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                        ExtractStructField used on array type - arrays should use memory operations"
                                    );
                                }
                                if let MirType::Struct { fields, .. } = struct_type {
                                    // Check field exists
                                    if let Some((_, expected_ty)) =
                                        fields.iter().find(|(name, _)| name == field_name)
                                    {
                                        // Check type consistency
                                        if expected_ty != field_ty
                                            && !matches!(expected_ty, MirType::Unknown)
                                            && !matches!(field_ty, MirType::Unknown)
                                            && std::env::var("RUST_LOG").is_ok()
                                        {
                                            eprintln!(
                                                "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                                ExtractStructField type mismatch for field '{}': expected {expected_ty:?}, got {field_ty:?}",
                                                field_name
                                            );
                                        }
                                    } else if std::env::var("RUST_LOG").is_ok() {
                                        eprintln!(
                                            "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                            ExtractStructField: field '{}' not found in struct",
                                            field_name
                                        );
                                    }
                                } else if std::env::var("RUST_LOG").is_ok() {
                                    eprintln!(
                                        "[WARN] Block {block_id:?}, instruction {instr_idx}: \
                                        ExtractStructField on non-struct type {struct_type:?}"
                                    );
                                }
                            }
                        }
                    }

                    // Validate MakeTuple
                    InstructionKind::MakeTuple { dest, elements } => {
                        if let Some(tuple_type) = function.get_value_type(*dest) {
                            // Check that MakeTuple is not creating an array type
                            if matches!(tuple_type, MirType::Array { .. })
                                && std::env::var("RUST_LOG").is_ok()
                            {
                                eprintln!(
                                    "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                    MakeTuple used to create array type - arrays should use frame_alloc + store operations"
                                );
                            }
                            if let MirType::Tuple(expected_types) = tuple_type {
                                // Check arity matches
                                if elements.len() != expected_types.len()
                                    && std::env::var("RUST_LOG").is_ok()
                                {
                                    eprintln!(
                                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                        MakeTuple arity mismatch: expected {} elements, got {}",
                                        expected_types.len(),
                                        elements.len()
                                    );
                                }
                                // Check element types
                                for (idx, (elem_val, expected_ty)) in
                                    elements.iter().zip(expected_types.iter()).enumerate()
                                {
                                    if let Value::Operand(elem_id) = elem_val {
                                        if let Some(elem_ty) = function.get_value_type(*elem_id) {
                                            if elem_ty != expected_ty
                                                && !matches!(elem_ty, MirType::Unknown)
                                                && !matches!(expected_ty, MirType::Unknown)
                                                && std::env::var("RUST_LOG").is_ok()
                                            {
                                                eprintln!(
                                                    "[WARN] Block {block_id:?}, instruction {instr_idx}: \
                                                    MakeTuple element {idx} type mismatch: expected {expected_ty:?}, got {elem_ty:?}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Validate MakeStruct
                    InstructionKind::MakeStruct {
                        dest: _,
                        fields,
                        struct_ty,
                    } => {
                        if let MirType::Struct {
                            fields: expected_fields,
                            ..
                        } = struct_ty
                        {
                            // Check all required fields are present
                            for (expected_name, _) in expected_fields {
                                if !fields.iter().any(|(name, _)| name == expected_name)
                                    && std::env::var("RUST_LOG").is_ok()
                                {
                                    eprintln!(
                                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                        MakeStruct missing required field '{}'",
                                        expected_name
                                    );
                                }
                            }
                            // Check for duplicate fields
                            let mut seen_fields = std::collections::HashSet::new();
                            for (field_name, _) in fields {
                                if !seen_fields.insert(field_name.clone())
                                    && std::env::var("RUST_LOG").is_ok()
                                {
                                    eprintln!(
                                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                        MakeStruct has duplicate field '{}'",
                                        field_name
                                    );
                                }
                            }
                            // Check field types
                            for (field_name, field_val) in fields {
                                if let Some((_, expected_ty)) =
                                    expected_fields.iter().find(|(name, _)| name == field_name)
                                {
                                    if let Value::Operand(val_id) = field_val {
                                        if let Some(val_ty) = function.get_value_type(*val_id) {
                                            if val_ty != expected_ty
                                                && !matches!(val_ty, MirType::Unknown)
                                                && !matches!(expected_ty, MirType::Unknown)
                                                && std::env::var("RUST_LOG").is_ok()
                                            {
                                                eprintln!(
                                                    "[WARN] Block {block_id:?}, instruction {instr_idx}: \
                                                    MakeStruct field '{}' type mismatch: expected {expected_ty:?}, got {val_ty:?}",
                                                    field_name
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Validate InsertField
                    InstructionKind::InsertField {
                        field_name,
                        new_value,
                        struct_ty,
                        ..
                    } => {
                        if let MirType::Struct { fields, .. } = struct_ty {
                            // Check field exists
                            if let Some((_, expected_ty)) =
                                fields.iter().find(|(name, _)| name == field_name)
                            {
                                // Check new value type
                                if let Value::Operand(val_id) = new_value {
                                    if let Some(val_ty) = function.get_value_type(*val_id) {
                                        if val_ty != expected_ty
                                            && !matches!(val_ty, MirType::Unknown)
                                            && !matches!(expected_ty, MirType::Unknown)
                                            && std::env::var("RUST_LOG").is_ok()
                                        {
                                            eprintln!(
                                                "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                                InsertField type mismatch for field '{}': expected {expected_ty:?}, got {val_ty:?}",
                                                field_name
                                            );
                                        }
                                    }
                                }
                            } else if std::env::var("RUST_LOG").is_ok() {
                                eprintln!(
                                    "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                    InsertField: field '{}' not found in struct",
                                    field_name
                                );
                            }
                        }
                    }

                    // Validate InsertTuple
                    InstructionKind::InsertTuple {
                        tuple_val,
                        index,
                        new_value,
                        ..
                    } => {
                        if let Value::Operand(tuple_id) = tuple_val {
                            if let Some(tuple_type) = function.get_value_type(*tuple_id) {
                                if let MirType::Tuple(elements) = tuple_type {
                                    // Check index bounds
                                    if *index >= elements.len() && std::env::var("RUST_LOG").is_ok()
                                    {
                                        eprintln!(
                                            "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                            InsertTuple index {} out of bounds for tuple with {} elements",
                                            index, elements.len()
                                        );
                                    }
                                    // Check type consistency
                                    if let Some(expected_ty) = elements.get(*index) {
                                        if let Value::Operand(val_id) = new_value {
                                            if let Some(val_ty) = function.get_value_type(*val_id) {
                                                if val_ty != expected_ty
                                                    && !matches!(val_ty, MirType::Unknown)
                                                    && !matches!(expected_ty, MirType::Unknown)
                                                    && std::env::var("RUST_LOG").is_ok()
                                                {
                                                    eprintln!(
                                                        "[ERROR] Block {block_id:?}, instruction {instr_idx}: \
                                                        InsertTuple type mismatch at index {}: expected {expected_ty:?}, got {val_ty:?}",
                                                        index
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "./passes/validation_tests.rs"]
mod validation_tests;

#[cfg(test)]
#[path = "./passes/aggregate_validation_tests.rs"]
mod aggregate_validation_tests;

/// A pass manager that can run multiple passes in sequence
#[derive(Default)]
pub struct PassManager {
    passes: Vec<Box<dyn MirPass>>,
}

impl PassManager {
    /// Create a new pass manager
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the manager
    pub fn add_pass<P: MirPass + 'static>(mut self, pass: P) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// Add a conditional pass to the manager
    /// The pass will only run if the condition function returns true
    pub fn add_conditional_pass<P: MirPass + 'static>(
        mut self,
        pass: P,
        condition: fn(&MirFunction) -> bool,
    ) -> Self {
        self.passes
            .push(Box::new(ConditionalPass::new(Box::new(pass), condition)));
        self
    }

    /// Run all passes on the function
    /// Returns true if any pass modified the function
    pub fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        for pass in &mut self.passes {
            if pass.run(function) {
                modified = true;
                eprintln!(
                    "Pass '{}' modified function '{}'",
                    pass.name(),
                    function.name
                );
            }
        }

        modified
    }

    /// Create a basic optimization pipeline (minimal optimizations)
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new_post_ssa())
    }

    /// Create a standard optimization pipeline (default)
    ///
    /// The pipeline is optimized for functions using value-based aggregates,
    /// conditionally running expensive memory-oriented passes only when needed.
    pub fn standard_pipeline() -> Self {
        Self::new()
            // Always run basic cleanup passes
            .add_pass(pre_opt::PreOptimizationPass::new())
            // Run constant folding for aggregates
            .add_pass(const_fold::ConstFoldPass::new())
            // Run Variable-SSA pass to convert mutable variables to SSA form
            .add_pass(var_ssa::VarSsaPass::new())
            // Conditionally run memory-oriented optimization passes
            .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
            // Always run validation and remaining optimization passes
            .add_pass(Validation::new()) // Validate SSA form before destruction
            .add_pass(ssa_destruction::SsaDestructionPass::new()) // Eliminate Phi nodes before codegen
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new_post_ssa()) // Validate post-SSA form without SSA invariants
    }

    /// Create an aggressive optimization pipeline
    pub fn aggressive_pipeline() -> Self {
        Self::new()
            // Pre-optimization and constant folding
            .add_pass(pre_opt::PreOptimizationPass::new())
            .add_pass(const_fold::ConstFoldPass::new())
            // Memory optimization (conditional)
            .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
            // Additional aggressive passes could be added here
            // For now, same as standard but could add more in future
            .add_pass(Validation::new())
            .add_pass(ssa_destruction::SsaDestructionPass::new())
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new_post_ssa())
    }
}

#[cfg(test)]
#[path = "passes_tests.rs"]
mod tests;
