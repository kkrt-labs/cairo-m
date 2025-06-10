//! # Places and Scopes
//!
//! This module defines the core data structures for tracking scopes and places (symbols)
//! in the semantic analysis. It follows patterns from Ruff's semantic analysis.
//!
//! ## Key Concepts
//!
//! - **Scope**: A region of code that contains symbols (like a function or module)
//! - **Place**: A named entity that can hold a value (variables, functions, parameters)
//! - **PlaceTable**: Symbol table for a specific scope, mapping names to places

use bitflags::bitflags;
use index_vec::{self, IndexVec};
use rustc_hash::FxHashMap;
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
}

impl fmt::Display for ScopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => write!(f, "module"),
            Self::Function => write!(f, "function"),
            Self::Namespace => write!(f, "namespace"),
            Self::Block => write!(f, "block"),
        }
    }
}

/// The symbol table for a single scope
///
/// This tracks all the places (symbols) within a scope and provides
/// efficient name-to-place lookup.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PlaceTable {
    /// All places in this scope, indexed by ScopedPlaceId
    places: IndexVec<ScopedPlaceId, Place>,
    /// Mapping from name to place ID for fast lookup
    places_by_name: FxHashMap<String, ScopedPlaceId>,
}

impl PlaceTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new place to this scope
    pub fn add_place(&mut self, name: String, flags: PlaceFlags) -> ScopedPlaceId {
        let place_id = ScopedPlaceId::new(self.places.len());
        let place = Place::new(name.clone(), flags);

        self.places.push(place);
        self.places_by_name.insert(name, place_id);

        place_id
    }

    /// Look up a place by name
    pub fn place_id_by_name(&self, name: &str) -> Option<ScopedPlaceId> {
        self.places_by_name.get(name).copied()
    }

    /// Get a place by its ID
    pub fn place(&self, id: ScopedPlaceId) -> Option<&Place> {
        self.places.get(id)
    }

    /// Get a mutable reference to a place by its ID
    pub fn place_mut(&mut self, id: ScopedPlaceId) -> Option<&mut Place> {
        self.places.get_mut(id)
    }

    /// Mark a place as used
    pub fn mark_as_used(&mut self, id: ScopedPlaceId) {
        if let Some(place) = self.place_mut(id) {
            place.flags.insert(PlaceFlags::USED);
        }
    }

    /// Iterate over all places in this scope
    pub fn places(&self) -> impl Iterator<Item = (ScopedPlaceId, &Place)> {
        self.places
            .iter()
            .enumerate()
            .map(|(i, place)| (ScopedPlaceId::new(i), place))
    }

    /// Get the number of places in this scope
    pub fn len(&self) -> usize {
        self.places.len()
    }

    /// Check if this scope has no places
    pub fn is_empty(&self) -> bool {
        self.places.is_empty()
    }
}

/// Represents a single symbol or place in the program
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The name of this place
    pub name: String,
    /// Flags indicating properties of this place
    pub flags: PlaceFlags,
}

impl Place {
    pub const fn new(name: String, flags: PlaceFlags) -> Self {
        Self { name, flags }
    }

    /// Check if this place is defined in its scope
    pub const fn is_defined(&self) -> bool {
        self.flags.contains(PlaceFlags::DEFINED)
    }

    /// Check if this place is used as a value
    pub const fn is_used(&self) -> bool {
        self.flags.contains(PlaceFlags::USED)
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
        let x_id = table.add_place("x".to_string(), PlaceFlags::DEFINED);
        let y_id = table.add_place("y".to_string(), PlaceFlags::PARAMETER);

        // Test lookup by name
        assert_eq!(table.place_id_by_name("x"), Some(x_id));
        assert_eq!(table.place_id_by_name("y"), Some(y_id));
        assert_eq!(table.place_id_by_name("z"), None);

        // Test place retrieval
        let x_place = table.place(x_id).unwrap();
        assert_eq!(x_place.name, "x");
        assert!(x_place.is_defined());
        assert!(!x_place.is_used());

        // Test marking as used
        table.mark_as_used(x_id);
        let x_place = table.place(x_id).unwrap();
        assert!(x_place.is_used());
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
