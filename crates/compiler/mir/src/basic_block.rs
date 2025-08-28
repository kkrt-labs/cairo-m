//! # MIR Basic Block
//!
//! This module defines basic blocks, the fundamental building blocks of the CFG.
//! A basic block is a straight-line sequence of instructions with exactly one entry
//! point and one exit point.

use crate::{indent_str, BasicBlockId, Instruction, PrettyPrint, Terminator};

/// A basic block in the Control Flow Graph
///
/// A basic block represents a straight-line sequence of instructions that:
/// - Has exactly one entry point (the first instruction)
/// - Has exactly one exit point (the terminator)
/// - Contains no jumps or branches except at the end
/// - Is atomic for control flow analysis
///
/// # Invariants
///
/// - Every basic block must have exactly one terminator
/// - Instructions within a block execute sequentially
/// - Control can only enter at the beginning and exit at the end
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// Optional name for debugging purposes
    pub name: Option<String>,

    /// The sequence of instructions in this block
    /// These execute sequentially without any control flow changes
    pub instructions: Vec<Instruction>,

    /// The terminator that ends this block and transfers control
    /// Every basic block must have exactly one terminator
    pub terminator: Terminator,

    /// Explicit CFG edges - predecessors of this block
    pub preds: Vec<BasicBlockId>,

    /// SSA construction state - predecessor set is final (Braun et al.)
    pub sealed: bool,

    /// SSA construction state - all local statements processed (Braun §2.1)
    pub filled: bool,
}

impl BasicBlock {
    /// Creates a new empty basic block with an unreachable terminator
    ///
    /// The unreachable terminator serves as a placeholder until the real
    /// terminator is set during MIR construction.
    pub const fn new() -> Self {
        Self {
            name: None,
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            preds: Vec::new(),
            sealed: false,
            filled: false,
        }
    }

    /// Creates a new basic block with a name and an unreachable terminator
    pub const fn with_name(name: String) -> Self {
        Self {
            name: Some(name),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            preds: Vec::new(),
            sealed: false,
            filled: false,
        }
    }

    /// Creates a new basic block with the given terminator
    pub const fn with_terminator(terminator: Terminator) -> Self {
        Self {
            name: None,
            instructions: Vec::new(),
            terminator,
            preds: Vec::new(),
            sealed: false,
            filled: false,
        }
    }

    /// Adds an instruction to the end of this block
    pub fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Sets the terminator for this block
    pub fn set_terminator(&mut self, terminator: Terminator) {
        self.terminator = terminator;
    }

    /// Returns true if this block has a meaningful terminator
    ///
    /// A block is considered properly terminated if it has any terminator
    /// other than `Unreachable`, which is used as a placeholder.
    pub const fn has_terminator(&self) -> bool {
        !matches!(self.terminator, Terminator::Unreachable)
    }

    /// Returns the number of instructions in this block
    pub const fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Returns true if this block is empty (no instructions)
    pub const fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Returns the terminator of this block
    pub const fn terminator(&self) -> &Terminator {
        &self.terminator
    }

    /// Validates the basic block structure
    ///
    /// Checks:
    /// - Block has a valid terminator (not placeholder)
    /// - Instructions are well-formed
    /// - No obvious inconsistencies
    pub(crate) fn validate(&self) -> Result<(), String> {
        // Note: We allow Unreachable terminators during construction,
        // but they should be replaced before final validation

        // Validate each instruction
        for (i, instruction) in self.instructions.iter().enumerate() {
            if let Err(err) = instruction.validate() {
                return Err(format!("Instruction {i} validation failed: {err}"));
            }
        }

        // Validate terminator
        if let Err(err) = self.terminator.validate() {
            return Err(format!("Terminator validation failed: {err}"));
        }

        Ok(())
    }

    /// Add a predecessor, avoiding duplicates
    pub(crate) fn add_pred(&mut self, pred: BasicBlockId) {
        if !self.preds.contains(&pred) {
            self.preds.push(pred);
        }
    }

    /// Remove a predecessor
    pub(crate) fn remove_pred(&mut self, pred: BasicBlockId) {
        self.preds.retain(|&p| p != pred);
    }

    /// Mark this block as sealed (no more predecessors will be added)
    pub const fn seal(&mut self) {
        self.sealed = true;
    }

    /// Mark this block as filled (all local statements processed)
    pub const fn mark_filled(&mut self) {
        self.filled = true;
    }

    /// Insert a phi instruction at the front of the block (after existing phis)
    /// Maintains the invariant that all phi instructions come first
    pub fn push_phi_front(&mut self, instruction: Instruction) {
        // Verify this is actually a phi instruction
        if !matches!(instruction.kind, crate::InstructionKind::Phi { .. }) {
            panic!("push_phi_front called with non-phi instruction");
        }

        // Find insertion point (after existing phis)
        let insert_pos = self
            .instructions
            .iter()
            .position(|instr| !matches!(instr.kind, crate::InstructionKind::Phi { .. }))
            .unwrap_or(self.instructions.len());

        self.instructions.insert(insert_pos, instruction);
    }

    /// Get the range of phi instructions at the start of this block
    pub(crate) fn phi_range(&self) -> std::ops::Range<usize> {
        let end = self
            .instructions
            .iter()
            .position(|instr| !matches!(instr.kind, crate::InstructionKind::Phi { .. }))
            .unwrap_or(self.instructions.len());
        0..end
    }

    /// Get all phi instructions in this block
    pub(crate) fn phi_instructions(&self) -> impl Iterator<Item = &Instruction> {
        let range = self.phi_range();
        self.instructions[range].iter()
    }

    /// Get all phi instructions mutably
    pub fn phi_instructions_mut(&mut self) -> impl Iterator<Item = &mut Instruction> {
        let range = self.phi_range();
        self.instructions[range].iter_mut()
    }

    /// Get all non-phi instructions in this block
    pub(crate) fn non_phi_instructions(&self) -> impl Iterator<Item = &Instruction> {
        let range = self.phi_range();
        self.instructions[range.end..].iter()
    }
}

impl Default for BasicBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl PrettyPrint for BasicBlock {
    fn pretty_print(&self, indent: usize) -> String {
        let mut result = String::new();
        let base_indent = indent_str(indent);

        // Print block name if available
        if let Some(ref name) = self.name {
            result.push_str(&format!("{}; {}\n", base_indent, name));
        }

        // Print instructions
        for instruction in &self.instructions {
            result.push_str(&format!("{}{}\n", base_indent, instruction.pretty_print(0)));
        }

        // Print terminator
        result.push_str(&format!(
            "{}{}\n",
            base_indent,
            self.terminator.pretty_print(0)
        ));

        result
    }
}
