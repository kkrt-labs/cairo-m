//! # MIR Terminators
//!
//! This module defines terminators, which end basic blocks and transfer control flow.
//! Every basic block must end with exactly one terminator.

use std::collections::HashSet;

use crate::{BasicBlockId, BinaryOp, PrettyPrint, Value};

/// A terminator ends a basic block and transfers control
///
/// Every basic block MUST end with exactly one terminator.
/// Terminators are the only instructions that can change control flow.
///
/// # Design Notes
///
/// - Each terminator specifies its target blocks explicitly
/// - Conditional branches specify both targets (taken/not taken)
/// - Return terminators end function execution
/// - Unreachable terminators indicate impossible code paths
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    /// Unconditional jump: `jump target`
    /// Always transfers control to the target block
    Jump { target: BasicBlockId },

    /// Conditional branch: `if condition then jump then_target else jump else_target`
    /// Transfers control based on the condition value
    If {
        condition: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    },

    /// Fused comparison and branch: `if left op right then jump then_target else jump else_target`
    /// This is an optimized form of a `BinaryOp` followed by an `If`.
    BranchCmp {
        op: BinaryOp,
        left: Value,
        right: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    },

    /// Function return: `return values`
    /// Ends function execution and returns zero or more values
    Return { values: Vec<Value> },

    /// Unreachable code: indicates this point should never be reached
    /// Used as a placeholder during construction and for optimization
    /// Also used for functions that never return (infinite loops, panics)
    Unreachable,
}

impl Terminator {
    /// Creates a new jump terminator
    pub const fn jump(target: BasicBlockId) -> Self {
        Self::Jump { target }
    }

    /// Creates a new conditional branch terminator
    pub const fn branch(
        condition: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    ) -> Self {
        Self::If {
            condition,
            then_target,
            else_target,
        }
    }

    /// Creates a new fused comparison and branch terminator
    pub const fn branch_cmp(
        op: BinaryOp,
        left: Value,
        right: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    ) -> Self {
        Self::BranchCmp {
            op,
            left,
            right,
            then_target,
            else_target,
        }
    }

    /// Creates a new return terminator with a single value
    pub fn return_value(value: Value) -> Self {
        Self::Return {
            values: vec![value],
        }
    }

    /// Creates a new return terminator with multiple values
    pub const fn return_values(values: Vec<Value>) -> Self {
        Self::Return { values }
    }

    /// Creates a new void return terminator
    pub const fn return_void() -> Self {
        Self::Return { values: Vec::new() }
    }

    /// Creates an unreachable terminator
    pub const fn unreachable() -> Self {
        Self::Unreachable
    }

    /// Returns all basic block targets of this terminator
    ///
    /// This is used for CFG construction and analysis.
    pub fn target_blocks(&self) -> Vec<BasicBlockId> {
        match self {
            Self::Jump { target } => vec![*target],
            Self::If {
                then_target,
                else_target,
                ..
            } => vec![*then_target, *else_target],
            Self::BranchCmp {
                then_target,
                else_target,
                ..
            } => vec![*then_target, *else_target],
            Self::Return { .. } => vec![], // Returns don't target blocks
            Self::Unreachable => vec![],   // Unreachable code has no targets
        }
    }

    /// Returns all values used by this terminator
    pub fn used_values(&self) -> HashSet<crate::ValueId> {
        let mut used = HashSet::new();

        match self {
            Self::Jump { .. } => {
                // No values used
            }

            Self::If { condition, .. } => {
                if let Value::Operand(id) = condition {
                    used.insert(*id);
                }
            }

            Self::BranchCmp { left, right, .. } => {
                if let Value::Operand(id) = left {
                    used.insert(*id);
                }
                if let Value::Operand(id) = right {
                    used.insert(*id);
                }
            }

            Self::Return { values } => {
                for value in values {
                    if let Value::Operand(id) = value {
                        used.insert(*id);
                    }
                }
            }

            Self::Unreachable => {
                // No values used
            }
        }

        used
    }

    /// Returns true if this terminator actually transfers control
    ///
    /// Unreachable terminators don't transfer control since they're never reached.
    pub const fn transfers_control(&self) -> bool {
        !matches!(self, Self::Unreachable)
    }

    /// Returns true if this terminator ends the function
    pub const fn ends_function(&self) -> bool {
        matches!(self, Self::Return { .. } | Self::Unreachable)
    }

    /// Returns true if this is a conditional branch
    pub const fn is_conditional(&self) -> bool {
        matches!(self, Self::If { .. } | Self::BranchCmp { .. })
    }

    /// Returns true if this is an unconditional branch (not counting returns)
    pub const fn is_unconditional_branch(&self) -> bool {
        matches!(self, Self::Jump { .. })
    }

    /// Validates this terminator
    pub const fn validate(&self) -> Result<(), String> {
        match self {
            Self::Jump { .. } => Ok(()),
            Self::If { .. } => Ok(()),
            Self::BranchCmp { .. } => Ok(()),
            Self::Return { .. } => Ok(()),
            Self::Unreachable => Ok(()),
        }
    }

    /// Returns the number of possible successors
    pub const fn successor_count(&self) -> usize {
        match self {
            Self::Jump { .. } => 1,
            Self::If { .. } | Self::BranchCmp { .. } => 2,
            Self::Return { .. } | Self::Unreachable => 0,
        }
    }

    /// Replaces all occurrences of `old_block` with `new_block` in targets
    ///
    /// This is useful for CFG transformations and optimization passes.
    pub fn replace_target(&mut self, old_block: BasicBlockId, new_block: BasicBlockId) {
        match self {
            Self::Jump { target } => {
                if *target == old_block {
                    *target = new_block;
                }
            }

            Self::If {
                then_target,
                else_target,
                ..
            } => {
                if *then_target == old_block {
                    *then_target = new_block;
                }
                if *else_target == old_block {
                    *else_target = new_block;
                }
            }

            Self::BranchCmp {
                then_target,
                else_target,
                ..
            } => {
                if *then_target == old_block {
                    *then_target = new_block;
                }
                if *else_target == old_block {
                    *else_target = new_block;
                }
            }

            Self::Return { .. } | Self::Unreachable => {
                // No targets to replace
            }
        }
    }
}

impl PrettyPrint for Terminator {
    fn pretty_print(&self, _indent: usize) -> String {
        match self {
            Self::Jump { target } => {
                format!("jump {target:?}")
            }

            Self::If {
                condition,
                then_target,
                else_target,
            } => {
                format!(
                    "if {} then jump {then_target:?} else jump {else_target:?}",
                    condition.pretty_print(0)
                )
            }

            Self::BranchCmp {
                op,
                left,
                right,
                then_target,
                else_target,
            } => {
                format!(
                    "if {} {:?} {} then jump {then_target:?} else jump {else_target:?}",
                    left.pretty_print(0),
                    op,
                    right.pretty_print(0)
                )
            }

            Self::Return { values } => {
                if values.is_empty() {
                    "return".to_string()
                } else if values.len() == 1 {
                    format!("return {}", values[0].pretty_print(0))
                } else {
                    let values_str = values
                        .iter()
                        .map(|v| v.pretty_print(0))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("return ({})", values_str)
                }
            }

            Self::Unreachable => "unreachable".to_string(),
        }
    }
}
