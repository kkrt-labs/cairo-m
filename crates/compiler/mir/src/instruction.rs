//! # MIR Instructions
//!
//! This module defines the instruction types for MIR.
//! Instructions perform computations but do not transfer control flow.

use std::collections::HashSet;

use cairo_m_compiler_parser::parser::UnaryOp;
use chumsky::span::SimpleSpan;

use crate::{MirType, PrettyPrint, Value, ValueId};

/// Binary operators supported in MIR
///
/// This enum includes both generic operators (for felt types) and
/// type-specific operators (for u32 types). The MIR generation phase
/// selects the appropriate operator based on operand types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Felt arithmetic operators
    Add,
    Sub,
    Mul,
    Div,

    // Felt comparison operators
    Eq,
    Neq,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // Logical operators (work on felt/bool)
    And,
    Or,

    // U32 arithmetic operators
    U32Add,
    U32Sub,
    U32Mul,
    U32Div,

    // U32 comparison operators
    U32Eq,
    U32Neq,
    U32Less,
    U32Greater,
    U32LessEqual,
    U32GreaterEqual,
}

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Eq => write!(f, "=="),
            Self::Neq => write!(f, "!="),
            Self::Less => write!(f, "<"),
            Self::Greater => write!(f, ">"),
            Self::LessEqual => write!(f, "<="),
            Self::GreaterEqual => write!(f, ">="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::U32Add => write!(f, "U32Add"),
            Self::U32Sub => write!(f, "U32Sub"),
            Self::U32Mul => write!(f, "U32Mul"),
            Self::U32Div => write!(f, "U32Div"),
            Self::U32Eq => write!(f, "U32Eq"),
            Self::U32Neq => write!(f, "U32Neq"),
            Self::U32Less => write!(f, "U32Less"),
            Self::U32Greater => write!(f, "U32Greater"),
            Self::U32LessEqual => write!(f, "U32LessEqual"),
            Self::U32GreaterEqual => write!(f, "U32GreaterEqual"),
        }
    }
}

impl BinaryOp {
    /// Convert from parser op based on operand type
    pub fn from_parser(
        op: cairo_m_compiler_parser::parser::BinaryOp,
        operand_type: &cairo_m_compiler_semantic::types::TypeData,
    ) -> Result<Self, String> {
        use cairo_m_compiler_parser::parser::BinaryOp as P;
        use cairo_m_compiler_semantic::types::TypeData as T;

        let mir_op = match (op, operand_type) {
            // Felt operations
            (P::Add, T::Felt) => Self::Add,
            (P::Sub, T::Felt) => Self::Sub,
            (P::Mul, T::Felt) => Self::Mul,
            (P::Div, T::Felt) => Self::Div,
            (P::Eq, T::Felt) => Self::Eq,
            (P::Neq, T::Felt) => Self::Neq,
            (P::Less, T::Felt) => Self::Less,
            (P::Greater, T::Felt) => Self::Greater,
            (P::LessEqual, T::Felt) => Self::LessEqual,
            (P::GreaterEqual, T::Felt) => Self::GreaterEqual,

            // U32 operations
            (P::Add, T::U32) => Self::U32Add,
            (P::Sub, T::U32) => Self::U32Sub,
            (P::Mul, T::U32) => Self::U32Mul,
            (P::Div, T::U32) => Self::U32Div,
            (P::Eq, T::U32) => Self::U32Eq,
            (P::Neq, T::U32) => Self::U32Neq,
            (P::Less, T::U32) => Self::U32Less,
            (P::Greater, T::U32) => Self::U32Greater,
            (P::LessEqual, T::U32) => Self::U32LessEqual,
            (P::GreaterEqual, T::U32) => Self::U32GreaterEqual,

            // Bool operations
            (P::Eq, T::Bool) => Self::Eq,
            (P::Neq, T::Bool) => Self::Neq,
            (P::And, T::Bool) => Self::And,
            (P::Or, T::Bool) => Self::Or,

            _ => {
                return Err(format!(
                    "Unsupported binary op {:?} for type {:?}",
                    op, operand_type
                ))
            }
        };

        Ok(mir_op)
    }

    /// Get the result type of this operation
    pub const fn result_type(&self) -> crate::MirType {
        match self {
            // Arithmetic ops return same type
            Self::Add | Self::Sub | Self::Mul | Self::Div => crate::MirType::felt(),
            Self::U32Add | Self::U32Sub | Self::U32Mul | Self::U32Div => crate::MirType::u32(),

            // Comparison ops return bool
            Self::Eq
            | Self::Neq
            | Self::Less
            | Self::Greater
            | Self::LessEqual
            | Self::GreaterEqual => crate::MirType::bool(),

            Self::U32Eq
            | Self::U32Neq
            | Self::U32Less
            | Self::U32Greater
            | Self::U32LessEqual
            | Self::U32GreaterEqual => crate::MirType::bool(),

            // Logical ops
            Self::And | Self::Or => crate::MirType::bool(),
        }
    }
}

/// Simple expression identifier for MIR that doesn't depend on Salsa lifetimes
///
/// This is derived from semantic `ExpressionId` but simplified for use in MIR.
/// It allows MIR to reference semantic expressions without database dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirExpressionId {
    /// Index of the expression within its file
    pub expression_index: usize,
    /// A simple file identifier
    pub file_id: u64,
}

/// Represents the signature of a called function
///
/// This struct contains the parameter and return types of a function being called,
/// allowing the code generator to handle argument passing and return value allocation
/// without needing to look up the callee's information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalleeSignature {
    pub param_types: Vec<MirType>,
    pub return_types: Vec<MirType>,
}

/// An instruction performs an operation but does NOT transfer control
///
/// Instructions always fall through to the next instruction in the block.
/// Control flow changes are handled exclusively by terminators.
///
/// # Design Notes
///
/// - All instructions follow three-address code (TAC) format
/// - Each instruction has at most one operation
/// - Instructions can define at most one value
/// - Source location is preserved for diagnostics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The kind of instruction and its operands
    pub kind: InstructionKind,

    /// Source location for diagnostics and debugging
    pub source_span: Option<SimpleSpan<usize>>,

    /// Original expression ID from semantic analysis
    /// Used for type queries and cross-referencing
    pub source_expr_id: Option<MirExpressionId>,

    /// Optional comment for debugging
    pub comment: Option<String>,
}

/// The different kinds of instructions available in MIR
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    /// Simple assignment: `dest = source`
    /// Used for variable assignments and copies
    Assign { dest: ValueId, source: Value },

    /// U32 assignment: `dest = source`
    /// Used for u32 variable assignments and copies
    AssignU32 { dest: ValueId, source: Value },

    /// Unary operation: `dest = op source`
    /// Used for unary operations like negation and logical not
    UnaryOp {
        op: UnaryOp,
        dest: ValueId,
        source: Value,
        /// If Some, indicates this operation should write its result
        /// directly to the memory location represented by the given ValueId.
        /// This is an optimization hint for the code generator.
        in_place_target: Option<ValueId>,
    },

    /// Binary operation: `dest = left op right`
    /// Covers arithmetic, comparison, and logical operations
    BinaryOp {
        op: BinaryOp,
        dest: ValueId,
        left: Value,
        right: Value,
        /// If Some, indicates this operation should write its result
        /// directly to the memory location represented by the given ValueId.
        /// This is an optimization hint for the code generator.
        in_place_target: Option<ValueId>,
    },

    /// Function call: `dests = call callee(args)`
    /// For calling functions that return one or more values
    Call {
        dests: Vec<ValueId>,
        callee: crate::FunctionId,
        args: Vec<Value>,
        signature: CalleeSignature,
    },

    /// Void function call: `call callee(args)`
    /// For calling functions that don't return a value
    VoidCall {
        callee: crate::FunctionId,
        args: Vec<Value>,
        signature: CalleeSignature,
    },

    /// Load from memory: `dest = load addr`
    /// For accessing memory locations and dereferencing pointers
    Load { dest: ValueId, address: Value },

    /// Load from memory: `[dest, dest+1] = load [addr, addr+1]`
    /// For accessing memory locations and dereferencing pointers
    LoadU32 { dest: ValueId, address: Value },

    /// Store to memory: `store addr, value`
    /// For writing to memory locations
    Store { address: Value, value: Value },

    /// Store to memory: `store [addr, addr+1], [value_low, value_high]`
    /// For writing to memory locations
    StoreU32 { address: Value, value: Value },

    /// Allocate space on the stack: `dest = stackalloc size`
    /// For allocating local variables and temporary storage
    StackAlloc {
        dest: ValueId,
        size: usize,
        // TODO: Add alignment information when needed
    },

    /// Get address of a value: `dest = &operand`
    /// For taking addresses of variables
    AddressOf { dest: ValueId, operand: ValueId },

    /// Get element pointer: `dest = getelementptr base, offset`
    /// Calculates memory address based on base pointer and offset
    /// Similar to LLVM's GEP instruction for struct member and array index access
    GetElementPtr {
        dest: ValueId,
        base: Value,
        offset: Value,
    },

    /// Cast/conversion: `dest = cast value as type`
    /// For type conversions (to be expanded with type system)
    Cast {
        dest: ValueId,
        source: Value,
        // target_type: TypeId, // TODO: Add when type system is integrated
    },

    /// Debug/diagnostic instruction
    /// Used for debugging and diagnostic output
    Debug { message: String, values: Vec<Value> },
}

impl Instruction {
    /// Creates a new assignment instruction
    pub const fn assign(dest: ValueId, source: Value) -> Self {
        Self {
            kind: InstructionKind::Assign { dest, source },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new u32 assignment instruction
    pub const fn assign_u32(dest: ValueId, source: Value) -> Self {
        Self {
            kind: InstructionKind::AssignU32 { dest, source },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new unary operation instruction
    pub const fn unary_op(op: UnaryOp, dest: ValueId, source: Value) -> Self {
        Self {
            kind: InstructionKind::UnaryOp {
                op,
                dest,
                source,
                in_place_target: None,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new binary operation instruction
    pub const fn binary_op(op: BinaryOp, dest: ValueId, left: Value, right: Value) -> Self {
        Self {
            kind: InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
                in_place_target: None,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new load instruction
    pub const fn load(dest: ValueId, address: Value) -> Self {
        Self {
            kind: InstructionKind::Load { dest, address },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    pub const fn load_u32(dest: ValueId, address: Value) -> Self {
        Self {
            kind: InstructionKind::LoadU32 { dest, address },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new store instruction
    pub const fn store(address: Value, value: Value) -> Self {
        Self {
            kind: InstructionKind::Store { address, value },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    pub const fn store_u32(address: Value, value: Value) -> Self {
        Self {
            kind: InstructionKind::StoreU32 { address, value },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new stack allocation instruction
    pub const fn stack_alloc(dest: ValueId, size: usize) -> Self {
        Self {
            kind: InstructionKind::StackAlloc { dest, size },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new address-of instruction
    pub const fn address_of(dest: ValueId, operand: ValueId) -> Self {
        Self {
            kind: InstructionKind::AddressOf { dest, operand },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new call instruction with multiple return values
    pub fn call(
        dests: Vec<ValueId>,
        callee: crate::FunctionId,
        args: Vec<Value>,
        signature: CalleeSignature,
    ) -> Self {
        debug_assert_eq!(
            dests.len(),
            signature.return_types.len(),
            "Call instruction destination count mismatch with signature return count"
        );

        Self {
            kind: InstructionKind::Call {
                dests,
                callee,
                args,
                signature,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new void call instruction
    pub const fn void_call(
        callee: crate::FunctionId,
        args: Vec<Value>,
        signature: CalleeSignature,
    ) -> Self {
        Self {
            kind: InstructionKind::VoidCall {
                callee,
                args,
                signature,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new get element pointer instruction
    pub const fn get_element_ptr(dest: ValueId, base: Value, offset: Value) -> Self {
        Self {
            kind: InstructionKind::GetElementPtr { dest, base, offset },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new cast instruction
    pub const fn cast(dest: ValueId, source: Value) -> Self {
        Self {
            kind: InstructionKind::Cast { dest, source },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new debug instruction
    pub const fn debug(message: String, values: Vec<Value>) -> Self {
        Self {
            kind: InstructionKind::Debug { message, values },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Sets the source span for this instruction
    pub const fn with_span(mut self, span: SimpleSpan<usize>) -> Self {
        self.source_span = Some(span);
        self
    }

    /// Sets the source expression ID for this instruction
    pub const fn with_expr_id(mut self, expr_id: MirExpressionId) -> Self {
        self.source_expr_id = Some(expr_id);
        self
    }

    /// Sets a comment for this instruction
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Returns the destination values if this instruction defines any
    pub fn destinations(&self) -> Vec<ValueId> {
        match &self.kind {
            InstructionKind::Assign { dest, .. }
            | InstructionKind::AssignU32 { dest, .. }
            | InstructionKind::UnaryOp { dest, .. }
            | InstructionKind::BinaryOp { dest, .. }
            | InstructionKind::Load { dest, .. }
            | InstructionKind::LoadU32 { dest, .. }
            | InstructionKind::StackAlloc { dest, .. }
            | InstructionKind::AddressOf { dest, .. }
            | InstructionKind::GetElementPtr { dest, .. }
            | InstructionKind::Cast { dest, .. } => vec![*dest],

            InstructionKind::Call { dests, .. } => dests.clone(),

            InstructionKind::VoidCall { .. }
            | InstructionKind::Store { .. }
            | InstructionKind::StoreU32 { .. }
            | InstructionKind::Debug { .. } => vec![],
        }
    }

    /// Returns the destination value if this instruction defines exactly one
    pub fn destination(&self) -> Option<ValueId> {
        let dests = self.destinations();
        if dests.len() == 1 {
            Some(dests[0])
        } else {
            None
        }
    }

    /// Returns all values used by this instruction
    pub fn used_values(&self) -> HashSet<ValueId> {
        let mut used = HashSet::new();

        match &self.kind {
            InstructionKind::Assign { source, .. } | InstructionKind::AssignU32 { source, .. } => {
                if let Value::Operand(id) = source {
                    used.insert(*id);
                }
            }

            InstructionKind::UnaryOp { source, .. } => {
                if let Value::Operand(id) = source {
                    used.insert(*id);
                }
            }

            InstructionKind::BinaryOp { left, right, .. } => {
                if let Value::Operand(id) = left {
                    used.insert(*id);
                }
                if let Value::Operand(id) = right {
                    used.insert(*id);
                }
            }

            InstructionKind::Call { args, .. } | InstructionKind::VoidCall { args, .. } => {
                for arg in args {
                    if let Value::Operand(id) = arg {
                        used.insert(*id);
                    }
                }
            }

            InstructionKind::Load { address, .. } => {
                if let Value::Operand(id) = address {
                    used.insert(*id);
                }
            }

            InstructionKind::LoadU32 { address, .. } => {
                if let Value::Operand(id) = address {
                    used.insert(*id);
                }
            }

            InstructionKind::Store { address, value } => {
                if let Value::Operand(id) = address {
                    used.insert(*id);
                }
                if let Value::Operand(id) = value {
                    used.insert(*id);
                }
            }

            InstructionKind::StoreU32 { address, value } => {
                if let Value::Operand(id) = address {
                    used.insert(*id);
                }
                if let Value::Operand(id) = value {
                    used.insert(*id);
                }
            }

            InstructionKind::StackAlloc { .. } => {
                // Stack allocation doesn't use any values as input
            }

            InstructionKind::AddressOf { operand, .. } => {
                used.insert(*operand);
            }

            InstructionKind::GetElementPtr { base, offset, .. } => {
                if let Value::Operand(id) = base {
                    used.insert(*id);
                }
                if let Value::Operand(id) = offset {
                    used.insert(*id);
                }
            }

            InstructionKind::Cast { source, .. } => {
                if let Value::Operand(id) = source {
                    used.insert(*id);
                }
            }

            InstructionKind::Debug { values, .. } => {
                for value in values {
                    if let Value::Operand(id) = value {
                        used.insert(*id);
                    }
                }
            }
        }

        used
    }

    /// Validates this instruction
    pub const fn validate(&self) -> Result<(), String> {
        match &self.kind {
            InstructionKind::Assign { .. } => Ok(()),
            InstructionKind::AssignU32 { .. } => Ok(()),
            InstructionKind::UnaryOp { .. } => Ok(()),
            InstructionKind::BinaryOp { .. } => Ok(()),
            InstructionKind::Call { .. } => Ok(()),
            InstructionKind::VoidCall { .. } => Ok(()),
            InstructionKind::Load { .. } => Ok(()),
            InstructionKind::LoadU32 { .. } => Ok(()),
            InstructionKind::Store { .. } => Ok(()),
            InstructionKind::StoreU32 { .. } => Ok(()),
            InstructionKind::StackAlloc { .. } => Ok(()),
            InstructionKind::AddressOf { .. } => Ok(()),
            InstructionKind::GetElementPtr { .. } => Ok(()),
            InstructionKind::Cast { .. } => Ok(()),
            InstructionKind::Debug { .. } => Ok(()),
        }
    }

    /// Returns true if this instruction has side effects
    pub const fn has_side_effects(&self) -> bool {
        matches!(
            self.kind,
            InstructionKind::VoidCall { .. }
                | InstructionKind::Store { .. }
                | InstructionKind::StackAlloc { .. }
                | InstructionKind::Debug { .. }
        )
    }

    /// Returns true if this instruction is pure (no side effects, result only depends on inputs)
    pub const fn is_pure(&self) -> bool {
        !self.has_side_effects()
    }
}

impl PrettyPrint for Instruction {
    fn pretty_print(&self, _indent: usize) -> String {
        let mut result = String::new();

        // Add comment if present
        if let Some(comment) = &self.comment {
            result.push_str(&format!("// {comment}\n"));
        }

        match &self.kind {
            InstructionKind::Assign { dest, source } => {
                result.push_str(&format!(
                    "{} = {}",
                    dest.pretty_print(0),
                    source.pretty_print(0)
                ));
            }

            InstructionKind::AssignU32 { dest, source } => {
                result.push_str(&format!(
                    "{} = {} (u32)",
                    dest.pretty_print(0),
                    source.pretty_print(0)
                ));
            }

            InstructionKind::UnaryOp {
                op,
                dest,
                source,
                in_place_target,
            } => {
                // If we have an in-place target, that's where the result actually goes
                let dest_str = if let Some(target) = in_place_target {
                    format!("%{}", target.index())
                } else {
                    dest.pretty_print(0)
                };

                result.push_str(&format!(
                    "{} = {:?} {}",
                    dest_str,
                    op,
                    source.pretty_print(0)
                ));
            }

            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
                in_place_target,
            } => {
                // If we have an in-place target, that's where the result actually goes
                let dest_str = if let Some(target) = in_place_target {
                    format!("%{}", target.index())
                } else {
                    dest.pretty_print(0)
                };

                result.push_str(&format!(
                    "{} = {} {:?} {}",
                    dest_str,
                    left.pretty_print(0),
                    op,
                    right.pretty_print(0)
                ));
            }

            InstructionKind::Call {
                dests,
                callee,
                args,
                signature: _,
            } => {
                let args_str = args
                    .iter()
                    .map(|arg| arg.pretty_print(0))
                    .collect::<Vec<_>>()
                    .join(", ");

                if dests.is_empty() {
                    // Should not happen, but handle gracefully
                    result.push_str(&format!("call {:?}({})", callee, args_str));
                } else if dests.len() == 1 {
                    result.push_str(&format!(
                        "{} = call {:?}({})",
                        dests[0].pretty_print(0),
                        callee,
                        args_str
                    ));
                } else {
                    let dests_str = dests
                        .iter()
                        .map(|d| d.pretty_print(0))
                        .collect::<Vec<_>>()
                        .join(", ");
                    result.push_str(&format!("{} = call {:?}({})", dests_str, callee, args_str));
                }
            }

            InstructionKind::VoidCall { callee, args, .. } => {
                let args_str = args
                    .iter()
                    .map(|arg| arg.pretty_print(0))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!("call {callee:?}({args_str})"));
            }

            InstructionKind::Load { dest, address } => {
                result.push_str(&format!(
                    "{} = load {}",
                    dest.pretty_print(0),
                    address.pretty_print(0)
                ));
            }

            InstructionKind::LoadU32 { dest, address } => {
                result.push_str(&format!(
                    "{} = loadU32 {}",
                    dest.pretty_print(0),
                    address.pretty_print(0)
                ));
            }

            InstructionKind::Store { address, value } => {
                result.push_str(&format!(
                    "store {}, {}",
                    address.pretty_print(0),
                    value.pretty_print(0)
                ));
            }

            InstructionKind::StoreU32 { address, value } => {
                result.push_str(&format!(
                    "storeU32 [{}], [{}]",
                    address.pretty_print(0),
                    value.pretty_print(0)
                ));
            }

            InstructionKind::StackAlloc { dest, size } => {
                result.push_str(&format!("{} = stackalloc {}", dest.pretty_print(0), size));
            }

            InstructionKind::AddressOf { dest, operand } => {
                result.push_str(&format!(
                    "{} = &{}",
                    dest.pretty_print(0),
                    operand.pretty_print(0)
                ));
            }

            InstructionKind::GetElementPtr { dest, base, offset } => {
                result.push_str(&format!(
                    "{} = getelementptr {}, {}",
                    dest.pretty_print(0),
                    base.pretty_print(0),
                    offset.pretty_print(0)
                ));
            }

            InstructionKind::Cast { dest, source } => {
                result.push_str(&format!(
                    "{} = cast {}",
                    dest.pretty_print(0),
                    source.pretty_print(0)
                ));
            }

            InstructionKind::Debug { message, values } => {
                let values_str = values
                    .iter()
                    .map(|val| val.pretty_print(0))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!("debug \"{message}\" [{values_str}]"));
            }
        }

        result
    }
}

impl PrettyPrint for ValueId {
    fn pretty_print(&self, _indent: usize) -> String {
        format!("%{}", self.index())
    }
}
