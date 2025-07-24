use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use cairo_m_compiler_parser::parser::{
    Expression, NamedType, Parameter, Pattern, Spanned, Statement, StructDef, TopLevelItem,
    TypeExpr,
};
use rustc_hash::{FxBuildHasher, FxHashSet};

pub trait SemanticSyntaxContext {
    /// Returns the path of the file being analyzed.
    fn path(&self) -> &str;

    /// Report a semantic error.
    fn report_semantic_error(&self, error: Diagnostic);
}

#[derive(Default)]
pub struct SemanticSyntaxChecker {}

impl SemanticSyntaxChecker {
    fn add_error<Ctx: SemanticSyntaxContext>(context: &Ctx, error: Diagnostic) {
        context.report_semantic_error(error);
    }

    pub fn check_top_level_items<Ctx: SemanticSyntaxContext>(
        &self,
        context: &Ctx,
        items: &[TopLevelItem],
    ) {
        Self::duplicate_top_level_items(context, items);
    }

    pub fn check_parameters<Ctx: SemanticSyntaxContext>(
        &self,
        context: &Ctx,
        params: &[Parameter],
    ) {
        Self::duplicate_parameter_name(context, params);
    }

    pub fn check_struct<Ctx: SemanticSyntaxContext>(
        &self,
        context: &Ctx,
        struct_def: &Spanned<StructDef>,
    ) {
        Self::duplicate_struct_fields(context, struct_def);
    }

    /// Verifies coherency between an expression's type and a type annotation.
    pub fn check_expr_type_cohesion<Ctx: SemanticSyntaxContext>(
        &self,
        context: &Ctx,
        expr: &Spanned<Expression>,
        type_ast: &Spanned<TypeExpr>,
    ) {
        match (expr.value(), type_ast.value()) {
            (Expression::Tuple(elements), TypeExpr::Tuple(tuple_types)) => {
                for (element, tuple_type) in elements.iter().zip(tuple_types) {
                    self.check_expr_type_cohesion(context, element, tuple_type);
                }
                return;
            }
            (Expression::Tuple(_), _) => {
                Self::add_error(
                    context,
                    Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        code: DiagnosticCode::TypeMismatch,
                        message: "type mismatch: expected tuple".to_string(),
                        file_path: context.path().to_string(),
                        span: type_ast.span(),
                        related_spans: vec![],
                    },
                );
                return;
            }
            _ => {}
        }
        // TODO: handle pointers once implemented.

        let res: Option<(String, String)> = match (type_ast.value(), expr.value()) {
            (TypeExpr::Named(named), Expression::Literal(_, None)) => match named.value() {
                NamedType::Felt => Some((named.value().to_string(), named.value().to_string())),
                NamedType::Bool => Some((named.value().to_string(), "felt".to_string())),
                NamedType::U32 => Some((named.value().to_string(), named.value().to_string())),
                NamedType::Custom(name) => Some((named.value().to_string(), name.to_string())),
            },
            (TypeExpr::Named(named), Expression::Literal(_, Some(suffix))) => match named.value() {
                NamedType::Felt => Some((named.value().to_string(), suffix.clone())),
                NamedType::Bool => Some((named.value().to_string(), suffix.clone())),
                NamedType::U32 => Some((named.value().to_string(), suffix.clone())),
                NamedType::Custom(name) => Some((name.to_string(), suffix.clone())),
            },
            (TypeExpr::Named(named), Expression::BooleanLiteral(_)) => match named.value() {
                NamedType::Felt => Some((named.value().to_string(), "bool".to_string())),
                NamedType::Bool => Some((named.value().to_string(), named.value().to_string())),
                NamedType::U32 => Some((named.value().to_string(), "bool".to_string())),
                NamedType::Custom(name) => Some((name.to_string(), "bool".to_string())),
            },
            _ => None,
        };

        if let Some((typed_name, actual_type)) = res
            && actual_type != typed_name
        {
            Self::add_error(
                context,
                Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: DiagnosticCode::TypeMismatch,
                    message: format!("expected `{typed_name}`, got `{actual_type}`"),
                    file_path: context.path().to_string(),
                    span: expr.span(),
                    related_spans: vec![(
                        expr.span(),
                        format!(
                            "change the type of the numeric literal from `{}` to `{}`",
                            actual_type, typed_name
                        ),
                    )],
                },
            );
        }
    }

    pub fn check_pattern<Ctx: SemanticSyntaxContext>(&self, context: &Ctx, pattern: &Pattern) {
        Self::duplicate_pattern_identifier(context, pattern);
    }

    pub fn check_loop_control_flow<Ctx: SemanticSyntaxContext>(
        &self,
        context: &Ctx,
        stmt: &Spanned<Statement>,
        loop_depth: usize,
    ) {
        let (statement_name, diag_code) = match stmt.value() {
            Statement::Break => ("break", DiagnosticCode::BreakOutsideLoop),
            Statement::Continue => ("continue", DiagnosticCode::ContinueOutsideLoop),
            _ => return,
        };
        if loop_depth == 0 {
            Self::add_error(
                context,
                Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: diag_code,
                    message: format!("`{}` outside of loop", statement_name),
                    file_path: context.path().to_string(),
                    span: stmt.span(),
                    related_spans: vec![],
                },
            );
        }
    }

    fn duplicate_pattern_identifier<Ctx: SemanticSyntaxContext>(ctx: &Ctx, pattern: &Pattern) {
        match pattern {
            Pattern::Tuple(names) => {
                let mut all_arg_names =
                    FxHashSet::with_capacity_and_hasher(names.len(), FxBuildHasher);
                for name in names {
                    if !all_arg_names.insert(name.value().as_str()) {
                        Self::add_error(
                            ctx,
                            Diagnostic {
                                severity: DiagnosticSeverity::Error,
                                code: DiagnosticCode::DuplicatePatternIdentifier,
                                message: format!(
                                    "identifier `{}` is bound more than once in the same pattern",
                                    name.value()
                                ),
                                file_path: ctx.path().to_string(),
                                span: name.span(),
                                related_spans: vec![],
                            },
                        );
                    }
                }
            }
            Pattern::Identifier(_) => {}
        }
    }

    fn duplicate_struct_fields<Ctx: SemanticSyntaxContext>(
        ctx: &Ctx,
        struct_def: &Spanned<StructDef>,
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
                Self::add_error(
                    ctx,
                    Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        code: DiagnosticCode::DuplicateStructField,
                        message: format!("field `{}` is already declared", field.value()),
                        file_path: ctx.path().to_string(),
                        span: field.span(),
                        related_spans: vec![],
                    },
                );
            }
        }
    }

    fn duplicate_top_level_items<Ctx: SemanticSyntaxContext>(ctx: &Ctx, items: &[TopLevelItem]) {
        let mut all_item_names = FxHashSet::with_capacity_and_hasher(items.len(), FxBuildHasher);
        for item in items {
            let names = match item {
                TopLevelItem::Function(func) => vec![func.value().name.value().as_str()],
                TopLevelItem::Struct(struct_def) => vec![struct_def.value().name.value().as_str()],
                TopLevelItem::Namespace(namespace) => vec![namespace.value().name.value().as_str()],
                TopLevelItem::Const(const_def) => vec![const_def.value().name.value().as_str()],
                TopLevelItem::Use(use_stmt) => use_stmt.value().items.names(),
            };

            let spans = match item {
                TopLevelItem::Function(func) => vec![func.value().name.span()],
                TopLevelItem::Struct(struct_def) => vec![struct_def.value().name.span()],
                TopLevelItem::Namespace(namespace) => vec![namespace.value().name.span()],
                TopLevelItem::Const(const_def) => vec![const_def.value().name.span()],
                TopLevelItem::Use(use_stmt) => use_stmt.value().items.spans(),
            };

            for (item_name, span) in names.iter().zip(spans) {
                if !all_item_names.insert(*item_name) {
                    Self::add_error(
                        ctx,
                        Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            code: DiagnosticCode::DuplicateDefinition,
                            message: format!("'{item_name}' defined more than once"),
                            file_path: ctx.path().to_string(),
                            span,
                            related_spans: vec![],
                        },
                    );
                }
            }
        }
    }

    // Taken from ruff
    fn duplicate_parameter_name<Ctx: SemanticSyntaxContext>(ctx: &Ctx, parameters: &[Parameter]) {
        if parameters.len() < 2 {
            return;
        }

        let mut all_arg_names =
            FxHashSet::with_capacity_and_hasher(parameters.len(), FxBuildHasher);

        for parameter in parameters {
            let range = parameter.name.span();
            let param_name = parameter.name.value();
            if !all_arg_names.insert(param_name) {
                Self::add_error(
                    ctx,
                    Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        code: DiagnosticCode::DuplicateParameter,
                        message: format!("'{param_name}' used as parameter more than once"),
                        file_path: ctx.path().to_string(),
                        span: range,
                        related_spans: vec![],
                    },
                );
            }
        }
    }
}
