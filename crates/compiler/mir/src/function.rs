//! # MIR Function
//!
//! This module defines the function-level MIR representation, including
//! the Control Flow Graph (CFG) of basic blocks.

use index_vec::IndexVec;
use rustc_hash::FxHashMap;

use crate::{indent_str, BasicBlock, BasicBlockId, MirType, PrettyPrint, ValueId};

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

    /// Maps semantic variable definitions to their MIR value representation
    /// This preserves the connection between semantic analysis and MIR
    pub locals: FxHashMap<MirDefinitionId, ValueId>,

    /// All basic blocks in this function, forming the CFG
    pub basic_blocks: IndexVec<BasicBlockId, BasicBlock>,

    /// The entry point of the function (always valid if function has blocks)
    pub entry_block: BasicBlockId,

    /// Function parameters mapped to their MIR values
    /// The order matches the function signature
    pub parameters: Vec<ValueId>,

    /// The return value, if the function returns something
    /// TODO: add support for multiple return values
    pub return_value: Option<ValueId>,

    /// Next available value ID for generating new temporaries
    /// This is maintained to ensure unique value IDs within the function
    pub(crate) next_value_id: u32,

    /// Type information for each value in the function
    /// Maps ValueId to its MirType for type checking and optimization
    pub value_types: FxHashMap<ValueId, MirType>,
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
            return_value: None,
            next_value_id: 0,
            value_types: FxHashMap::default(),
        }
    }

    /// Adds a new basic block and returns its ID
    pub fn add_basic_block(&mut self) -> BasicBlockId {
        self.basic_blocks.push(BasicBlock::new())
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

        // Print locals mapping
        if !self.locals.is_empty() {
            result.push_str(&format!("{base_indent}  locals: {{\n"));
            for (def_id, value_id) in &self.locals {
                result.push_str(&format!("{base_indent}    {def_id:?} -> {value_id:?}\n"));
            }
            result.push_str(&format!("{base_indent}  }}\n"));
        }

        result.push_str(&format!(
            "{}  entry: {entry:?}\n",
            base_indent,
            entry = self.entry_block
        ));
        result.push('\n');

        // Print basic blocks
        for (block_id, block) in self.basic_blocks() {
            result.push_str(&format!("{base_indent}  {block_id:?}:\n"));
            result.push_str(&block.pretty_print(indent + 2));
            result.push('\n');
        }

        result.push_str(&format!("{base_indent}}}\n"));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Terminator, Value};

    #[test]
    fn test_return_value_field_with_literal() {
        let mut func = MirFunction::new("test".to_string());

        // Create a return value assignment
        let return_value_id = func.new_value_id();
        func.return_value = Some(return_value_id);

        // Set up the terminator
        func.get_basic_block_mut(func.entry_block)
            .unwrap()
            .set_terminator(Terminator::return_value(Value::integer(42)));

        // Verify the return_value field is set
        assert_eq!(func.return_value, Some(return_value_id));

        // Verify function validation passes
        assert!(func.validate().is_ok());
    }

    #[test]
    fn test_return_value_field_with_operand() {
        let mut func = MirFunction::new("test".to_string());

        // Create a value to return
        let value_id = func.new_value_id();
        func.return_value = Some(value_id);

        // Set up the terminator
        func.get_basic_block_mut(func.entry_block)
            .unwrap()
            .set_terminator(Terminator::return_value(Value::operand(value_id)));

        // Verify the return_value field is set
        assert_eq!(func.return_value, Some(value_id));

        // Verify function validation passes
        assert!(func.validate().is_ok());
    }
}
