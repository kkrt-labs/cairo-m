//! # Cairo-M Intermediate Representation (MIR)
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

// Re-export commonly used types from submodules
pub use basic_block::BasicBlock;
use chumsky::span::SimpleSpan;
pub use function::{MirDefinitionId, MirFunction};
pub use instruction::{Instruction, InstructionKind, MirExpressionId};
pub use mir_types::{MirType, StructField};
pub use module::MirModule;
pub use passes::{DeadCodeElimination, MirPass, PassManager, Validation};
pub use terminator::Terminator;
pub use value::{Literal, Value};

pub mod basic_block;
pub mod db;
pub mod function;
pub mod instruction;
pub mod ir_generation;
pub mod mir_types;
pub mod module;
pub mod passes;
pub mod terminator;
pub mod value;

// Re-export the main IR generation function
// Re-export database traits and functions
pub use db::{MirDb, generate_mir as db_generate_mir};
pub use ir_generation::generate_mir;

#[cfg(test)]
pub mod testing;

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

// --- Error Types ---

/// Represents an error in MIR construction or validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirError {
    /// A semantic error that prevents MIR generation
    SemanticError {
        message: String,
        span: Option<SimpleSpan<usize>>,
    },
    /// An unresolved reference (forward declaration, missing import, etc.)
    UnresolvedReference {
        name: String,
        span: Option<SimpleSpan<usize>>,
    },
    /// Invalid MIR structure (validation error)
    ValidationError {
        message: String,
        function_id: Option<FunctionId>,
        block_id: Option<BasicBlockId>,
    },
}

impl std::fmt::Display for MirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemanticError { message, span } => {
                write!(f, "Semantic error: {message}")?;
                if let Some(span) = span {
                    write!(f, " at {span:?}")?;
                }
                Ok(())
            }
            Self::UnresolvedReference { name, span } => {
                write!(f, "Unresolved reference: {name}")?;
                if let Some(span) = span {
                    write!(f, " at {span:?}")?;
                }
                Ok(())
            }
            Self::ValidationError {
                message,
                function_id,
                block_id,
            } => {
                write!(f, "Validation error: {message}")?;
                if let Some(func_id) = function_id {
                    write!(f, " in function {func_id:?}")?;
                }
                if let Some(block_id) = block_id {
                    write!(f, " in block {block_id:?}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for MirError {}

/// Result type for MIR operations
pub type MirResult<T> = Result<T, MirError>;

// --- Pretty Printing Support ---

/// Trait for pretty-printing MIR constructs
pub trait PrettyPrint {
    fn pretty_print(&self, indent: usize) -> String;
}

/// Helper function to create indentation
pub(crate) fn indent_str(level: usize) -> String {
    "  ".repeat(level)
}
