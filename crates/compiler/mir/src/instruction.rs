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
    pub(crate) fn from_parser(
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

    /// Cast/conversion: `source as dest`
    /// For type conversions between compatible types
    Cast {
        dest: ValueId,
        source: Value,
        source_type: MirType,
        target_type: MirType,
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

    /// Create a fixed-size array from values: `dest = make_fixed_array([v0, v1, ...])`
    /// Arrays are value-based aggregates in MIR but materialize to memory when necessary
    /// `is_const` marks arrays originating from semantic `const` contexts and guarantees read-only
    MakeFixedArray {
        dest: ValueId,
        elements: Vec<Value>,
        element_ty: MirType,
        is_const: bool,
    },

    /// Index into a fixed-size array: `dest = arrayindex array, index`
    /// When `index` is a literal, enables SROA; otherwise materialization paths apply.
    ArrayIndex {
        dest: ValueId,
        array: Value,
        index: Value,
        element_ty: MirType,
    },

    /// Insert/Update array element: `dest = arrayinsert array_val, index, value`
    /// Creates a new array value with element at `index` replaced.
    ArrayInsert {
        dest: ValueId,
        array_val: Value,
        index: Value,
        new_value: Value,
        array_ty: MirType,
    },
    /// Assert equality between two values.
    AssertEq { left: Value, right: Value },

    /// Experimental : only used by WASM crate
    /// Load value from an absolute address in the VM
    Load {
        dest: ValueId,
        base_address: Value,
        offset: Value,
        element_ty: MirType,
    },

    /// Experimental : only used by WASM crate
    /// Store value to an absolute address in the VM
    Store {
        base_address: Value,
        offset: Value,
        source: Value,
        element_ty: MirType,
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

    /// Creates a new cast instruction
    pub const fn cast(
        dest: ValueId,
        source: Value,
        source_type: MirType,
        target_type: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::Cast {
                dest,
                source,
                source_type,
                target_type,
            },
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

    /// Creates a new make fixed array instruction (non-const)
    pub const fn make_fixed_array(
        dest: ValueId,
        elements: Vec<Value>,
        element_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::MakeFixedArray {
                dest,
                elements,
                element_ty,
                is_const: false,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new const make fixed array instruction
    pub const fn make_const_fixed_array(
        dest: ValueId,
        elements: Vec<Value>,
        element_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::MakeFixedArray {
                dest,
                elements,
                element_ty,
                is_const: true,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new array index instruction
    pub const fn array_index(
        dest: ValueId,
        array: Value,
        index: Value,
        element_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::ArrayIndex {
                dest,
                array,
                index,
                element_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    /// Creates a new array insert instruction
    pub const fn array_insert(
        dest: ValueId,
        array_val: Value,
        index: Value,
        new_value: Value,
        array_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::ArrayInsert {
                dest,
                array_val,
                index,
                new_value,
                array_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    // Creates a new load instruction
    pub const fn load(
        dest: ValueId,
        base_address: Value,
        offset: Value,
        element_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::Load {
                dest,
                base_address,
                offset,
                element_ty,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        }
    }

    // Creates a new store instruction
    pub const fn store(
        base_address: Value,
        offset: Value,
        source: Value,
        element_ty: MirType,
    ) -> Self {
        Self {
            kind: InstructionKind::Store {
                base_address,
                offset,
                source,
                element_ty,
            },
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

    /// Returns the destination values if this instruction defines any
    pub fn destinations(&self) -> Vec<ValueId> {
        match &self.kind {
            InstructionKind::Assign { dest, .. }
            | InstructionKind::UnaryOp { dest, .. }
            | InstructionKind::BinaryOp { dest, .. }
            | InstructionKind::Cast { dest, .. }
            | InstructionKind::Phi { dest, .. }
            | InstructionKind::MakeTuple { dest, .. }
            | InstructionKind::ExtractTupleElement { dest, .. }
            | InstructionKind::MakeStruct { dest, .. }
            | InstructionKind::ExtractStructField { dest, .. }
            | InstructionKind::InsertField { dest, .. }
            | InstructionKind::InsertTuple { dest, .. }
            | InstructionKind::MakeFixedArray { dest, .. }
            | InstructionKind::ArrayIndex { dest, .. }
            | InstructionKind::ArrayInsert { dest, .. }
            | InstructionKind::Load { dest, .. } => vec![*dest],

            InstructionKind::Call { dests, .. } => dests.clone(),

            InstructionKind::Debug { .. }
            | InstructionKind::Nop
            | InstructionKind::AssertEq { .. }
            | InstructionKind::Store { .. } => vec![],
        }
    }

    /// Returns the destination value if this instruction defines exactly one
    pub(crate) fn destination(&self) -> Option<ValueId> {
        let dests = self.destinations();
        if dests.len() == 1 {
            Some(dests[0])
        } else {
            None
        }
    }

    /// Returns all values used by this instruction
    pub(crate) fn used_values(&self) -> HashSet<ValueId> {
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

            InstructionKind::MakeFixedArray { elements, .. } => {
                visit_values(elements, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::ArrayIndex { array, index, .. } => {
                visit_value(array, |id| {
                    used.insert(id);
                });
                visit_value(index, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::ArrayInsert {
                array_val,
                index,
                new_value,
                ..
            } => {
                visit_value(array_val, |id| {
                    used.insert(id);
                });
                visit_value(index, |id| {
                    used.insert(id);
                });
                visit_value(new_value, |id| {
                    used.insert(id);
                });
            }
            InstructionKind::AssertEq { left, right } => {
                visit_value(left, |id| {
                    used.insert(id);
                });
                visit_value(right, |id| {
                    used.insert(id);
                });
            }
            InstructionKind::Load {
                base_address,
                offset,
                ..
            } => {
                visit_value(base_address, |id| {
                    used.insert(id);
                });
                visit_value(offset, |id| {
                    used.insert(id);
                });
            }

            InstructionKind::Store {
                base_address,
                offset,
                source,
                ..
            } => {
                visit_value(base_address, |id| {
                    used.insert(id);
                });
                visit_value(offset, |id| {
                    used.insert(id);
                });
                visit_value(source, |id| {
                    used.insert(id);
                });
            }
        }

        used
    }

    /// Replace all occurrences of `from` value with `to` value in this instruction
    pub(crate) fn replace_value_uses(&mut self, from: ValueId, to: ValueId) {
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
            InstructionKind::MakeFixedArray { elements, .. } => {
                replace_value_ids(elements, from, to);
            }
            InstructionKind::ArrayIndex { array, index, .. } => {
                replace_value_id(array, from, to);
                replace_value_id(index, from, to);
            }
            InstructionKind::ArrayInsert {
                array_val,
                index,
                new_value,
                ..
            } => {
                replace_value_id(array_val, from, to);
                replace_value_id(index, from, to);
                replace_value_id(new_value, from, to);
            }
            InstructionKind::AssertEq { left, right } => {
                replace_value_id(left, from, to);
                replace_value_id(right, from, to);
            }
            InstructionKind::Load {
                base_address,
                offset,
                ..
            } => {
                replace_value_id(base_address, from, to);
                replace_value_id(offset, from, to);
            }
            InstructionKind::Store {
                base_address,
                offset,
                source,
                ..
            } => {
                replace_value_id(base_address, from, to);
                replace_value_id(offset, from, to);
                replace_value_id(source, from, to);
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
            InstructionKind::MakeFixedArray { .. } => Ok(()),
            InstructionKind::ArrayIndex { .. } => Ok(()),
            InstructionKind::ArrayInsert { .. } => Ok(()),
            InstructionKind::AssertEq { .. } => Ok(()),
            InstructionKind::Load { .. } => Ok(()),
            InstructionKind::Store { .. } => Ok(()),
        }
    }

    /// Returns true if this instruction has side effects
    ///
    /// Inserting into an array: because arrays are passed by pointer, this has side effects, as it modifies the array in place.
    pub const fn has_side_effects(&self) -> bool {
        matches!(
            self.kind,
            InstructionKind::Call { .. }
                | InstructionKind::Debug { .. }
                | InstructionKind::ArrayInsert { .. }
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

    /// Get phi operands mutably if this is a phi instruction
    pub const fn phi_operands_mut(&mut self) -> Option<&mut Vec<(BasicBlockId, Value)>> {
        if let InstructionKind::Phi { sources, .. } = &mut self.kind {
            Some(sources)
        } else {
            None
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

            InstructionKind::Cast {
                dest,
                source,
                source_type,
                target_type,
            } => {
                result.push_str(&format!(
                    "{} = cast {} from {} to {}",
                    dest.pretty_print(0),
                    source.pretty_print(0),
                    source_type,
                    target_type
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

            InstructionKind::MakeFixedArray { dest, elements, .. } => {
                let elements_str = elements
                    .iter()
                    .map(|elem| elem.pretty_print(0))
                    .collect::<Vec<_>>()
                    .join(", ");
                result.push_str(&format!(
                    "{} = makefixedarray [{}]",
                    dest.pretty_print(0),
                    elements_str
                ));
            }

            InstructionKind::ArrayIndex {
                dest,
                array,
                index,
                element_ty: _, // Type info not shown for cleaner output
            } => {
                result.push_str(&format!(
                    "{} = arrayindex {}, {}",
                    dest.pretty_print(0),
                    array.pretty_print(0),
                    index.pretty_print(0)
                ));
            }

            InstructionKind::ArrayInsert {
                dest,
                array_val,
                index,
                new_value,
                array_ty: _, // Type info not shown for cleaner output
            } => {
                result.push_str(&format!(
                    "{} = arrayinsert {}, {}, {}",
                    dest.pretty_print(0),
                    array_val.pretty_print(0),
                    index.pretty_print(0),
                    new_value.pretty_print(0)
                ));
            }
            InstructionKind::AssertEq { left, right } => {
                result.push_str(&format!(
                    "AssertEq {}, {}",
                    left.pretty_print(0),
                    right.pretty_print(0)
                ));
            }
            InstructionKind::Load {
                dest,
                base_address,
                offset,
                element_ty: _,
            } => {
                result.push_str(&format!(
                    "{} = load {}, {}",
                    dest.pretty_print(0),
                    base_address.pretty_print(0),
                    offset.pretty_print(0)
                ));
            }
            InstructionKind::Store {
                base_address,
                offset,
                source,
                element_ty: _,
            } => {
                result.push_str(&format!(
                    "store {}, {}, {}",
                    base_address.pretty_print(0),
                    offset.pretty_print(0),
                    source.pretty_print(0)
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
