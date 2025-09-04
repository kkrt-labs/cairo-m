//! # Scope Validation
//!
//! This module implements scope-related validation rules for Cairo-M:
//! - **Undeclared variable detection**: Identifies uses of undefined identifiers
//! - **Unused variable detection**: Warns about defined but unused variables
//!   (except variables with underscore prefix)
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

use crate::builtins::is_builtin_function_name;
use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};

use crate::db::{Crate, SemanticDb};
use crate::validation::Validator;
use crate::{File, SemanticIndex};

/// Validator for scope-related semantic rules
///
/// This validator implements comprehensive scope checking to catch common
/// programming errors related to variable scope and usage.
pub struct ScopeValidator;

impl Validator for ScopeValidator {
    fn validate(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        sink: &dyn DiagnosticSink,
    ) {
        // Only validate the specific module/file we were asked to validate
        // Check for undeclared variables globally for this module
        self.check_undeclared_variables_global(index, file, db, crate_id, sink);

        // Check import validity - ensure imported items actually exist in target modules
        self.check_import_validity(index, file, db, crate_id, sink);

        // Check each scope in this module
        for (scope_id, scope) in index.scopes() {
            self.check_scope(scope_id, scope, file, db, index, sink);
        }
    }

    fn name(&self) -> &'static str {
        "ScopeValidator"
    }
}

impl ScopeValidator {
    /// Check a single scope for violations
    ///
    /// This analyzes a single scope for scope-specific issues like unused variables.
    ///
    /// Note: most duplicate definitions are detected during AST traversal.
    ///
    /// TODO: Add more sophisticated scope-level validation:
    /// - Check for variable shadowing within the same scope
    /// - Validate const vs mutable usage patterns
    /// - Check initialization before use within the scope
    #[allow(clippy::too_many_arguments)]
    fn check_scope(
        &self,
        scope_id: crate::FileScopeId,
        _scope: &crate::Scope,
        file: File,
        db: &dyn SemanticDb,
        index: &SemanticIndex,
        sink: &dyn DiagnosticSink,
    ) {
        // Check for unused variables (but not in the global scope for functions/structs)
        self.check_unused_variables(scope_id, file, db, index, sink);
    }

    /// Check for unused variables (warnings for local variables)
    ///
    /// Variables with underscore prefix (e.g., _unused) are ignored and won't
    /// trigger unused variable warnings. This is a common convention for
    /// variables that are intentionally unused.
    ///
    /// TODO: Improve unused variable detection:
    /// - Different handling for different variable types (params vs locals)
    /// - Consider usage in different contexts (read vs write)
    fn check_unused_variables(
        &self,
        scope_id: crate::FileScopeId,
        file: File,
        db: &dyn SemanticDb,
        index: &SemanticIndex,
        sink: &dyn DiagnosticSink,
    ) {
        for (def_idx, def) in index.definitions_in_scope(scope_id) {
            // Only warn for non-function/struct items
            let is_func_or_struct = matches!(
                def.kind,
                crate::definition::DefinitionKind::Function(_)
                    | crate::definition::DefinitionKind::Struct(_)
            );
            if is_func_or_struct {
                continue;
            }
            // Ignore underscore-prefixed names
            if def.name.starts_with('_') {
                continue;
            }
            // Skip if used
            if index.is_definition_used(def_idx) {
                continue;
            }
            // Otherwise, report unused variable/const/param
            sink.push(Diagnostic::unused_variable(
                file.file_path(db).to_string(),
                &def.name,
                def.name_span,
            ));
        }
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
        crate_id: Crate,
        sink: &dyn DiagnosticSink,
    ) {
        let mut seen_undeclared = HashSet::new();

        // Check each identifier usage to see if it was resolved to a definition
        for (usage_index, usage) in index.identifier_usages().iter().enumerate() {
            if !index.is_usage_resolved(usage_index) {
                // First, try local resolution but skip let-binding if used within its own initializer.
                let found_local = index
                    .resolve_name_at_position(&usage.name, usage.scope_id, usage.span)
                    .is_some();

                // If not found locally, attempt resolving through imports with the same guard.
                let found_import = if !found_local {
                    index
                        .resolve_name_with_imports_at_position(
                            db,
                            crate_id,
                            file,
                            &usage.name,
                            usage.scope_id,
                            usage.span,
                        )
                        .is_some()
                } else {
                    true
                };

                if !found_import {
                    // Built-in function names like `assert` are allowed without local definition
                    if is_builtin_function_name(&usage.name).is_some() {
                        continue;
                    }
                    // Only report each undeclared variable once
                    if seen_undeclared.insert(usage.name.clone()) {
                        sink.push(Diagnostic::undeclared_variable(
                            file.file_path(db).to_string(),
                            &usage.name,
                            usage.span,
                        ));
                    }
                }
            }
        }

        // Check each type usage to see if it was resolved to a definition
        for (usage_index, usage) in index.type_usages().iter().enumerate() {
            if !index.is_type_usage_resolved(usage_index) {
                // If not resolved, try to resolve with imports using the centralized method
                if index
                    .resolve_name_with_imports_at_position(
                        db,
                        crate_id,
                        file,
                        &usage.name,
                        usage.scope_id,
                        usage.span,
                    )
                    .is_none()
                {
                    sink.push(Diagnostic::undeclared_type(
                        file.file_path(db).to_string(),
                        &usage.name,
                        usage.span,
                    ));
                }
            }
        }
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
        crate_id: Crate,
        sink: &dyn DiagnosticSink,
    ) {
        // Get the project's semantic index to access all modules
        let project_index = match crate::db::project_semantic_index(db, crate_id) {
            Ok(project_index) => project_index,
            Err(_) => return, // If project index fails, skip validation
        };
        let modules = project_index.modules();

        // Check all imports in this file
        for (_scope_id, use_def_ref) in &index.imports {
            let imported_module_name = &use_def_ref.imported_module;
            let imported_item = &use_def_ref.item;

            // Check if the target module exists
            if let Some(imported_module_index) = modules.get(imported_module_name.value()) {
                // Check if the imported item exists in the target module
                if let Some(imported_root) = imported_module_index.root_scope() {
                    if imported_module_index
                        .latest_definition_index_by_name(imported_root, imported_item.value())
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
                                    sink.push(
                                        Diagnostic::error(
                                            DiagnosticCode::UnresolvedImport,
                                            format!(
                                                "unresolved import `{}` from module `{}`",
                                                imported_item.value(),
                                                imported_module_name.value()
                                            ),
                                        )
                                        .with_location(
                                            file.file_path(db).to_string(),
                                            imported_item.span(),
                                        ),
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                // The target module doesn't exist - report error
                // Find the definition that corresponds to this import to get the span
                for (_def_idx, def) in index.all_definitions() {
                    if let crate::definition::DefinitionKind::Use(use_def) = &def.kind {
                        if use_def.imported_module == *imported_module_name {
                            sink.push(
                                Diagnostic::error(
                                    DiagnosticCode::UnresolvedModule,
                                    format!("unresolved module `{}`", imported_module_name.value()),
                                )
                                .with_location(
                                    file.file_path(db).to_string(),
                                    imported_module_name.span(),
                                ),
                            );
                            break;
                        }
                    }
                }
            }
        }
    }

    // Note: legacy helper removed during migration to definition-based APIs.
}
