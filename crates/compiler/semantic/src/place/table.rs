//! Efficient place storage and lookup using hash tables
//!
//! This module implements PlaceTable inspired by ruff's architecture,
//! providing O(1) lookups for both simple names and complex place expressions.

use std::hash::{Hash, Hasher};

use hashbrown::HashTable;
use index_vec::IndexVec;
use rustc_hash::FxHasher;

use super::expr::PlaceExpr;
use super::{PlaceFlags, ScopedPlaceId};

/// A place expression with its associated flags
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceExprWithFlags {
    /// The place expression
    pub expr: PlaceExpr,
    /// Flags indicating properties of this place
    pub flags: PlaceFlags,
}

/// Efficient storage and lookup of places within a scope
///
/// This table provides:
/// - O(1) lookup by name for simple identifiers
/// - O(1) lookup by full expression for complex places
/// - Support for variable shadowing
/// - Efficient iteration over all places
#[derive(Debug, Default)]
pub struct PlaceTable {
    /// All places in this scope, indexed by ScopedPlaceId
    places: IndexVec<ScopedPlaceId, PlaceExprWithFlags>,
    /// Hash table for O(1) lookups by name or expression
    place_set: HashTable<ScopedPlaceId>,
}

impl PlaceTable {
    /// Create a new empty place table
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new place and return its ID
    ///
    /// If the place already exists, updates its flags and returns the existing ID.
    pub fn add_place(&mut self, place: PlaceExpr, flags: PlaceFlags) -> ScopedPlaceId {
        let hash = Self::hash_place_expr(&place);

        // Check if place already exists
        if let Some(&existing_id) = self
            .place_set
            .find(hash, |id| self.places[*id].expr == place)
        {
            // Update flags for existing place
            self.places[existing_id].flags |= flags;
            return existing_id;
        }

        // Add new place
        let place_with_flags = PlaceExprWithFlags { expr: place, flags };
        let id = self.places.push(place_with_flags);
        self.place_set
            .insert_unique(hash, id, |id| Self::hash_place_expr(&self.places[*id].expr));
        id
    }

    /// Look up a place by name (for simple identifiers)
    ///
    /// This is optimized for the common case of looking up simple variable names.
    pub fn place_id_by_name(&self, name: &str) -> Option<ScopedPlaceId> {
        self.place_set
            .find(Self::hash_name(name), |id| {
                self.places[*id].expr.as_name() == Some(name)
            })
            .copied()
    }

    /// Look up a place by expression
    ///
    /// This handles both simple names and complex expressions like `obj.field` or `arr[0]`.
    pub fn place_id_by_expr(&self, expr: &PlaceExpr) -> Option<ScopedPlaceId> {
        self.place_set
            .find(Self::hash_place_expr(expr), |id| {
                &self.places[*id].expr == expr
            })
            .copied()
    }

    /// Get place by ID
    pub fn place(&self, id: ScopedPlaceId) -> Option<&PlaceExprWithFlags> {
        self.places.get(id)
    }

    /// Get mutable place by ID
    pub fn place_mut(&mut self, id: ScopedPlaceId) -> Option<&mut PlaceExprWithFlags> {
        self.places.get_mut(id)
    }

    /// Mark a place as used
    pub fn mark_as_used(&mut self, id: ScopedPlaceId) {
        if let Some(place) = self.places.get_mut(id) {
            place.flags.insert(PlaceFlags::USED);
        }
    }

    /// Iterate over all places in this table
    pub fn places(&self) -> impl Iterator<Item = (ScopedPlaceId, &PlaceExprWithFlags)> {
        self.places
            .iter()
            .enumerate()
            .map(|(i, place)| (ScopedPlaceId::new(i), place))
    }

    /// Get the number of places in this table
    pub fn len(&self) -> usize {
        self.places.len()
    }

    /// Check if this table has no places
    pub fn is_empty(&self) -> bool {
        self.places.is_empty()
    }

    /// Hash a simple name
    ///
    /// This is used for fast lookups of simple identifiers.
    fn hash_name(name: &str) -> u64 {
        let mut hasher = FxHasher::default();
        name.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a place expression
    ///
    /// For simple names, this produces the same hash as `hash_name` to enable
    /// lookups by either name or expression.
    fn hash_place_expr(expr: &PlaceExpr) -> u64 {
        let mut hasher = FxHasher::default();
        // For simple names, just hash the name so lookups by name work
        if expr.is_name() {
            expr.root_name().hash(&mut hasher);
        } else {
            // Hash the full expression
            expr.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl Clone for PlaceTable {
    fn clone(&self) -> Self {
        let places = self.places.clone();
        let mut place_set = HashTable::with_capacity(places.len());

        // Rebuild the hash table
        for (i, place) in places.iter().enumerate() {
            let id = ScopedPlaceId::new(i);
            let hash = Self::hash_place_expr(&place.expr);
            place_set.insert_unique(hash, id, |_| hash);
        }

        Self { places, place_set }
    }
}

impl PartialEq for PlaceTable {
    fn eq(&self, other: &Self) -> bool {
        // Only compare the places vector since place_set is derived from it
        self.places == other.places
    }
}

impl Eq for PlaceTable {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_name_lookup() {
        let mut table = PlaceTable::new();

        // Add a simple name
        let x_id = table.add_place(PlaceExpr::name("x".to_string()), PlaceFlags::DEFINED);

        // Look up by name
        assert_eq!(table.place_id_by_name("x"), Some(x_id));
        assert_eq!(table.place_id_by_name("y"), None);

        // Look up by expression
        let x_expr = PlaceExpr::name("x".to_string());
        assert_eq!(table.place_id_by_expr(&x_expr), Some(x_id));
    }

    #[test]
    fn test_complex_expression_lookup() {
        let mut table = PlaceTable::new();

        // Add a complex expression
        let expr = PlaceExpr::name("obj".to_string()).with_member("field".to_string());
        let id = table.add_place(expr.clone(), PlaceFlags::DEFINED);

        // Look up by expression
        assert_eq!(table.place_id_by_expr(&expr), Some(id));

        // Simple name lookup should not find it
        assert_eq!(table.place_id_by_name("obj.field"), None);
    }

    #[test]
    fn test_flag_updates() {
        let mut table = PlaceTable::new();

        // Add a place
        let expr = PlaceExpr::name("x".to_string());
        let id = table.add_place(expr.clone(), PlaceFlags::DEFINED);

        // Add the same place with different flags
        let id2 = table.add_place(expr, PlaceFlags::USED);

        // Should be the same ID
        assert_eq!(id, id2);

        // Flags should be combined
        let place = table.place(id).unwrap();
        assert!(place.flags.contains(PlaceFlags::DEFINED));
        assert!(place.flags.contains(PlaceFlags::USED));
    }

    #[test]
    fn test_mark_as_used() {
        let mut table = PlaceTable::new();

        let id = table.add_place(PlaceExpr::name("x".to_string()), PlaceFlags::DEFINED);

        // Initially not used
        assert!(!table.place(id).unwrap().flags.contains(PlaceFlags::USED));

        // Mark as used
        table.mark_as_used(id);
        assert!(table.place(id).unwrap().flags.contains(PlaceFlags::USED));
    }

    #[test]
    fn test_iteration() {
        let mut table = PlaceTable::new();

        let x_id = table.add_place(PlaceExpr::name("x".to_string()), PlaceFlags::DEFINED);
        let y_id = table.add_place(PlaceExpr::name("y".to_string()), PlaceFlags::PARAMETER);

        let places: Vec<_> = table.places().collect();
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].0, x_id);
        assert_eq!(places[1].0, y_id);
    }
}
