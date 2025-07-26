//! # Shared Validation Helpers
//!
//! This module contains helper functions that are used by multiple validators
//! to avoid code duplication. These functions implement common validation
//! patterns that apply across different validation passes.

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use cairo_m_compiler_parser::parser::{Parameter, Pattern, Spanned, StructDef};
use rustc_hash::{FxBuildHasher, FxHashSet};

/// Check for duplicate parameter names in a function or method
pub fn check_duplicate_parameter_names(
    params: &[Parameter],
    file_path: &str,
    sink: &dyn DiagnosticSink,
) {
    if params.len() < 2 {
        return;
    }

    let mut all_arg_names = FxHashSet::with_capacity_and_hasher(params.len(), FxBuildHasher);

    for parameter in params {
        let range = parameter.name.span();
        let param_name = parameter.name.value();
        if !all_arg_names.insert(param_name) {
            sink.push(
                Diagnostic::error(
                    DiagnosticCode::DuplicateParameter,
                    format!("'{param_name}' used as parameter more than once"),
                )
                .with_location(file_path.to_string(), range),
            );
        }
    }
}

/// Check for duplicate identifiers in a pattern (e.g., in tuple destructuring)
pub fn check_duplicate_pattern_identifiers(
    pattern: &Pattern,
    file_path: &str,
    sink: &dyn DiagnosticSink,
) {
    match pattern {
        Pattern::Tuple(names) => {
            let mut all_names = FxHashSet::with_capacity_and_hasher(names.len(), FxBuildHasher);
            for name in names {
                if !all_names.insert(name.value().as_str()) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::DuplicatePatternIdentifier,
                            format!(
                                "identifier `{}` is bound more than once in the same pattern",
                                name.value()
                            ),
                        )
                        .with_location(file_path.to_string(), name.span()),
                    );
                }
            }
        }
        Pattern::Identifier(_) => {}
    }
}

/// Check for duplicate field names in a struct definition
pub fn check_duplicate_struct_fields(
    struct_def: &Spanned<StructDef>,
    file_path: &str,
    sink: &dyn DiagnosticSink,
) {
    let fields = struct_def
        .value()
        .fields
        .iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    let mut all_field_names = FxHashSet::with_capacity_and_hasher(fields.len(), FxBuildHasher);
    for field in fields {
        if !all_field_names.insert(field.value().as_str()) {
            sink.push(
                Diagnostic::error(
                    DiagnosticCode::DuplicateStructField,
                    format!("field `{}` is already declared", field.value()),
                )
                .with_location(file_path.to_string(), field.span()),
            );
        }
    }
}
