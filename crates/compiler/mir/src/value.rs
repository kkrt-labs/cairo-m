//! # MIR Values
//!
//! This module defines values and operands in the MIR system.
//! Values represent data that flows through the program.

use crate::PrettyPrint;

/// Represents any value in the program: literals, variables, temporaries, etc.
///
/// Values in MIR can be either immediate constants or references to computed values.
/// This design supports both efficient constant propagation and general computation.
///
/// # Design Notes
///
/// - Literals are embedded directly for efficiency
/// - Operands reference values computed by instructions
/// - The type is Copy for efficient passing around
/// - Error values support graceful error recovery
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Value {
    /// A constant literal value
    /// These are embedded directly for efficient constant propagation
    Literal(Literal),

    /// An operand that references a computed value (variable, temporary, etc.)
    /// The `ValueId` points to the instruction that produces this value
    Operand(crate::ValueId),

    /// A placeholder for unresolved or error values
    /// Used for error recovery during MIR construction
    Error,
}

/// Literal constant values
///
/// These represent compile-time known constants that can be embedded
/// directly in the MIR without requiring computation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Literal {
    /// Integer literal (felt in Cairo-M)
    Integer(i32),

    /// Boolean literal
    Boolean(bool),

    /// Unit value (void, empty tuple)
    Unit,
}

impl Value {
    /// Creates a new integer literal value
    pub const fn integer(value: i32) -> Self {
        Self::Literal(Literal::Integer(value))
    }

    /// Creates a new boolean literal value
    pub const fn boolean(value: bool) -> Self {
        Self::Literal(Literal::Boolean(value))
    }

    /// Creates the unit value
    pub const fn unit() -> Self {
        Self::Literal(Literal::Unit)
    }

    /// Creates a new operand value
    pub const fn operand(id: crate::ValueId) -> Self {
        Self::Operand(id)
    }

    /// Creates an error value for error recovery
    pub const fn error() -> Self {
        Self::Error
    }

    /// Returns true if this is a literal value
    pub const fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    /// Returns true if this is an operand reference
    pub const fn is_operand(&self) -> bool {
        matches!(self, Self::Operand(_))
    }

    /// Returns true if this is an error value
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Returns the literal value if this is a literal
    pub const fn as_literal(&self) -> Option<Literal> {
        match self {
            Self::Literal(lit) => Some(*lit),
            _ => None,
        }
    }

    /// Returns the operand ID if this is an operand
    pub const fn as_operand(&self) -> Option<crate::ValueId> {
        match self {
            Self::Operand(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns true if this value is known at compile time
    pub const fn is_constant(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    /// Attempts to evaluate this value as a constant integer
    ///
    /// Returns `Some(value)` if this is an integer literal,
    /// `None` otherwise. Useful for constant folding.
    pub const fn as_const_integer(&self) -> Option<i32> {
        match self {
            Self::Literal(Literal::Integer(value)) => Some(*value),
            _ => None,
        }
    }

    /// Attempts to evaluate this value as a constant boolean
    pub const fn as_const_boolean(&self) -> Option<bool> {
        match self {
            Self::Literal(Literal::Boolean(value)) => Some(*value),
            _ => None,
        }
    }
}

impl Literal {
    /// Returns true if this is an integer literal
    pub const fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    /// Returns true if this is a boolean literal
    pub const fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    /// Returns true if this is the unit literal
    pub const fn is_unit(&self) -> bool {
        matches!(self, Self::Unit)
    }

    /// Returns the integer value if this is an integer literal
    pub const fn as_integer(&self) -> Option<i32> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the boolean value if this is a boolean literal
    pub const fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }
}

impl PrettyPrint for Value {
    fn pretty_print(&self, _indent: usize) -> String {
        match self {
            Self::Literal(lit) => lit.pretty_print(0),
            Self::Operand(id) => format!("%{}", id.index()),
            Self::Error => "<error>".to_string(),
        }
    }
}

impl PrettyPrint for Literal {
    fn pretty_print(&self, _indent: usize) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Unit => "()".to_string(),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty_print(0))
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty_print(0))
    }
}

// Convenience conversion methods
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::integer(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::boolean(value)
    }
}

impl From<crate::ValueId> for Value {
    fn from(id: crate::ValueId) -> Self {
        Self::operand(id)
    }
}

impl From<Literal> for Value {
    fn from(lit: Literal) -> Self {
        Self::Literal(lit)
    }
}
