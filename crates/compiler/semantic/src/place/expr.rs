//! Place expressions that can be assigned to or referenced
//!
//! This module implements a flexible representation for assignable expressions
//! inspired by ruff's PlaceExpr architecture. It supports:
//! - Simple names (e.g., `x`)
//! - Member access (e.g., `point.x`)
//! - Array subscripts (e.g., `arr[0]`)

use std::hash::Hash;

use cairo_m_compiler_parser::parser::Expression;
use smallvec::SmallVec;

/// Sub-segments that can be appended to a place expression
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceExprSubSegment {
    /// Struct field access, e.g. `.field` in `point.field`
    Member(String),
    /// Array index access, e.g. `[0]` in `arr[0]`
    IntSubscript(i64),
}

/// A place expression that can be assigned to
///
/// This represents any expression that can appear on the left-hand side
/// of an assignment. It consists of a root name followed by zero or more
/// sub-segments (member accesses or subscripts).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaceExpr {
    /// The root identifier
    root_name: String,
    /// Sub-segments like member accesses or subscripts
    /// SmallVec optimizes for the common case of 0-1 segments
    sub_segments: SmallVec<[PlaceExprSubSegment; 1]>,
}

impl PlaceExpr {
    /// Create a simple name place
    pub fn name(name: String) -> Self {
        Self {
            root_name: name,
            sub_segments: SmallVec::new(),
        }
    }

    /// Check if this is just a simple name (no sub-segments)
    pub fn is_name(&self) -> bool {
        self.sub_segments.is_empty()
    }

    /// Get the name if this is a simple name place
    pub fn as_name(&self) -> Option<&str> {
        if self.is_name() {
            Some(&self.root_name)
        } else {
            None
        }
    }

    /// Get the root name of this place expression
    pub fn root_name(&self) -> &str {
        &self.root_name
    }

    /// Get all sub-segments
    pub fn sub_segments(&self) -> &[PlaceExprSubSegment] {
        &self.sub_segments
    }

    /// Add a member access to this place expression
    pub fn with_member(mut self, member: String) -> Self {
        self.sub_segments.push(PlaceExprSubSegment::Member(member));
        self
    }

    /// Add an array subscript to this place expression
    pub fn with_subscript(mut self, index: i64) -> Self {
        self.sub_segments
            .push(PlaceExprSubSegment::IntSubscript(index));
        self
    }

    /// Get all root expressions that need to be bound for this place to be valid
    ///
    /// For example, for `x.y.z`, this returns `[x, x.y]`
    pub fn root_exprs(&self) -> Vec<Self> {
        let mut roots = Vec::new();
        let mut current = Self::name(self.root_name.clone());

        for segment in &self.sub_segments {
            roots.push(current.clone());
            match segment {
                PlaceExprSubSegment::Member(member) => {
                    current = current.with_member(member.clone());
                }
                PlaceExprSubSegment::IntSubscript(idx) => {
                    current = current.with_subscript(*idx);
                }
            }
        }

        roots
    }
}

/// Convert Cairo-M AST expressions to PlaceExpr
impl TryFrom<&Expression> for PlaceExpr {
    type Error = ();

    fn try_from(expr: &Expression) -> Result<Self, ()> {
        match expr {
            Expression::Identifier(name) => Ok(Self::name(name.value().clone())),
            Expression::MemberAccess { object, field } => {
                let mut place = Self::try_from(object.value())?;
                place
                    .sub_segments
                    .push(PlaceExprSubSegment::Member(field.value().clone()));
                Ok(place)
            }
            Expression::IndexAccess { array, index } => {
                // Only support literal integer indices for now
                match index.value() {
                    Expression::Literal(idx) => {
                        let mut place = Self::try_from(array.value())?;
                        place
                            .sub_segments
                            .push(PlaceExprSubSegment::IntSubscript(*idx as i64));
                        Ok(place)
                    }
                    _ => Err(()),
                }
            }
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for PlaceExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.root_name)?;
        for segment in &self.sub_segments {
            match segment {
                PlaceExprSubSegment::Member(member) => write!(f, ".{}", member)?,
                PlaceExprSubSegment::IntSubscript(idx) => write!(f, "[{}]", idx)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_name() {
        let place = PlaceExpr::name("x".to_string());
        assert!(place.is_name());
        assert_eq!(place.as_name(), Some("x"));
        assert_eq!(place.root_name(), "x");
        assert!(place.sub_segments().is_empty());
    }

    #[test]
    fn test_member_access() {
        let place = PlaceExpr::name("point".to_string()).with_member("x".to_string());
        assert!(!place.is_name());
        assert_eq!(place.as_name(), None);
        assert_eq!(place.root_name(), "point");
        assert_eq!(place.sub_segments().len(), 1);
        assert_eq!(place.to_string(), "point.x");
    }

    #[test]
    fn test_nested_member_access() {
        let place = PlaceExpr::name("obj".to_string())
            .with_member("inner".to_string())
            .with_member("field".to_string());
        assert_eq!(place.to_string(), "obj.inner.field");
        assert_eq!(place.sub_segments().len(), 2);
    }

    #[test]
    fn test_array_subscript() {
        let place = PlaceExpr::name("arr".to_string()).with_subscript(0);
        assert_eq!(place.to_string(), "arr[0]");
        assert_eq!(place.sub_segments().len(), 1);
    }

    #[test]
    fn test_complex_expression() {
        let place = PlaceExpr::name("data".to_string())
            .with_member("items".to_string())
            .with_subscript(5)
            .with_member("value".to_string());
        assert_eq!(place.to_string(), "data.items[5].value");
    }

    #[test]
    fn test_root_exprs() {
        let place = PlaceExpr::name("x".to_string())
            .with_member("y".to_string())
            .with_member("z".to_string());

        let roots = place.root_exprs();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].to_string(), "x");
        assert_eq!(roots[1].to_string(), "x.y");
    }
}
