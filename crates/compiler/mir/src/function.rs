//! # MIR Function
//!
//! This module defines the function-level MIR representation, including
//! the Control Flow Graph (CFG) of basic blocks.

use index_vec::IndexVec;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;

use crate::{
    indent_str, BasicBlock, BasicBlockId, Instruction, MirType, PrettyPrint, Terminator, Value,
    ValueId,
};

/// A simple definition identifier for MIR that doesn't depend on Salsa lifetimes
///
/// This is derived from `DefinitionId` but simplified for use in MIR.
/// It allows MIR to reference semantic definitions without database dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirDefinitionId {
    /// Index of the definition within its file
    pub definition_index: usize,
    /// A simple file identifier (we can use a hash or index)
    pub file_id: u64,
}

/// The MIR for a single function, laid out as a Control Flow Graph (CFG)
///
/// A `MirFunction` represents the complete control flow and data flow
/// for a single function, using a graph of basic blocks.
///
/// # Design Notes
///
/// - Basic blocks are stored in an `IndexVec` for efficient access
/// - Each function has exactly one entry block
/// - Local variables from semantic analysis are mapped to MIR values
/// - The function maintains the mapping from semantic definitions to MIR values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirFunction {
    /// The name of the function (for debugging and linking)
    pub name: String,

    /// Maps semantic variable definitions to MIR values during lowering.
    /// Not used by optimization passes, which work directly with ValueIds.
    /// This preserves the connection between semantic analysis and MIR for debugging.
    pub locals: FxHashMap<MirDefinitionId, ValueId>,

    /// All basic blocks in this function, forming the CFG
    pub basic_blocks: IndexVec<BasicBlockId, BasicBlock>,

    /// The entry point of the function (always valid if function has blocks)
    pub entry_block: BasicBlockId,

    /// Function parameters mapped to their MIR values
    /// The order matches the function signature
    pub parameters: Vec<ValueId>,

    /// The return values of the function
    /// Empty for void functions, contains one or more values for functions with returns
    pub return_values: Vec<ValueId>,

    /// Next available value ID for generating new temporaries
    /// This is maintained to ensure unique value IDs within the function
    pub(crate) next_value_id: u32,

    /// Type information for each value in the function
    /// Maps ValueId to its MirType for type checking and optimization
    pub value_types: FxHashMap<ValueId, MirType>,

    /// Track which ValueIds have been used as destinations
    /// Used to enforce SSA - each ValueId can only be defined once
    pub(crate) defined_values: FxHashSet<ValueId>,

    // ==================== SSA Construction State ====================
    // Based on Braun et al. "Simple and Efficient Construction of Static Single Assignment Form"
    /// currentDef[var][block] -> ValueId (Algorithm 1)
    /// Maps (variable, block) pairs to their current definition
    pub(crate) current_def: FxHashMap<(MirDefinitionId, BasicBlockId), ValueId>,

    /// incompletePhis[block][variable] -> ValueId (Algorithm 2)
    /// Maps blocks to incomplete phi nodes for each variable
    pub(crate) incomplete_phis: FxHashMap<BasicBlockId, FxHashMap<MirDefinitionId, ValueId>>,

    /// Track which blocks are sealed (sealedBlocks in paper)
    /// A block is sealed when no more predecessors will be added to it
    pub(crate) sealed_blocks: HashSet<BasicBlockId>,
}

impl MirFunction {
    /// Creates a new empty function with the given name
    pub fn new(name: String) -> Self {
        let mut basic_blocks = IndexVec::new();
        let entry_block = basic_blocks.push(BasicBlock::new());

        Self {
            name,
            locals: FxHashMap::default(),
            basic_blocks,
            entry_block,
            parameters: Vec::new(),
            return_values: Vec::new(),
            next_value_id: 0,
            value_types: FxHashMap::default(),
            defined_values: FxHashSet::default(),
            // Initialize SSA state
            current_def: FxHashMap::default(),
            incomplete_phis: FxHashMap::default(),
            sealed_blocks: HashSet::new(),
        }
    }

    /// Adds a new basic block and returns its ID
    pub fn add_basic_block(&mut self) -> BasicBlockId {
        self.basic_blocks.push(BasicBlock::new())
    }

    /// Adds a new basic block with a name and returns its ID
    pub fn add_basic_block_with_name(&mut self, name: String) -> BasicBlockId {
        let block = BasicBlock::with_name(name);
        self.basic_blocks.push(block)
    }

    /// Gets a basic block by ID
    pub fn get_basic_block(&self, id: BasicBlockId) -> Option<&BasicBlock> {
        self.basic_blocks.get(id)
    }

    /// Gets a mutable reference to a basic block by ID
    pub fn get_basic_block_mut(&mut self, id: BasicBlockId) -> Option<&mut BasicBlock> {
        self.basic_blocks.get_mut(id)
    }

    /// Generates a new unique value ID within this function
    pub fn new_value_id(&mut self) -> ValueId {
        let id = ValueId::new(self.next_value_id as usize);
        self.next_value_id += 1;
        id
    }

    /// Generates a new unique value ID with type information
    pub fn new_typed_value_id(&mut self, mir_type: MirType) -> ValueId {
        let id = self.new_value_id();
        self.value_types.insert(id, mir_type);
        id
    }

    /// Sets the type for a value ID
    pub fn set_value_type(&mut self, value_id: ValueId, mir_type: MirType) {
        self.value_types.insert(value_id, mir_type);
    }

    /// Gets the type for a value ID
    pub fn get_value_type(&self, value_id: ValueId) -> Option<&MirType> {
        self.value_types.get(&value_id)
    }

    /// Gets the type for a value ID, returning Unknown if not found
    pub fn get_value_type_or_unknown(&self, value_id: ValueId) -> MirType {
        self.value_types
            .get(&value_id)
            .cloned()
            .unwrap_or(MirType::unknown())
    }

    /// Marks a ValueId as defined, enforcing SSA form
    /// Returns an error if the ValueId has already been defined
    pub fn mark_as_defined(&mut self, dest: ValueId) -> Result<(), String> {
        if !self.defined_values.insert(dest) {
            return Err(format!(
                "SSA violation: ValueId {:?} is being defined multiple times",
                dest
            ));
        }
        Ok(())
    }

    /// Maps a semantic definition to a MIR value
    pub fn map_definition(&mut self, def_id: MirDefinitionId, value_id: ValueId) {
        self.locals.insert(def_id, value_id);
    }

    /// Looks up the MIR value for a semantic definition
    pub fn lookup_definition(&self, def_id: MirDefinitionId) -> Option<ValueId> {
        self.locals.get(&def_id).copied()
    }

    /// Returns an iterator over all basic blocks
    pub fn basic_blocks(&self) -> impl Iterator<Item = (BasicBlockId, &BasicBlock)> {
        self.basic_blocks.iter_enumerated()
    }

    /// Returns the number of basic blocks in this function
    pub fn block_count(&self) -> usize {
        self.basic_blocks.len()
    }

    /// Returns the number of local variables in this function
    pub fn local_count(&self) -> usize {
        self.locals.len()
    }

    /// Returns a map from each ValueId to its usage count in the function.
    /// This is useful for optimization passes like dead code elimination or instruction fusion.
    pub fn get_value_use_counts(&self) -> FxHashMap<ValueId, usize> {
        let mut counts = FxHashMap::default();
        for (_id, block) in self.basic_blocks() {
            for instruction in &block.instructions {
                for used_value in instruction.used_values() {
                    *counts.entry(used_value).or_default() += 1;
                }
            }
            for used_value in block.terminator.used_values() {
                *counts.entry(used_value).or_default() += 1;
            }
        }
        counts
    }

    /// Validates the function structure
    ///
    /// Checks:
    /// - Entry block exists and is valid
    /// - All basic blocks are properly terminated
    /// - All referenced blocks exist
    /// - No unreachable blocks (optional warning)
    pub fn validate(&self) -> Result<(), String> {
        // Check entry block exists
        if self.basic_blocks.get(self.entry_block).is_none() {
            return Err(format!("Entry block {:?} does not exist", self.entry_block));
        }

        // Validate each basic block
        for (block_id, block) in self.basic_blocks() {
            if let Err(err) = block.validate() {
                return Err(format!("Block {block_id:?} validation failed: {err}"));
            }

            // Check that terminator targets are valid
            for target in block.terminator.target_blocks() {
                if self.basic_blocks.get(target).is_none() {
                    return Err(format!(
                        "Block {block_id:?} targets non-existent block {target:?}"
                    ));
                }
            }

            // NEW: Validate phi instruction placement and consistency
            let mut seen_non_phi = false;
            for (i, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    crate::InstructionKind::Phi { dest, sources, ty } => {
                        if seen_non_phi {
                            return Err(format!(
                                "Block {:?}: Phi instruction at position {} found after non-phi instruction",
                                block_id, i
                            ));
                        }

                        // Check that destination is defined exactly once
                        if !self.defined_values.contains(dest) {
                            return Err(format!(
                                "Block {:?}: Phi instruction destination {:?} not in defined_values",
                                block_id, dest
                            ));
                        }

                        // Check that each source block is actually a predecessor
                        for (source_block, _value) in sources {
                            if !block.preds.contains(source_block) {
                                return Err(format!(
                                    "Block {:?}: Phi instruction has operand from block {:?} which is not a predecessor",
                                    block_id, source_block
                                ));
                            }
                        }

                        // Check that we have operands from all predecessors (if sealed)
                        if block.sealed && sources.len() != block.preds.len() {
                            return Err(format!(
                                "Block {:?}: Sealed block has {} predecessors but phi has {} operands",
                                block_id, block.preds.len(), sources.len()
                            ));
                        }

                        // Check that destination type matches phi type
                        if let Some(dest_type) = self.get_value_type(*dest) {
                            if dest_type != ty {
                                return Err(format!(
                                    "Block {:?}: Phi destination type {:?} doesn't match instruction type {:?}",
                                    block_id, dest_type, ty
                                ));
                            }
                        }
                    }
                    _ => {
                        seen_non_phi = true;
                    }
                }
            }
        }

        Ok(())
    }

    /// Checks if a basic block is reachable from the entry block
    ///
    /// This performs a depth-first search to determine reachability.
    /// Useful for dead code elimination and validation.
    pub fn is_block_reachable(&self, target: BasicBlockId) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![self.entry_block];

        while let Some(current) = stack.pop() {
            if current == target {
                return true;
            }

            if visited.insert(current)
                && let Some(block) = self.get_basic_block(current)
            {
                for successor in block.terminator.target_blocks() {
                    stack.push(successor);
                }
            }
        }

        false
    }

    /// Returns all unreachable basic blocks
    ///
    /// This is useful for optimization passes and validation warnings.
    pub fn unreachable_blocks(&self) -> Vec<BasicBlockId> {
        self.basic_blocks()
            .map(|(id, _)| id)
            .filter(|&id| !self.is_block_reachable(id))
            .collect()
    }

    /// Connect two blocks by adding pred/succ edges
    /// This is the canonical way to add CFG edges
    ///
    /// Note: Successors are now derived from terminators, so this method
    /// only maintains the predecessor list. The terminator of the predecessor
    /// block should be set separately to establish the actual control flow.
    pub fn connect(&mut self, pred: BasicBlockId, succ: BasicBlockId) {
        // Get mutable reference to successor block - panic if not found
        let succ_block = self
            .basic_blocks
            .get_mut(succ)
            .unwrap_or_else(|| panic!("Successor block {:?} does not exist", succ));
        succ_block.add_pred(pred);
    }

    /// Replace an edge from pred->old_succ with pred->new_succ
    /// Updates predecessor lists and expects terminator to be updated separately
    pub fn replace_edge(
        &mut self,
        pred: BasicBlockId,
        old_succ: BasicBlockId,
        new_succ: BasicBlockId,
    ) {
        // Get mutable reference to old successor block - panic if not found
        let old_succ_block = self
            .basic_blocks
            .get_mut(old_succ)
            .unwrap_or_else(|| panic!("Old successor block {:?} does not exist", old_succ));
        old_succ_block.remove_pred(pred);

        // Add new edge
        self.connect(pred, new_succ);
    }

    /// Disconnect two blocks by removing pred/succ edges
    /// Only removes from predecessor list; terminator should be updated separately
    pub fn disconnect(&mut self, pred: BasicBlockId, succ: BasicBlockId) {
        // Get mutable reference to successor block - panic if not found
        let succ_block = self
            .basic_blocks
            .get_mut(succ)
            .unwrap_or_else(|| panic!("Successor block {:?} does not exist", succ));
        succ_block.remove_pred(pred);
    }

    /// Replace all occurrences of `from` value with `to` value throughout the function
    /// This is needed for trivial phi elimination
    pub fn replace_all_uses(&mut self, from: ValueId, to: ValueId) {
        if from == to {
            return; // No-op
        }

        for i in 0..self.basic_blocks.len() {
            let block_id = BasicBlockId::from_raw(i);
            if let Some(block) = self.basic_blocks.get_mut(block_id) {
                // Replace in all instructions
                for instruction in &mut block.instructions {
                    instruction.replace_value_uses(from, to);
                }

                // Replace in terminator
                block.terminator.replace_value_uses(from, to);
            }
        }

        // Update parameter list if needed
        for param in &mut self.parameters {
            if *param == from {
                *param = to;
            }
        }

        // Update return values if needed
        for ret_val in &mut self.return_values {
            if *ret_val == from {
                *ret_val = to;
            }
        }

        // Remove the old value from type information
        if let Some(ty) = self.value_types.remove(&from) {
            // If `to` doesn't have a type, give it the type from `from`
            self.value_types.entry(to).or_insert(ty);
        }

        // Remove from defined_values
        self.defined_values.remove(&from);
    }

    /// Create a new phi instruction at the front of the given block
    /// Returns the destination ValueId
    pub fn new_phi(&mut self, block_id: BasicBlockId, ty: MirType) -> ValueId {
        let dest = self.new_typed_value_id(ty.clone());

        // Mark as defined for SSA validation
        self.mark_as_defined(dest)
            .expect("Phi destination should be unique");

        let phi_instr = Instruction::empty_phi(dest, ty);

        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.push_phi_front(phi_instr);
        }

        dest
    }

    /// Create a phi instruction with specific operands
    pub fn new_phi_with_operands(
        &mut self,
        block_id: BasicBlockId,
        ty: MirType,
        operands: Vec<(BasicBlockId, Value)>,
    ) -> ValueId {
        let dest = self.new_typed_value_id(ty.clone());

        // Mark as defined for SSA validation
        self.mark_as_defined(dest)
            .expect("Phi destination should be unique");

        let phi_instr = Instruction::phi(dest, ty, operands);

        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.push_phi_front(phi_instr);
        }

        dest
    }

    // ==================== SSA Construction Methods ====================
    // Based on Braun et al. "Simple and Efficient Construction of Static Single Assignment Form"

    /// Write a variable in a block (Algorithm 1, writeVariable)
    pub fn write_variable(&mut self, var: MirDefinitionId, block: BasicBlockId, value: ValueId) {
        self.current_def.insert((var, block), value);
        // Also update the locals map for compatibility
        self.locals.insert(var, value);
    }

    /// Read a variable from a block (Algorithm 1, readVariable)
    pub fn read_variable(&mut self, var: MirDefinitionId, block: BasicBlockId) -> ValueId {
        if let Some(&value) = self.current_def.get(&(var, block)) {
            return value;
        }

        // Variable not defined locally, use recursive algorithm
        self.read_variable_recursive(var, block)
    }

    /// Recursive variable reading (Algorithm 2, readVariableRecursive)
    fn read_variable_recursive(&mut self, var: MirDefinitionId, block: BasicBlockId) -> ValueId {
        let val = if !self.sealed_blocks.contains(&block) {
            // Incomplete CFG: val ← new Phi(block)
            let val = self.new_incomplete_phi(block, var);
            // incompletePhis[block][variable] ← val
            self.incomplete_phis
                .entry(block)
                .or_default()
                .insert(var, val);
            val
        } else if self.basic_blocks[block].preds.len() == 1 {
            // Optimize the common case of one predecessor: No phi needed
            // val ← readVariable(variable, block.preds[0])
            let pred = self.basic_blocks[block].preds[0];

            self.read_variable(var, pred)
        } else {
            // Break potential cycles with operandless phi
            // val ← new Phi(block)
            let val = self.new_incomplete_phi(block, var);
            // writeVariable(variable, block, val)
            self.write_variable(var, block, val);
            // val ← addPhiOperands(variable, val)
            self.add_phi_operands(var, val)
        };
        self.write_variable(var, block, val);
        val
    }

    /// Create a new incomplete phi instruction (helper for both complete and incomplete phis)
    fn new_incomplete_phi(&mut self, block: BasicBlockId, var: MirDefinitionId) -> ValueId {
        let value_id = self.locals.get(&var).expect("Variable must be defined");
        let var_type = self
            .get_value_type(*value_id)
            .expect("Variable must have a type");
        self.new_phi(block, var_type.clone())
    }

    /// Add phi operands from all predecessors (Algorithm 2, addPhiOperands)
    fn add_phi_operands(&mut self, var: MirDefinitionId, phi: ValueId) -> ValueId {
        // Get the block containing this phi
        let phi_block = self.find_phi_block(phi).expect("Phi must exist in a block");

        // Determine operands from predecessors - build explicit block-value pairs
        let preds = self.basic_blocks[phi_block].preds.clone();
        let mut block_value_pairs = Vec::new();

        for &pred in &preds {
            // Recursively read variable from predecessor
            let operand = self.read_variable(var, pred);
            block_value_pairs.push((pred, operand));
        }

        // Update the phi instruction with operands
        self.update_phi_operands(phi_block, phi, block_value_pairs);

        // Try trivial phi elimination
        self.try_remove_trivial_phi(phi, var)
    }

    /// Find which block contains a phi instruction
    fn find_phi_block(&self, phi: ValueId) -> Option<BasicBlockId> {
        for (block_id, block) in self.basic_blocks.iter_enumerated() {
            for instr in &block.instructions {
                if let crate::InstructionKind::Phi { dest, .. } = &instr.kind {
                    if *dest == phi {
                        return Some(block_id);
                    }
                }
            }
        }
        None
    }

    /// Update phi instruction operands
    fn update_phi_operands(
        &mut self,
        block: BasicBlockId,
        phi: ValueId,
        block_value_pairs: Vec<(BasicBlockId, ValueId)>,
    ) {
        if let Some(block_ref) = self.basic_blocks.get_mut(block) {
            for instr in &mut block_ref.instructions {
                if let crate::InstructionKind::Phi {
                    dest,
                    sources: phi_sources,
                    ..
                } = &mut instr.kind
                {
                    if *dest == phi {
                        // Convert Vec<(BasicBlockId, ValueId)> to Vec<(BasicBlockId, Value)>
                        *phi_sources = block_value_pairs
                            .into_iter()
                            .map(|(pred_block, value_id)| (pred_block, Value::operand(value_id)))
                            .collect();
                        return;
                    }
                }
            }
        }
    }

    /// Try to eliminate trivial phi (Algorithm 3, tryRemoveTrivialPhi)
    fn try_remove_trivial_phi(&mut self, phi: ValueId, var: MirDefinitionId) -> ValueId {
        // Get phi operands
        let phi_block = self.find_phi_block(phi).expect("Phi must exist");
        let operands = self.get_phi_operands(phi_block, phi);

        // Check if phi is trivial (all operands are the same non-phi value)
        let mut unique = None;
        for op in &operands {
            if *op == phi {
                continue; // Ignore self-references
            }
            if unique.is_none() {
                unique = Some(*op);
            } else if unique != Some(*op) {
                return phi; // Not trivial - has multiple different operands
            }
        }

        let Some(replacement) = unique else {
            return phi; // Only self-references, keep phi
        };

        // Phi is trivial - replace all uses with the unique operand
        self.replace_value_uses(phi, replacement);

        // Remove phi instruction
        self.remove_phi_instruction(phi_block, phi);

        replacement
    }

    /// Get operands of a phi instruction
    fn get_phi_operands(&self, block: BasicBlockId, phi: ValueId) -> Vec<ValueId> {
        if let Some(block) = self.basic_blocks.get(block) {
            for instr in &block.instructions {
                if let crate::InstructionKind::Phi { dest, sources, .. } = &instr.kind {
                    if *dest == phi {
                        return sources
                            .iter()
                            .map(|(_, v)| {
                                if let Value::Operand(id) = v {
                                    *id
                                } else {
                                    panic!("Expected operand in phi")
                                }
                            })
                            .collect();
                    }
                }
            }
        }
        vec![]
    }

    /// Replace all uses of old_value with new_value
    fn replace_value_uses(&mut self, old_value: ValueId, new_value: ValueId) {
        for block in &mut self.basic_blocks {
            // Replace in instructions
            for instr in &mut block.instructions {
                instr.replace_value_uses(old_value, new_value);
            }
            // Replace in terminator
            block.terminator.replace_value_uses(old_value, new_value);
        }
    }

    /// Remove a phi instruction from a block
    fn remove_phi_instruction(&mut self, block: BasicBlockId, phi: ValueId) {
        if let Some(block) = self.basic_blocks.get_mut(block) {
            block.instructions.retain(|instr| {
                !matches!(&instr.kind, crate::InstructionKind::Phi { dest, .. } if *dest == phi)
            });
        }
    }

    /// Seal a block (Algorithm 2, sealBlock)
    /// This is called when no more predecessors will be added to the block
    pub fn seal_block(&mut self, block: BasicBlockId) {
        // Add to sealed blocks
        self.sealed_blocks.insert(block);

        // Complete all incomplete phis for this block
        if let Some(phis) = self.incomplete_phis.remove(&block) {
            for (var, phi) in phis {
                let completed_phi = self.add_phi_operands(var, phi);
                self.write_variable(var, block, completed_phi);
            }
        }
    }

    /// Check if a block is sealed
    pub fn is_block_sealed(&self, block: BasicBlockId) -> bool {
        self.sealed_blocks.contains(&block)
    }

    /// Set terminator while properly maintaining CFG edges
    /// This is a helper for optimization passes that need to change control flow
    pub fn set_terminator_with_edges(&mut self, block_id: BasicBlockId, new_term: Terminator) {
        // Get old target blocks
        let old_targets = if let Some(block) = self.basic_blocks.get(block_id) {
            block.terminator.target_blocks()
        } else {
            return; // Block doesn't exist
        };

        // Disconnect from old targets
        for target in old_targets {
            self.disconnect(block_id, target);
        }

        // Set new terminator
        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.set_terminator(new_term.clone());
        }

        // Connect to new targets
        for target in new_term.target_blocks() {
            self.connect(block_id, target);
        }
    }
}

impl PrettyPrint for MirFunction {
    fn pretty_print(&self, indent: usize) -> String {
        let mut result = String::new();
        let base_indent = indent_str(indent);

        result.push_str(&format!("{}fn {} {{\n", base_indent, self.name));

        // Print parameters
        if !self.parameters.is_empty() {
            result.push_str(&format!(
                "{}  parameters: {:?}\n",
                base_indent, self.parameters
            ));
        }

        // // Print locals mapping
        // if !self.locals.is_empty() {
        //     result.push_str(&format!("{base_indent}  locals: {{\n"));
        //     for (def_id, value_id) in &self.locals {
        //         result.push_str(&format!("{base_indent}    {def_id:?} -> {value_id:?}\n"));
        //     }
        //     result.push_str(&format!("{base_indent}  }}\n"));
        // }

        result.push_str(&format!(
            "{}  entry: {entry:?}\n",
            base_indent,
            entry = self.entry_block
        ));
        result.push('\n');

        // Print basic blocks
        for (block_id, block) in self.basic_blocks() {
            let block_display = if let Some(ref name) = block.name {
                format!("{block_id:?} ({name})")
            } else {
                format!("{block_id:?}")
            };
            result.push_str(&format!("{base_indent}  {block_display}:\n"));
            result.push_str(&block.pretty_print(indent + 2));
            result.push('\n');
        }

        result.push_str(&format!("{base_indent}}}\n"));
        result
    }
}

#[cfg(test)]
#[path = "function_tests.rs"]
mod tests;
