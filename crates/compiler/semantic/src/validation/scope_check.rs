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

use std::collections::HashSet;

use cairo_m_compiler_diagnostics::Diagnostic;

use crate::db::{Project, SemanticDb};
use crate::validation::Validator;
use crate::{File, PlaceFlags, SemanticIndex};

/// Validator for scope-related semantic rules
///
/// This validator implements comprehensive scope checking to catch common
/// programming errors related to variable scope and usage.
pub struct ScopeValidator;

impl Validator for ScopeValidator {
    fn validate(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only validate the specific module/file we were asked to validate
        // Check for undeclared variables globally for this module
        diagnostics.extend(self.check_undeclared_variables_global(index, file, db, project));

        // Check import validity - ensure imported items actually exist in target modules
        diagnostics.extend(self.check_import_validity(index, file, db, project));

        // Check each scope in this module
        for (scope_id, scope) in index.scopes() {
            if let Some(place_table) = index.place_table(scope_id) {
                diagnostics.extend(self.check_scope(scope_id, scope, place_table, file, db, index));
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
        file: File,
        db: &dyn SemanticDb,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for duplicate definitions within this scope
        diagnostics.extend(self.check_duplicate_definitions(
            scope_id,
            place_table,
            file,
            db,
            index,
        ));

        // Check for unused variables (but not in the global scope for functions/structs)
        diagnostics.extend(self.check_unused_variables(scope_id, place_table, file, db, index));

        diagnostics
    }

    /// Check for duplicate definitions within a scope
    ///
    /// Variable shadowing is allowed for let/local bindings, but duplicate
    /// function names, duplicate parameter names, and duplicate imports are errors.
    fn check_duplicate_definitions(
        &self,
        scope_id: crate::FileScopeId,
        place_table: &crate::PlaceTable,
        file: File,
        db: &dyn SemanticDb,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen_functions = HashSet::new();
        let mut seen_parameters = HashSet::new();
        let mut seen_imports = HashSet::new();

        for (place_id, place) in place_table.places() {
            if place.flags.contains(PlaceFlags::DEFINED) {
                // Check for duplicate function names
                if place.flags.contains(PlaceFlags::FUNCTION) {
                    if !seen_functions.insert(&place.name) {
                        let span = if let Some((_, definition)) =
                            index.definition_for_place(scope_id, place_id)
                        {
                            definition.name_span
                        } else {
                            panic!("No definition found for function {}", place.name);
                        };
                        diagnostics.push(Diagnostic::duplicate_definition(
                            file.file_path(db).to_string(),
                            &place.name,
                            span,
                        ));
                    }
                }
                // Check for duplicate parameter names
                else if place.flags.contains(PlaceFlags::PARAMETER)
                    && !seen_parameters.insert(&place.name)
                {
                    let span = if let Some((_, definition)) =
                        index.definition_for_place(scope_id, place_id)
                    {
                        definition.name_span
                    } else {
                        panic!("No definition found for parameter {}", place.name);
                    };
                    diagnostics.push(Diagnostic::duplicate_definition(
                        file.file_path(db).to_string(),
                        &place.name,
                        span,
                    ));
                }
                // Check for duplicate import names
                else if let Some((_, definition)) = index.definition_for_place(scope_id, place_id)
                {
                    if matches!(definition.kind, crate::definition::DefinitionKind::Use(_))
                        && !seen_imports.insert(&place.name)
                    {
                        diagnostics.push(Diagnostic::duplicate_definition(
                            file.file_path(db).to_string(),
                            &place.name,
                            definition.name_span,
                        ));
                    }
                }
                // Allow shadowing for regular variables (let/local)
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
        file: File,
        db: &dyn SemanticDb,
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
                diagnostics.push(Diagnostic::unused_variable(
                    file.file_path(db).to_string(),
                    &place.name,
                    span,
                ));
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
    fn check_undeclared_variables_global(
        &self,
        index: &SemanticIndex,
        file: File,
        db: &dyn SemanticDb,
        project: Project,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen_undeclared = HashSet::new();

        // Check each identifier usage to see if it was resolved to a definition
        for (usage_index, usage) in index.identifier_usages().iter().enumerate() {
            if !index.is_usage_resolved(usage_index) {
                // If not resolved locally, try to resolve with imports using the centralized method
                if index
                    .resolve_name_with_imports(db, project, file, &usage.name, usage.scope_id)
                    .is_none()
                {
                    // Only report each undeclared variable once
                    if seen_undeclared.insert(usage.name.clone()) {
                        diagnostics.push(Diagnostic::undeclared_variable(
                            file.file_path(db).to_string(),
                            &usage.name,
                            usage.span,
                        ));
                    }
                }
            }
        }

        diagnostics
    }

    /// Check that all imported items actually exist in their target modules
    ///
    /// This validates that use statements like `use lib::{a, nonexistent_b}`
    /// only reference items that actually exist in the target module.
    fn check_import_validity(
        &self,
        index: &SemanticIndex,
        file: File,
        db: &dyn SemanticDb,
        project: Project,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Get the project's semantic index to access all modules
        let project_index = match crate::db::project_semantic_index(db, project) {
            Ok(project_index) => project_index,
            Err(_) => return diagnostics, // If project index fails, skip validation
        };
        let modules = project_index.modules();

        // Check all imports in this file
        for (_scope_id, use_def_ref) in &index.imports {
            let imported_module_name = &use_def_ref.imported_module;
            let imported_item = &use_def_ref.item;

            // Check if the target module exists
            if let Some(imported_module_index) = modules.get(imported_module_name) {
                // Check if the imported item exists in the target module
                if let Some(imported_root) = imported_module_index.root_scope() {
                    if imported_module_index
                        .resolve_name_to_definition(imported_item, imported_root)
                        .is_none()
                    {
                        // The imported item doesn't exist in the target module
                        // We need to find the span for this specific import item to create a proper diagnostic

                        // Find the definition that corresponds to this import
                        for (_def_idx, def) in index.all_definitions() {
                            if let crate::definition::DefinitionKind::Use(use_def) = &def.kind {
                                if use_def.imported_module == *imported_module_name
                                    && use_def.item == *imported_item
                                {
                                    diagnostics.push(Diagnostic::undeclared_variable(
                                        file.file_path(db).to_string(),
                                        imported_item,
                                        def.name_span,
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            // Note: If the target module doesn't exist, that's handled by other validation
            // (the module import itself will fail during dependency resolution)
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
