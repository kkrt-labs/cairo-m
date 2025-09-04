//! # Scopes
//!
//! This module defines scope IDs and metadata used by the semantic analysis.

use std::fmt;

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
            Self::Block => write!(f, "block"),
            Self::Loop { depth } => write!(f, "loop (depth: {})", depth),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_hierarchy() {
        let module_scope = Scope::new(None, ScopeKind::Module);
        let _function_scope_id = FileScopeId::new(1);
        let function_scope = Scope::new(Some(FileScopeId::new(0)), ScopeKind::Function);

        assert_eq!(module_scope.parent, None);
        assert_eq!(function_scope.parent, Some(FileScopeId::new(0)));
        assert_eq!(function_scope.kind, ScopeKind::Function);
    }
}
