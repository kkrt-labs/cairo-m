//! Place management system for semantic analysis
//!
//! This module provides the infrastructure for tracking symbols and their properties
//! within scopes during semantic analysis. It uses an architecture inspired by ruff's
//! semantic analyzer.

pub mod expr;
pub mod table;

use std::fmt;

use bitflags::bitflags;
use index_vec::{self};

// Re-export the new place system
pub use self::expr::{PlaceExpr, PlaceExprSubSegment};
pub use self::table::{PlaceExprWithFlags, PlaceTable};

index_vec::define_index_type! {
    /// A unique ID for a scope within a file
    pub struct FileScopeId = usize;

    MAX_INDEX = usize::MAX;
}

impl FileScopeId {
    pub const fn as_usize(self) -> usize {
        self.raw()
    }
}

index_vec::define_index_type! {
    /// A unique ID for a place within a scope
    pub struct ScopedPlaceId = usize;

    MAX_INDEX = usize::MAX;
}

impl ScopedPlaceId {
    pub const fn as_usize(self) -> usize {
        self.raw()
    }
}

/// Represents a scope in the program
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Parent scope, if any (None for module scope)
    pub parent: Option<FileScopeId>,
    /// The kind of scope this represents
    pub kind: ScopeKind,
}

impl Scope {
    pub const fn new(parent: Option<FileScopeId>, kind: ScopeKind) -> Self {
        Self { parent, kind }
    }
}

/// Different types of scopes in Cairo-M
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeKind {
    /// Module-level scope (top-level)
    Module,
    /// Function body scope
    Function,
    /// Namespace scope
    Namespace,
    /// Block scope (for future block scoping support)
    Block,
    /// Loop scope with nesting depth
    Loop { depth: usize },
}

impl fmt::Display for ScopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => write!(f, "module"),
            Self::Function => write!(f, "function"),
            Self::Namespace => write!(f, "namespace"),
            Self::Block => write!(f, "block"),
            Self::Loop { depth } => write!(f, "loop (depth: {})", depth),
        }
    }
}

bitflags! {
    /// Flags indicating properties of a place
    // TODO: assess whether we need this. This might be thrown away as un-needed.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlaceFlags: u8 {
        /// The place is defined in this scope (e.g., `let x = ...`)
        const DEFINED = 1 << 0;
        /// The place is used as a value in this scope
        const USED = 1 << 1;
        /// The place is a function parameter
        const PARAMETER = 1 << 2;
        /// The place is a function name
        const FUNCTION = 1 << 3;
        /// The place is a struct name
        const STRUCT = 1 << 4;
        /// The place is a constant
        const CONSTANT = 1 << 5;
    }
}

impl fmt::Display for PlaceFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        if self.contains(Self::DEFINED) {
            flags.push("defined");
        }
        if self.contains(Self::USED) {
            flags.push("used");
        }
        if self.contains(Self::PARAMETER) {
            flags.push("parameter");
        }
        if self.contains(Self::FUNCTION) {
            flags.push("function");
        }
        if self.contains(Self::STRUCT) {
            flags.push("struct");
        }
        if self.contains(Self::CONSTANT) {
            flags.push("constant");
        }

        if flags.is_empty() {
            write!(f, "none")
        } else {
            write!(f, "{}", flags.join("|"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_table_basic_operations() {
        let mut table = PlaceTable::new();

        // Add some places
        let x_id = table.add_place(PlaceExpr::name("x".to_string()), PlaceFlags::DEFINED);
        let y_id = table.add_place(PlaceExpr::name("y".to_string()), PlaceFlags::PARAMETER);

        // Test lookup by name
        assert_eq!(table.place_id_by_name("x"), Some(x_id));
        assert_eq!(table.place_id_by_name("y"), Some(y_id));
        assert_eq!(table.place_id_by_name("z"), None);

        // Test place retrieval
        let x_place = table.place(x_id).unwrap();
        assert_eq!(x_place.expr.as_name(), Some("x"));
        assert!(x_place.flags.contains(PlaceFlags::DEFINED));
        assert!(!x_place.flags.contains(PlaceFlags::USED));

        // Test marking as used
        table.mark_as_used(x_id);
        let x_place = table.place(x_id).unwrap();
        assert!(x_place.flags.contains(PlaceFlags::USED));
    }

    #[test]
    fn test_scope_hierarchy() {
        let module_scope = Scope::new(None, ScopeKind::Module);
        let _function_scope_id = FileScopeId::new(1);
        let function_scope = Scope::new(Some(FileScopeId::new(0)), ScopeKind::Function);

        assert_eq!(module_scope.parent, None);
        assert_eq!(function_scope.parent, Some(FileScopeId::new(0)));
        assert_eq!(function_scope.kind, ScopeKind::Function);
    }

    #[test]
    fn test_place_flags() {
        let mut flags = PlaceFlags::DEFINED;
        assert!(flags.contains(PlaceFlags::DEFINED));
        assert!(!flags.contains(PlaceFlags::USED));

        flags.insert(PlaceFlags::USED);
        assert!(flags.contains(PlaceFlags::DEFINED));
        assert!(flags.contains(PlaceFlags::USED));

        flags.remove(PlaceFlags::DEFINED);
        assert!(!flags.contains(PlaceFlags::DEFINED));
        assert!(flags.contains(PlaceFlags::USED));
    }
}
