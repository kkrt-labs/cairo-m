//a # Cairo-M Intermediate Representation (MIR)
//!
//! This crate defines the data structures for the Mid-level Intermediate Representation
//! of the Cairo-M compiler. The MIR is a high-level, platform-independent representation
//! of the program that is generated from the semantic AST and is used for optimizations
//! and code generation.
//!
//! ## Design Principles
//!
//! The design is inspired by LLVM IR and is based on:
//!
//! 1. **Control Flow Graph (CFG)**: Functions are represented as directed graphs of basic blocks
//! 2. **Three-Address Code (TAC)**: Instructions are simple, atomic operations with at most one operation
//! 3. **Static Single Assignment (SSA)**: Each virtual register is assigned exactly once (simplified form)
//! 4. **Explicit Control Flow**: All control flow is explicit through terminators
//!
//! ## Architecture
//!
//! ```text
//! MirModule
//! functions: IndexVec<FunctionId, MirFunction>
//! ...
//!
//! MirFunction
//! basic_blocks: IndexVec<BasicBlockId, BasicBlock>
//! locals: Map<DefinitionId, ValueId>
//! entry_block: BasicBlockId
//!
//! BasicBlock
//! instructions: Vec<Instruction>
//! terminator: Terminator
//! ```
//!
//! ## Integration with Semantic Analysis
//!
//! The MIR integrates closely with the semantic analysis phase:
//! - Uses `DefinitionId` from semantic analysis for variable mapping
//! - Preserves source location information for diagnostics
//! - Leverages semantic type information for accurate lowering
//!
//! ## Error Handling
//!
//! The MIR supports graceful error recovery:
//! - Generates partial MIR even with semantic errors
//! - Uses placeholder values for unresolved references
//! - Maintains diagnostic source mapping

#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

pub use basic_block::BasicBlock;
pub use builder::{CfgBuilder, CfgState, InstrBuilder};
pub use function::{MirDefinitionId, MirFunction};
pub use instruction::{
    AccessPath, BinaryOp, FieldPath, Instruction, InstructionKind, MirExpressionId,
};
pub use layout::DataLayout;
pub use mir_types::MirType;
pub use module::MirModule;
pub use passes::{DeadCodeElimination, FuseCmpBranch, MirPass, PassManager, SroaPass, Validation};
pub use terminator::Terminator;
pub use value::{Literal, Place, Value};

pub mod analysis;
pub mod backend;
pub mod basic_block;
pub mod builder;
pub mod cfg;
pub mod db;
pub mod function;
pub mod instruction;
pub mod layout;
pub mod lowering;
pub mod mir_types;
pub mod module;
pub mod passes;
pub mod pipeline;
pub mod terminator;
pub mod value;

pub use db::{generate_mir as db_generate_mir, MirDb};
pub use lowering::generate_mir;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod instruction_tests;

// --- Core Identifiers ---

index_vec::define_index_type! {
    /// Unique identifier for a function within a MIR module
    pub struct FunctionId = usize;
}

index_vec::define_index_type! {
    /// Unique identifier for a basic block within a function
    pub struct BasicBlockId = usize;
}

index_vec::define_index_type! {
    /// Unique identifier for a value (virtual register) within a function
    pub struct ValueId = usize;
}

// --- Pretty Printing Support ---

/// Trait for pretty-printing MIR constructs
pub trait PrettyPrint {
    fn pretty_print(&self, indent: usize) -> String;
}

/// Helper function to create indentation
pub(crate) fn indent_str(level: usize) -> String {
    "  ".repeat(level)
}
