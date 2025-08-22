//! # MIR Instructions
//!
//! This module defines the instruction types for MIR.
//! Instructions perform computations but do not transfer control flow.

use std::collections::HashSet;

use cairo_m_compiler_parser::parser::UnaryOp;
use chumsky::span::SimpleSpan;

use crate::value_visitor::{visit_value, visit_values};
use crate::{BasicBlockId, MirType, PrettyPrint, Value, ValueId};

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

    // U32 bitwise operators (not supported for felt)
    U32BitwiseAnd,
    U32BitwiseOr,
    U32BitwiseXor,
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
            Self::U32BitwiseAnd => write!(f, "& (u32)"),
            Self::U32BitwiseOr => write!(f, "| (u32)"),
            Self::U32BitwiseXor => write!(f, "^ (u32)"),
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

            // U32 bitwise operations
            (P::BitwiseAnd, T::U32) => Self::U32BitwiseAnd,
            (P::BitwiseOr, T::U32) => Self::U32BitwiseOr,
            (P::BitwiseXor, T::U32) => Self::U32BitwiseXor,

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

            // U32 bitwise ops return u32
            Self::U32BitwiseAnd | Self::U32BitwiseOr | Self::U32BitwiseXor => crate::MirType::u32(),

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
    Assign {
        dest: ValueId,
        source: Value,
        ty: MirType,
    },

    /// Unary operation: `dest = op source`
    /// Used for unary operations like negation and logical not
    UnaryOp {
        op: UnaryOp,
        dest: ValueId,
        source: Value,
    },

    /// Binary operation: `dest = left op right`
    /// Covers arithmetic, comparison, and logical operations
    BinaryOp {
        op: BinaryOp,
        dest: ValueId,
        left: Value,
        right: Value,
    },

    /// Function call: `dests = call callee(args)`
    /// For calling functions that return one or more values
    Call {
        dests: Vec<ValueId>,
        callee: crate::FunctionId,
        args: Vec<Value>,
        signature: CalleeSignature,
    },

    /// Load from memory: `dest = load addr`
    /// For accessing memory locations and dereferencing pointers
    Load {
        dest: ValueId,
        ty: MirType,
        address: Value,
    },

    /// Store to memory: `store addr, value`
    /// For writing to memory locations
    Store {
        address: Value,
        value: Value,
        ty: MirType,
    },

    /// Allocate space in the function frame: `dest = framealloc type`
    /// For allocating local variables and temporary storage
    FrameAlloc { dest: ValueId, ty: MirType },

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

    /// No operation instruction
    /// Used as a placeholder during transformations
    Nop,

    /// Phi node for SSA form: `dest = φ(block1: value1, block2: value2, ...)`
    /// Used at control flow merge points to select values from different paths
    ///
    /// A Phi instruction conceptually executes at the beginning of a basic block
    /// and selects the value corresponding to the predecessor block from which
    /// control flow arrived.
    Phi {
        dest: ValueId,
        ty: MirType,
        sources: Vec<(crate::BasicBlockId, Value)>,
    },

    /// Build a tuple from a list of values: `dest = make_tuple(v0, v1, ...)`
    MakeTuple { dest: ValueId, elements: Vec<Value> },

    /// Extract an element from a tuple value: `dest = extract_tuple_element(tuple_val, index)`
    ExtractTupleElement {
        dest: ValueId,
        tuple: Value,
        index: usize,
        element_ty: MirType,
    },

    /// Build a struct from a list of field values: `dest = make_struct { field1: v1, ... }`
    MakeStruct {
        dest: ValueId,
        fields: Vec<(String, Value)>,
        struct_ty: MirType,
    },

    /// Extract a field from a struct value: `dest = extract_struct_field(struct_val, "field_name")`
    ExtractStructField {
        dest: ValueId,
        struct_val: Value,
        field_name: String,
        field_ty: MirType,
    },

    /// Insert a new field value into a struct: `dest = insert_field(struct_val, "field", value)`
    InsertField {
        dest: ValueId,
        struct_val: Value,
        field_name: String,
        new_value: Value,
        struct_ty: MirType,
    },

    /// Insert a new element value into a tuple: `dest = insert_tuple(tuple_val, index, value)`
    InsertTuple {
        dest: ValueId,
        tuple_val: Value,
        index: usize,
        new_value: Value,
        tuple_ty: MirType,
    },
}

impl Instruction {
    /// Creates a new assignment instruction
    pub const fn assign(dest: ValueId, source: Value, ty: MirType) -> Self {
        Self {
            kind: InstructionKind::Assign { dest, source, ty },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new unary operation instruction
    pub const fn unary_op(op: UnaryOp, dest: ValueId, source: Value) -> Self {
        Self {
            kind: InstructionKind::UnaryOp { op, dest, source },
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
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new load instruction
    pub const fn load(dest: ValueId, ty: MirType, address: Value) -> Self {
        Self {
            kind: InstructionKind::Load { dest, ty, address },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new store instruction
    pub const fn store(address: Value, value: Value, ty: MirType) -> Self {
        Self {
            kind: InstructionKind::Store { address, value, ty },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new frame allocation instruction
    pub const fn frame_alloc(dest: ValueId, ty: MirType) -> Self {
        Self {
            kind: InstructionKind::FrameAlloc { dest, ty },
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
        assert_eq!(
            dests.len(),
            signature.return_types.len(),
            "Call instruction: destination count ({}) must match return types ({})",
            dests.len(),
            signature.return_types.len()
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

    /// Creates a new make tuple instruction
    pub const fn make_tuple(dest: ValueId, elements: Vec<Value>) -> Self {
        Self {
            kind: InstructionKind::MakeTuple { dest, elements },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new extract tuple element instruction
    pub const fn extract_tuple_element(
        dest: ValueId,
        tuple: Value,
        index: usize,
        element_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::ExtractTupleElement {
                dest,
                tuple,
                index,
                element_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new make struct instruction
    pub const fn make_struct(
        dest: ValueId,
        fields: Vec<(String, Value)>,
        struct_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::MakeStruct {
                dest,
                fields,
                struct_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new extract struct field instruction
    pub const fn extract_struct_field(
        dest: ValueId,
        struct_val: Value,
        field_name: String,
        field_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::ExtractStructField {
                dest,
                struct_val,
                field_name,
                field_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new insert field instruction
    pub const fn insert_field(
        dest: ValueId,
        struct_val: Value,
        field_name: String,
        new_value: Value,
        struct_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::InsertField {
                dest,
                struct_val,
                field_name,
                new_value,
                struct_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new insert tuple instruction
    pub const fn insert_tuple(
        dest: ValueId,
        tuple_val: Value,
        index: usize,
        new_value: Value,
        tuple_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::InsertTuple {
                dest,
                tuple_val,
                index,
                new_value,
                tuple_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    pub const fn nop() -> Self {
        Self {
            kind: InstructionKind::Nop,
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
            | InstructionKind::UnaryOp { dest, .. }
            | InstructionKind::BinaryOp { dest, .. }
            | InstructionKind::Load { dest, .. }
            | InstructionKind::FrameAlloc { dest, .. }
            | InstructionKind::AddressOf { dest, .. }
            | InstructionKind::GetElementPtr { dest, .. }
            | InstructionKind::Cast { dest, .. }
            | InstructionKind::Phi { dest, .. }
            | InstructionKind::MakeTuple { dest, .. }
            | InstructionKind::ExtractTupleElement { dest, .. }
            | InstructionKind::MakeStruct { dest, .. }
            | InstructionKind::ExtractStructField { dest, .. }
            | InstructionKind::InsertField { dest, .. }
            | InstructionKind::InsertTuple { dest, .. } => vec![*dest],

            InstructionKind::Call { dests, .. } => dests.clone(),

            InstructionKind::Store { .. }
            | InstructionKind::Debug { .. }
            | InstructionKind::Nop => vec![],
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
            InstructionKind::Assign { source, .. } => {
                visit_value(source, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::UnaryOp { source, .. } => {
                visit_value(source, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::BinaryOp { left, right, .. } => {
                visit_value(left, |id| {
                    used.insert(id);
                });
                visit_value(right, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Call { args, .. } => {
                visit_values(args, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Load { address, .. } => {
                visit_value(address, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Store { address, value, .. } => {
                visit_value(address, |id| {
                    used.insert(id);
                });
                visit_value(value, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::FrameAlloc { .. } => {
                // Frame allocation doesn't use any values as input
            }

            InstructionKind::AddressOf { operand, .. } => {
                used.insert(*operand);
            }

            InstructionKind::GetElementPtr { base, offset, .. } => {
                visit_value(base, |id| {
                    used.insert(id);
                });
                visit_value(offset, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Cast { source, .. } => {
                visit_value(source, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Debug { values, .. } => {
                visit_values(values, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Phi { sources, .. } => {
                for (_, value) in sources {
                    visit_value(value, |id| {
                        used.insert(id);
                    });
                }
            }

            InstructionKind::Nop => {
                // No operation - no values used
            }

            InstructionKind::MakeTuple { elements, .. } => {
                visit_values(elements, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::ExtractTupleElement { tuple, .. } => {
                visit_value(tuple, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::MakeStruct { fields, .. } => {
                for (_, value) in fields {
                    visit_value(value, |id| {
                        used.insert(id);
                    });
                }
            }

            InstructionKind::ExtractStructField { struct_val, .. } => {
                visit_value(struct_val, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::InsertField {
                struct_val,
                new_value,
                ..
            } => {
                if let Value::Operand(id) = struct_val {
                    used.insert(*id);
                }
                if let Value::Operand(id) = new_value {
                    used.insert(*id);
                }
            }

            InstructionKind::InsertTuple {
                tuple_val,
                new_value,
                ..
            } => {
                if let Value::Operand(id) = tuple_val {
                    used.insert(*id);
                }
                if let Value::Operand(id) = new_value {
                    used.insert(*id);
                }
            }
        }

        used
    }

    /// Replace all occurrences of `from` value with `to` value in this instruction
    pub fn replace_value_uses(&mut self, from: ValueId, to: ValueId) {
        if from == to {
            return; // No-op
        }

        use crate::value_visitor::{replace_value_id, replace_value_ids};

        match &mut self.kind {
            InstructionKind::Assign { source, .. } => {
                replace_value_id(source, from, to);
            }
            InstructionKind::UnaryOp { source, .. } => {
                replace_value_id(source, from, to);
            }
            InstructionKind::BinaryOp { left, right, .. } => {
                replace_value_id(left, from, to);
                replace_value_id(right, from, to);
            }
            InstructionKind::Call { args, .. } => {
                replace_value_ids(args, from, to);
            }
            InstructionKind::Load { address, .. } => {
                replace_value_id(address, from, to);
            }
            InstructionKind::Store { address, value, .. } => {
                replace_value_id(address, from, to);
                replace_value_id(value, from, to);
            }
            InstructionKind::FrameAlloc { .. } => {
                // Frame allocation doesn't use any values as input - nothing to replace
            }
            InstructionKind::AddressOf { operand, .. } => {
                if *operand == from {
                    *operand = to;
                }
            }
            InstructionKind::GetElementPtr { base, offset, .. } => {
                replace_value_id(base, from, to);
                replace_value_id(offset, from, to);
            }
            InstructionKind::Cast { source, .. } => {
                replace_value_id(source, from, to);
            }
            InstructionKind::Debug { values, .. } => {
                replace_value_ids(values, from, to);
            }
            InstructionKind::Phi { sources, .. } => {
                for (_, value) in sources {
                    replace_value_id(value, from, to);
                }
            }
            InstructionKind::Nop => {
                // No operation - no values to replace
            }
            InstructionKind::MakeTuple { elements, .. } => {
                replace_value_ids(elements, from, to);
            }
            InstructionKind::ExtractTupleElement { tuple, .. } => {
                replace_value_id(tuple, from, to);
            }
            InstructionKind::MakeStruct { fields, .. } => {
                for (_, value) in fields {
                    replace_value_id(value, from, to);
                }
            }
            InstructionKind::ExtractStructField { struct_val, .. } => {
                replace_value_id(struct_val, from, to);
            }
            InstructionKind::InsertField {
                struct_val,
                new_value,
                ..
            } => {
                replace_value_id(struct_val, from, to);
                replace_value_id(new_value, from, to);
            }
            InstructionKind::InsertTuple {
                tuple_val,
                new_value,
                ..
            } => {
                replace_value_id(tuple_val, from, to);
                replace_value_id(new_value, from, to);
            }
        }
    }

    /// Validates this instruction
    pub const fn validate(&self) -> Result<(), String> {
        match &self.kind {
            InstructionKind::Assign { .. } => Ok(()),
            InstructionKind::UnaryOp { .. } => Ok(()),
            InstructionKind::BinaryOp { .. } => Ok(()),
            InstructionKind::Call { .. } => Ok(()),
            InstructionKind::Load { .. } => Ok(()),
            InstructionKind::Store { .. } => Ok(()),
            InstructionKind::FrameAlloc { .. } => Ok(()),
            InstructionKind::AddressOf { .. } => Ok(()),
            InstructionKind::GetElementPtr { .. } => Ok(()),
            InstructionKind::Cast { .. } => Ok(()),
            InstructionKind::Debug { .. } => Ok(()),
            InstructionKind::Phi { .. } => Ok(()),
            InstructionKind::Nop => Ok(()),
            InstructionKind::MakeTuple { .. } => Ok(()),
            InstructionKind::ExtractTupleElement { .. } => Ok(()),
            InstructionKind::MakeStruct { .. } => Ok(()),
            InstructionKind::ExtractStructField { .. } => Ok(()),
            InstructionKind::InsertField { .. } => Ok(()),
            InstructionKind::InsertTuple { .. } => Ok(()),
        }
    }

    /// Returns true if this instruction has side effects
    pub const fn has_side_effects(&self) -> bool {
        matches!(
            self.kind,
            InstructionKind::Call { .. }
                | InstructionKind::Store { .. }
                | InstructionKind::FrameAlloc { .. }
                | InstructionKind::Debug { .. }
        )
    }

    /// Returns true if this instruction is pure (no side effects, result only depends on inputs)
    pub const fn is_pure(&self) -> bool {
        !self.has_side_effects()
    }

    /// Create a new phi instruction
    pub const fn phi(dest: ValueId, ty: MirType, sources: Vec<(BasicBlockId, Value)>) -> Self {
        Self {
            kind: InstructionKind::Phi { dest, ty, sources },
            comment: None,
            source_span: None,
            source_expr_id: None,
        }
    }

    /// Create an empty phi instruction (operands to be filled later)
    pub const fn empty_phi(dest: ValueId, ty: MirType) -> Self {
        Self::phi(dest, ty, Vec::new())
    }

    /// Check if this instruction is a phi
    pub const fn is_phi(&self) -> bool {
        matches!(self.kind, InstructionKind::Phi { .. })
    }

    /// Get phi operands if this is a phi instruction
    pub fn phi_operands(&self) -> Option<&[(BasicBlockId, Value)]> {
        if let InstructionKind::Phi { sources, .. } = &self.kind {
            Some(sources)
        } else {
            None
        }
    }

    /// Get phi operands mutably if this is a phi instruction
    pub const fn phi_operands_mut(&mut self) -> Option<&mut Vec<(BasicBlockId, Value)>> {
        if let InstructionKind::Phi { sources, .. } = &mut self.kind {
            Some(sources)
        } else {
            None
        }
    }

    /// Add an operand to a phi instruction
    /// Returns true if operand was added, false if not a phi
    pub fn add_phi_operand(&mut self, block: BasicBlockId, value: Value) -> bool {
        if let Some(sources) = self.phi_operands_mut() {
            sources.push((block, value));
            true
        } else {
            false
        }
    }

    /// Set all phi operands at once
    /// Returns true if successful, false if not a phi
    pub fn set_phi_operands(&mut self, operands: Vec<(BasicBlockId, Value)>) -> bool {
        if let Some(sources) = self.phi_operands_mut() {
            *sources = operands;
            true
        } else {
            false
        }
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
            InstructionKind::Assign { dest, source, ty } => {
                if matches!(ty, MirType::Felt) {
                    result.push_str(&format!(
                        "{} = {}",
                        dest.pretty_print(0),
                        source.pretty_print(0),
                    ));
                } else {
                    result.push_str(&format!(
                        "{} = {} ({})",
                        dest.pretty_print(0),
                        source.pretty_print(0),
                        ty
                    ));
                }
            }

            InstructionKind::UnaryOp { op, dest, source } => {
                result.push_str(&format!(
                    "{} = {:?} {}",
                    dest.pretty_print(0),
                    op,
                    source.pretty_print(0)
                ));
            }

            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
            } => {
                result.push_str(&format!(
                    "{} = {} {} {}",
                    dest.pretty_print(0),
                    left.pretty_print(0),
                    op, // Use Display trait instead of Debug
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

            InstructionKind::Load { dest, ty, address } => {
                result.push_str(&format!(
                    "{} = load {} {}",
                    dest.pretty_print(0),
                    ty,
                    address.pretty_print(0)
                ));
            }

            InstructionKind::Store { address, value, ty } => {
                if matches!(ty, MirType::Felt) {
                    result.push_str(&format!(
                        "store {}, {}",
                        address.pretty_print(0),
                        value.pretty_print(0),
                    ));
                } else {
                    result.push_str(&format!(
                        "store {}, {} ({})",
                        address.pretty_print(0),
                        value.pretty_print(0),
                        ty
                    ));
                }
            }

            InstructionKind::FrameAlloc { dest, ty } => {
                result.push_str(&format!("{} = framealloc {}", dest.pretty_print(0), ty));
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

            InstructionKind::Phi { dest, ty, sources } => {
                let sources_str = sources
                    .iter()
                    .map(|(block, val)| format!("[%{}]: {}", block.index(), val.pretty_print(0)))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!(
                    "{} = φ {} {{ {} }}",
                    dest.pretty_print(0),
                    ty,
                    sources_str
                ));
            }

            InstructionKind::Nop => {
                result.push_str("nop");
            }

            InstructionKind::MakeTuple { dest, elements } => {
                let elements_str = elements
                    .iter()
                    .map(|elem| elem.pretty_print(0))
                    .collect::<Vec<_>>()
                    .join(", ");
                if elements.is_empty() {
                    result.push_str(&format!("{} = maketuple", dest.pretty_print(0)));
                } else {
                    result.push_str(&format!(
                        "{} = maketuple {}",
                        dest.pretty_print(0),
                        elements_str
                    ));
                }
            }

            InstructionKind::ExtractTupleElement {
                dest,
                tuple,
                index,
                element_ty: _, // Type info not shown for cleaner output
            } => {
                result.push_str(&format!(
                    "{} = extracttuple {}, {}",
                    dest.pretty_print(0),
                    tuple.pretty_print(0),
                    index
                ));
            }

            InstructionKind::MakeStruct {
                dest,
                fields,
                struct_ty: _, // Type info not shown for cleaner output
            } => {
                let fields_str = fields
                    .iter()
                    .map(|(name, value)| format!("{}: {}", name, value.pretty_print(0)))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!(
                    "{} = makestruct {{ {} }}",
                    dest.pretty_print(0),
                    fields_str
                ));
            }

            InstructionKind::ExtractStructField {
                dest,
                struct_val,
                field_name,
                field_ty: _, // Type info not shown for cleaner output
            } => {
                result.push_str(&format!(
                    "{} = extractfield {}, \"{}\"",
                    dest.pretty_print(0),
                    struct_val.pretty_print(0),
                    field_name
                ));
            }

            InstructionKind::InsertField {
                dest,
                struct_val,
                field_name,
                new_value,
                struct_ty: _, // Type info not shown for cleaner output
            } => {
                result.push_str(&format!(
                    "{} = insertfield {}, \"{}\", {}",
                    dest.pretty_print(0),
                    struct_val.pretty_print(0),
                    field_name,
                    new_value.pretty_print(0)
                ));
            }

            InstructionKind::InsertTuple {
                dest,
                tuple_val,
                index,
                new_value,
                tuple_ty: _, // Type info not shown for cleaner output
            } => {
                result.push_str(&format!(
                    "{} = inserttuple {}, {}, {}",
                    dest.pretty_print(0),
                    tuple_val.pretty_print(0),
                    index,
                    new_value.pretty_print(0)
                ));
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
