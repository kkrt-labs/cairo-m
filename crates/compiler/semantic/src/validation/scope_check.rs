//! # Scope Validation
//!
//! This module implements scope-related validation rules for Cairo-M:
//! - **Undeclared variable detection**: Identifies uses of undefined identifiers
//! - **Unused variable detection**: Warns about defined but unused variables
//! - **Duplicate definition detection**: Catches multiple definitions of the same name
//!
//! # Implementation Notes
//!
//! The validator works by analyzing the complete semantic index and checking
//! for violations across all scopes. It uses the use-def chains built during
//! semantic analysis to efficiently detect scope-related issues.
//!
//! # Future Improvements
//!
//! TODO: Add support for more advanced scope validation:
//! - Variable shadowing analysis
//! - Use-before-definition detection with proper ordering
//! - Cross-module scope validation
//! - Const vs mutable variable validation

use crate::db::SemanticDb;
use crate::validation::{Diagnostic, Validator};
use crate::{File, PlaceFlags, SemanticIndex};
use std::collections::HashSet;

/// Validator for scope-related semantic rules
///
/// This validator implements comprehensive scope checking to catch common
/// programming errors related to variable scope and usage.
pub struct ScopeValidator;

impl Validator for ScopeValidator {
    fn validate(
        &self,
        _db: &dyn SemanticDb,
        _file: File,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for undeclared variables once globally (not per scope)
        diagnostics.extend(self.check_undeclared_variables_global(index));

        // Check each scope for other violations
        for (scope_id, scope) in index.scopes() {
            if let Some(place_table) = index.place_table(scope_id) {
                diagnostics.extend(self.check_scope(scope_id, scope, place_table, index));
            }
        }

        diagnostics
    }

    fn name(&self) -> &'static str {
        "ScopeValidator"
    }
}

impl ScopeValidator {
    /// Check a single scope for violations
    ///
    /// This analyzes a single scope for scope-specific issues like duplicate
    /// definitions and unused variables.
    ///
    /// TODO: Add more sophisticated scope-level validation:
    /// - Check for variable shadowing within the same scope
    /// - Validate const vs mutable usage patterns
    /// - Check initialization before use within the scope
    fn check_scope(
        &self,
        scope_id: crate::FileScopeId,
        _scope: &crate::Scope,
        place_table: &crate::PlaceTable,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for duplicate definitions within this scope
        diagnostics.extend(self.check_duplicate_definitions(scope_id, place_table, index));

        // Check for unused variables (but not in the global scope for functions/structs)
        diagnostics.extend(self.check_unused_variables(scope_id, place_table, index));

        diagnostics
    }

    /// Check for duplicate definitions within a scope
    ///
    /// TODO: Improve duplicate detection to handle:
    /// - Function overloading (if supported by Cairo-M)
    /// - Different kinds of definitions (type vs value)
    /// - Generic specializations
    fn check_duplicate_definitions(
        &self,
        scope_id: crate::FileScopeId,
        place_table: &crate::PlaceTable,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen_names = HashSet::new();

        for (place_id, place) in place_table.places() {
            if place.flags.contains(PlaceFlags::DEFINED) && !seen_names.insert(&place.name) {
                // Get the proper span from the definition
                let span =
                    if let Some((_, definition)) = index.definition_for_place(scope_id, place_id) {
                        definition.name_span
                    } else {
                        chumsky::span::SimpleSpan::from(0..0)
                    };
                diagnostics.push(Diagnostic::duplicate_definition(&place.name, span));
            }
        }

        diagnostics
    }

    /// Check for unused variables (warnings for local variables)
    ///
    /// TODO: Improve unused variable detection:
    /// - Different handling for different variable types (params vs locals)
    /// - Allow-list for common unused patterns (e.g., _unused prefix)
    /// - Consider usage in different contexts (read vs write)
    fn check_unused_variables(
        &self,
        scope_id: crate::FileScopeId,
        place_table: &crate::PlaceTable,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (place_id, place) in place_table.places() {
            // Only check local variables and parameters, not functions or structs
            let is_local_or_param = !place.flags.contains(PlaceFlags::FUNCTION)
                && !place.flags.contains(PlaceFlags::STRUCT);

            if is_local_or_param && place.flags.contains(PlaceFlags::DEFINED) && !place.is_used() {
                // Get the proper span from the definition
                let span =
                    if let Some((_, definition)) = index.definition_for_place(scope_id, place_id) {
                        definition.name_span
                    } else {
                        chumsky::span::SimpleSpan::from(0..0)
                    };
                diagnostics.push(Diagnostic::unused_variable(&place.name, span));
            }
        }

        diagnostics
    }

    /// Check for undeclared variables globally by looking at all use-def chains
    ///
    /// TODO: Improve undeclared variable detection:
    /// - Better error messages with suggestions for similar names
    /// - Handle different identifier contexts (types vs values vs modules)
    /// - Support for qualified names and module resolution
    fn check_undeclared_variables_global(&self, index: &SemanticIndex) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen_undeclared = HashSet::new();

        // Check each identifier usage to see if it was resolved to a definition
        for (usage_index, usage) in index.identifier_usages().iter().enumerate() {
            if !index.is_usage_resolved(usage_index) {
                // Only report each undeclared variable once
                if seen_undeclared.insert(usage.name.clone()) {
                    diagnostics.push(Diagnostic::undeclared_variable(&usage.name, usage.span));
                }
            }
        }

        diagnostics
    }

    /// Helper method to resolve a name in the scope chain
    fn _resolve_name_in_scope_chain(
        &self,
        name: &str,
        start_scope: crate::FileScopeId,
        index: &SemanticIndex,
    ) -> bool {
        let mut current_scope = Some(start_scope);

        while let Some(scope_id) = current_scope {
            if let Some(place_table) = index.place_table(scope_id) {
                // Check if the name exists in this scope
                for (_, place) in place_table.places() {
                    if place.name == name && place.flags.contains(PlaceFlags::DEFINED) {
                        return true; // Found definition
                    }
                }
            }

            // Move to parent scope
            current_scope = index.scope(scope_id).and_then(|scope| scope.parent);
        }

        false // Not found in any scope
    }
}
