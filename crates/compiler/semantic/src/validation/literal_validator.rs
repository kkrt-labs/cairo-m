//! # Literal Validator
//!
//! This validator handles validation of literal values:
//! - u32 literal range checking
//! - Detects negative literals (unary negation on literals)
//! - Future: other bounded integer types

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use cairo_m_compiler_parser::parser::{Expression, NamedType, UnaryOp};

use crate::db::{Crate, SemanticDb};
use crate::type_resolution::definition_semantic_type;
use crate::types::TypeData;
use crate::validation::Validator;
use crate::{DefinitionKind, File, SemanticIndex};

/// Validator for literal values and their range constraints
///
/// This validator ensures that literal values fit within the bounds
/// of their declared types, particularly for bounded integer types like u32 and felt.
#[derive(Debug, Default)]
pub struct LiteralValidator;

/// Maximum value for felt type (2^31 - 1)
const FELT_MAX: u32 = (1u32 << 31) - 1;

impl Validator for LiteralValidator {
    fn validate(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        sink: &dyn DiagnosticSink,
    ) {
        // Check all let/const definitions with explicit u32 or felt type
        for (def_idx, definition) in index.all_definitions() {
            match &definition.kind {
                DefinitionKind::Let(let_ref) => {
                    // Check if this is explicitly typed as u32 or felt
                    if let Some(type_expr) = &let_ref.explicit_type_ast {
                        if let cairo_m_compiler_parser::parser::TypeExpr::Named(type_name) =
                            type_expr.value()
                        {
                            match type_name.value() {
                                NamedType::U32 => {
                                    // Check if there's a value expression to validate
                                    if let Some(value_expr_id) = let_ref.value_expr_id {
                                        if let Some(expr_info) = index.expression(value_expr_id) {
                                            self.check_u32_literal(
                                                db,
                                                &expr_info.ast_node,
                                                expr_info.ast_span,
                                                file,
                                                sink,
                                            );
                                        }
                                    }
                                }
                                NamedType::Felt => {
                                    // Check if there's a value expression to validate
                                    if let Some(value_expr_id) = let_ref.value_expr_id {
                                        if let Some(expr_info) = index.expression(value_expr_id) {
                                            self.check_felt_literal(
                                                db,
                                                &expr_info.ast_node,
                                                expr_info.ast_span,
                                                file,
                                                sink,
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                DefinitionKind::Const(const_ref) => {
                    // For const, we need to infer the type from the definition
                    let def_id = crate::semantic_index::DefinitionId::new(db, file, def_idx);
                    let def_type = definition_semantic_type(db, crate_id, def_id);

                    match def_type.data(db) {
                        TypeData::U32 => {
                            // Check if there's a value expression to validate
                            if let Some(value_expr_id) = const_ref.value_expr_id {
                                if let Some(expr_info) = index.expression(value_expr_id) {
                                    self.check_u32_literal(
                                        db,
                                        &expr_info.ast_node,
                                        expr_info.ast_span,
                                        file,
                                        sink,
                                    );
                                }
                            }
                        }
                        TypeData::Felt => {
                            // Check if there's a value expression to validate
                            if let Some(value_expr_id) = const_ref.value_expr_id {
                                if let Some(expr_info) = index.expression(value_expr_id) {
                                    self.check_felt_literal(
                                        db,
                                        &expr_info.ast_node,
                                        expr_info.ast_span,
                                        file,
                                        sink,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn name(&self) -> &'static str {
        "LiteralValidator"
    }
}

impl LiteralValidator {
    /// Check if a literal value fits within u32 range
    fn check_u32_literal(
        &self,
        db: &dyn SemanticDb,
        expr: &Expression,
        span: chumsky::span::SimpleSpan,
        file: File,
        sink: &dyn DiagnosticSink,
    ) {
        match expr {
            Expression::Literal(value, _) => {
                if *value as u64 > u32::MAX as u64 {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "literal value {} is out of range for type u32 (0-{})",
                                value,
                                u32::MAX
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), span)
                        .with_related_span(
                            file.file_path(db).to_string(),
                            span,
                            format!("u32 can only hold values from 0 to {}", u32::MAX),
                        ),
                    );
                }
            }
            Expression::UnaryOp {
                op: UnaryOp::Neg,
                expr: _,
            } => {
                // Negative values are not allowed for u32
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        "negative literal values are not allowed for type u32".to_string(),
                    )
                    .with_location(file.file_path(db).to_string(), span)
                    .with_related_span(
                        file.file_path(db).to_string(),
                        span,
                        format!("u32 can only hold values from 0 to {}", u32::MAX),
                    ),
                );
            }
            _ => {
                // Other expressions are handled by type checking
            }
        }
    }

    /// Check if a literal value fits within felt range
    fn check_felt_literal(
        &self,
        db: &dyn SemanticDb,
        expr: &Expression,
        span: chumsky::span::SimpleSpan,
        file: File,
        sink: &dyn DiagnosticSink,
    ) {
        match expr {
            Expression::Literal(value, _) => {
                if *value > FELT_MAX {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "literal value {} is out of range for type felt (0-{})",
                                value, FELT_MAX
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), span)
                        .with_related_span(
                            file.file_path(db).to_string(),
                            span,
                            format!(
                                "felt can only hold values from 0 to {} (2^31 - 1)",
                                FELT_MAX
                            ),
                        ),
                    );
                }
            }
            Expression::UnaryOp {
                op: UnaryOp::Neg,
                expr: _,
            } => {
                // Negative values are not allowed for felt in Cairo-M (uses M31 field)
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        "negative literal values are not allowed for type felt".to_string(),
                    )
                    .with_location(file.file_path(db).to_string(), span)
                    .with_related_span(
                        file.file_path(db).to_string(),
                        span,
                        format!(
                            "felt can only hold values from 0 to {} (2^31 - 1)",
                            FELT_MAX
                        ),
                    ),
                );
            }
            _ => {
                // Other expressions are handled by type checking
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{crate_from_program, test_db};
    use crate::module_semantic_index;

    #[test]
    fn test_u32_negative_literal_validation() {
        let db = test_db();

        // Negative literal
        let negative_program = "fn test() { let x: u32 = -42; }";
        let crate_id = crate_from_program(&db, negative_program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        let validator = LiteralValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &index, &sink);
        let diagnostics = sink.into_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Negative u32 literal should produce one diagnostic"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("negative literal values are not allowed for type u32")
        );

        // Edge case: -0 should still be flagged as negative
        let zero_program = "fn test() { let x: u32 = -0; }";
        let crate_id = crate_from_program(&db, zero_program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &index, &sink);
        let diagnostics = sink.into_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Negative zero u32 literal should produce one diagnostic"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("negative literal values are not allowed for type u32")
        );
    }

    #[test]
    fn test_felt_overflow_validation() {
        let db = test_db();
        let validator = LiteralValidator;

        // Test value at max boundary (2^31 - 1)
        let max_program = "fn test() { let x: felt = 2147483647; }"; // 2^31 - 1
        let crate_id = crate_from_program(&db, max_program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &index, &sink);
        let diagnostics = sink.into_diagnostics();
        assert_eq!(
            diagnostics.len(),
            0,
            "Maximum felt value should not produce diagnostics"
        );

        // Test value beyond max (2^31)
        let overflow_program = "fn test() { let x: felt = 2147483648; }"; // 2^31
        let crate_id = crate_from_program(&db, overflow_program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &index, &sink);
        let diagnostics = sink.into_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Overflowing felt literal should produce one diagnostic"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("is out of range for type felt")
        );

        // Test negative felt
        let negative_program = "fn test() { let x: felt = -42; }";
        let crate_id = crate_from_program(&db, negative_program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &index, &sink);
        let diagnostics = sink.into_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Negative felt literal should produce one diagnostic"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("negative literal values are not allowed for type felt")
        );
    }

    #[test]
    fn test_const_felt_overflow_validation() {
        let db = test_db();
        let validator = LiteralValidator;

        // Test const with overflow
        let overflow_program = "const X: felt = 2147483648;"; // 2^31
        let crate_id = crate_from_program(&db, overflow_program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &index, &sink);
        let diagnostics = sink.into_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Overflowing const felt literal should produce one diagnostic"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("is out of range for type felt")
        );
    }
}
