//! Value visitor helpers for reducing duplication in instruction and terminator handling
//!
//! This module provides macros and utilities to visit and replace Value::Operand references
//! uniformly across the codebase, eliminating repetitive pattern matching.

use crate::{Value, ValueId};

/// Visit a single value and apply a closure if it's an operand
#[inline]
pub fn visit_value<F>(value: &Value, mut visitor: F)
where
    F: FnMut(ValueId),
{
    if let Value::Operand(id) = value {
        visitor(*id);
    }
}

/// Visit multiple values and apply a closure to each operand
#[inline]
pub fn visit_values<'a, I, F>(values: I, mut visitor: F)
where
    I: IntoIterator<Item = &'a Value>,
    F: FnMut(ValueId),
{
    for value in values {
        visit_value(value, &mut visitor);
    }
}

/// Replace a value ID in a mutable value reference
#[inline]
pub fn replace_value_id(value: &mut Value, from: ValueId, to: ValueId) {
    if let Value::Operand(id) = value {
        if *id == from {
            *id = to;
        }
    }
}

/// Replace value IDs in multiple mutable value references
#[inline]
pub fn replace_value_ids<'a, I>(values: I, from: ValueId, to: ValueId)
where
    I: IntoIterator<Item = &'a mut Value>,
{
    for value in values {
        replace_value_id(value, from, to);
    }
}

/// Macro for collecting operand IDs from values
#[macro_export]
macro_rules! collect_operands {
    ($set:expr, $($value:expr),* $(,)?) => {
        {
            use $crate::value_visitor::visit_value;
            $(
                visit_value($value, |id| { $set.insert(id); });
            )*
        }
    };
}

/// Macro for replacing operand IDs in values
#[macro_export]
macro_rules! replace_operands {
    ($from:expr, $to:expr, $($value:expr),* $(,)?) => {
        {
            use $crate::value_visitor::replace_value_id;
            $(
                replace_value_id($value, $from, $to);
            )*
        }
    };
}
