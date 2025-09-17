//! Value visitor helpers for reducing duplication in instruction and terminator handling
//!
//! This module provides macros and utilities to visit and replace Value::Operand references
//! uniformly across the codebase, eliminating repetitive pattern matching.

use crate::{Place, Projection, Value, ValueId};

/// Visit a single value and apply a closure if it's an operand
#[inline]
pub(crate) fn visit_value<F>(value: &Value, mut visitor: F)
where
    F: FnMut(ValueId),
{
    if let Value::Operand(id) = value {
        visitor(*id);
    }
}

/// Visit multiple values and apply a closure to each operand
#[inline]
pub(crate) fn visit_values<'a, I, F>(values: I, mut visitor: F)
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
pub(crate) fn replace_value_id(value: &mut Value, from: ValueId, to: ValueId) {
    if let Value::Operand(id) = value {
        if *id == from {
            *id = to;
        }
    }
}

/// Replace value IDs in multiple mutable value references
#[inline]
pub(crate) fn replace_value_ids<'a, I>(values: I, from: ValueId, to: ValueId)
where
    I: IntoIterator<Item = &'a mut Value>,
{
    for value in values {
        replace_value_id(value, from, to);
    }
}

/// Visit a place and apply a closure to every operand it references (base + projections)
pub(crate) fn visit_place<F>(place: &Place, mut visitor: F)
where
    F: FnMut(ValueId),
{
    visitor(place.base);
    for projection in &place.projections {
        if let Projection::Index(value) = projection {
            visit_value(value, &mut visitor);
        }
    }
}

/// Replace occurrences of a value ID inside a place (base + projections)
pub(crate) fn replace_place_value_ids(place: &mut Place, from: ValueId, to: ValueId) {
    if place.base == from {
        place.base = to;
    }

    for projection in &mut place.projections {
        if let Projection::Index(value) = projection {
            replace_value_id(value, from, to);
        }
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
