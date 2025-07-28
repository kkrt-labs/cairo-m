//! # Structural Validator
//!
//! This validator handles structural semantic checks that were previously done
//! during index building but don't affect the validity of the index itself:
//! - Duplicate parameter names in functions
//! - Duplicate field names in struct definitions
//! - Duplicate identifiers in pattern destructuring
//! - Type cohesion between expressions and type annotations

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use cairo_m_compiler_parser::parser::{
    Expression, FunctionDef, NamedType, Spanned, Statement, StructDef, TopLevelItem, TypeExpr,
    parse_file,
};

use crate::db::{Crate, SemanticDb};
use crate::definition::DefinitionKind;
use crate::validation::{Validator, shared};
use crate::{File, SemanticIndex};

/// Validator for structural semantic rules
///
/// This validator checks for duplicate names and other structural issues
/// that don't affect the semantic index but are still semantic errors.
pub struct StructuralValidator;

impl Validator for StructuralValidator {
    fn validate(
        &self,
        db: &dyn SemanticDb,
        _crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        sink: &dyn DiagnosticSink,
    ) {
        let file_path = file.file_path(db);

        let parsed_program = parse_file(db, file);
        if !parsed_program.diagnostics.is_empty() {
            panic!("Got unexpected parse errors");
        }
        let parsed_module = parsed_program.module;

        // Check each definition in this module
        for (_def_idx, definition) in index.all_definitions() {
            match &definition.kind {
                DefinitionKind::Function(_) => {
                    // Find the function in the AST and check its parameters
                    if let Some(func_def) =
                        self.find_function_in_module(&parsed_module, &definition.name)
                    {
                        shared::check_duplicate_parameter_names(&func_def.params, file_path, sink);
                    }
                }
                DefinitionKind::Struct(_) => {
                    // Find the struct in the AST and check its fields
                    if let Some(struct_def) =
                        self.find_struct_in_module(&parsed_module, &definition.name)
                    {
                        shared::check_duplicate_struct_fields(&struct_def, file_path, sink);
                    }
                }
                _ => {}
            }
        }

        // Check patterns in let statements and function bodies
        self.check_patterns_in_module(&parsed_module, file_path, sink);

        // Check type cohesion in let statements
        self.check_type_cohesion_in_module(&parsed_module, file_path, sink);
    }

    fn name(&self) -> &'static str {
        "StructuralValidator"
    }
}

impl StructuralValidator {
    fn find_function_in_module<'a>(
        &self,
        module: &'a cairo_m_compiler_parser::parser::ParsedModule,
        name: &str,
    ) -> Option<&'a FunctionDef> {
        module.items().iter().find_map(|item| match item {
            TopLevelItem::Function(func) if func.value().name.value() == name => Some(func.value()),
            _ => None,
        })
    }

    fn find_struct_in_module(
        &self,
        module: &cairo_m_compiler_parser::parser::ParsedModule,
        name: &str,
    ) -> Option<Spanned<StructDef>> {
        module.items().iter().find_map(|item| match item {
            TopLevelItem::Struct(struct_def) if struct_def.value().name.value() == name => {
                Some(struct_def.clone())
            }
            _ => None,
        })
    }

    fn check_patterns_in_module(
        &self,
        module: &cairo_m_compiler_parser::parser::ParsedModule,
        file_path: &str,
        sink: &dyn DiagnosticSink,
    ) {
        for item in module.items() {
            if let TopLevelItem::Function(func) = item {
                self.check_patterns_in_statements(&func.value().body, file_path, sink);
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn check_patterns_in_statements(
        &self,
        statements: &[Spanned<Statement>],
        file_path: &str,
        sink: &dyn DiagnosticSink,
    ) {
        for stmt in statements {
            match stmt.value() {
                Statement::Let { pattern, .. } => {
                    shared::check_duplicate_pattern_identifiers(pattern, file_path, sink);
                }
                Statement::Block(statements) => {
                    self.check_patterns_in_statements(statements, file_path, sink);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.check_patterns_in_statements(&[(**then_block).clone()], file_path, sink);
                    if let Some(else_stmt) = else_block {
                        self.check_patterns_in_statements(
                            &[(**else_stmt).clone()],
                            file_path,
                            sink,
                        );
                    }
                }
                Statement::Loop { body } | Statement::While { body, .. } => {
                    self.check_patterns_in_statements(&[(**body).clone()], file_path, sink);
                }
                _ => {}
            }
        }
    }

    fn check_type_cohesion_in_module(
        &self,
        module: &cairo_m_compiler_parser::parser::ParsedModule,
        file_path: &str,
        sink: &dyn DiagnosticSink,
    ) {
        for item in module.items() {
            if let TopLevelItem::Function(func) = item {
                self.check_type_cohesion_in_statements(&func.value().body, file_path, sink);
            }
        }
    }

    fn check_type_cohesion_in_statements(
        &self,
        statements: &[Spanned<Statement>],
        file_path: &str,
        sink: &dyn DiagnosticSink,
    ) {
        for stmt in statements {
            match stmt.value() {
                Statement::Let {
                    value,
                    statement_type: Some(type_ast),
                    ..
                } => {
                    self.check_expr_type_cohesion(value, type_ast, file_path, sink);
                }
                Statement::Block(statements) => {
                    self.check_type_cohesion_in_statements(statements, file_path, sink);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.check_type_cohesion_in_statements(
                        &[(**then_block).clone()],
                        file_path,
                        sink,
                    );
                    if let Some(else_stmt) = else_block {
                        self.check_type_cohesion_in_statements(
                            &[(**else_stmt).clone()],
                            file_path,
                            sink,
                        );
                    }
                }
                Statement::Loop { body } | Statement::While { body, .. } => {
                    self.check_type_cohesion_in_statements(&[(**body).clone()], file_path, sink);
                }
                _ => {}
            }
        }
    }

    /// Verifies coherency between an expression's type and a type annotation.
    #[allow(clippy::only_used_in_recursion)]
    fn check_expr_type_cohesion(
        &self,
        expr: &Spanned<Expression>,
        type_ast: &Spanned<TypeExpr>,
        file_path: &str,
        sink: &dyn DiagnosticSink,
    ) {
        // Recursively handle tuples.
        match (expr.value(), type_ast.value()) {
            (Expression::Tuple(elements), TypeExpr::Tuple(tuple_types)) => {
                for (element, tuple_type) in elements.iter().zip(tuple_types) {
                    self.check_expr_type_cohesion(element, tuple_type, file_path, sink);
                }
                return;
            }
            (Expression::Tuple(_), _) => {
                sink.push(Diagnostic {
                    severity: cairo_m_compiler_diagnostics::DiagnosticSeverity::Error,
                    code: DiagnosticCode::TypeMismatch,
                    message: "type mismatch: expected tuple".to_string(),
                    file_path: file_path.to_string(),
                    span: type_ast.span(),
                    related_spans: vec![],
                });
                return;
            }
            _ => {}
        }

        // TODO: handle pointers once implemented.
        let res: Option<(String, String)> = match (type_ast.value(), expr.value()) {
            (TypeExpr::Named(named), expr) => {
                let expected_type = format!("{}", named.value());

                let actual_type = match expr {
                    // If no suffix, we use the type from the TypeExpr.
                    Expression::Literal(_, None) => match named.value() {
                        NamedType::Bool => "felt", // Default literal type when no suffix
                        _ => expected_type.as_str(),
                    },
                    Expression::Literal(_, Some(suffix)) => suffix,
                    Expression::BooleanLiteral(_) => "bool",
                    _ => return,
                };

                Some((expected_type.to_string(), actual_type.to_string()))
            }
            _ => None,
        };

        // If both types are different, report an error.
        if let Some((typed_name, actual_type)) = res {
            if actual_type != typed_name {
                sink.push(Diagnostic {
                    severity: cairo_m_compiler_diagnostics::DiagnosticSeverity::Error,
                    code: DiagnosticCode::TypeMismatch,
                    message: format!("expected `{typed_name}`, got `{actual_type}`"),
                    file_path: file_path.to_string(),
                    span: expr.span(),
                    related_spans: vec![(
                        expr.span(),
                        format!(
                            "change the type of the numeric literal from `{}` to `{}`",
                            actual_type, typed_name
                        ),
                    )],
                });
            }
        }
    }
}
