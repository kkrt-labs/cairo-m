use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode};
use cairo_m_compiler_parser::parser::TopLevelItem;
use rustc_hash::{FxBuildHasher, FxHashSet};

pub trait SemanticSyntaxContext {
    /// Returns the path of the file being analyzed.
    fn path(&self) -> &str;

    /// Report a semantic error.
    fn report_semantic_error(&self, error: Diagnostic);
}

/// Minimal semantic syntax checker for index building
///
/// This checker only performs validations that are absolutely necessary
/// during index building to ensure the index remains valid. All other
/// validations are deferred to the validation passes.
#[derive(Default)]
pub struct SemanticSyntaxChecker {}

impl SemanticSyntaxChecker {
    fn add_error<Ctx: SemanticSyntaxContext>(context: &Ctx, error: Diagnostic) {
        context.report_semantic_error(error);
    }

    /// Check for duplicate top-level items - this is a must-have check
    /// because duplicate definitions in the same scope would corrupt the index
    pub fn check_top_level_items<Ctx: SemanticSyntaxContext>(
        &self,
        context: &Ctx,
        items: &[TopLevelItem],
    ) {
        Self::duplicate_top_level_items(context, items);
    }

    fn duplicate_top_level_items<Ctx: SemanticSyntaxContext>(ctx: &Ctx, items: &[TopLevelItem]) {
        let mut all_item_names = FxHashSet::with_capacity_and_hasher(items.len(), FxBuildHasher);
        for item in items {
            let names = match item {
                TopLevelItem::Function(func) => vec![func.value().name.value().as_str()],
                TopLevelItem::Struct(struct_def) => vec![struct_def.value().name.value().as_str()],
                TopLevelItem::Const(const_def) => vec![const_def.value().name.value().as_str()],
                TopLevelItem::Use(use_stmt) => use_stmt.value().items.names(),
            };

            let spans = match item {
                TopLevelItem::Function(func) => vec![func.value().name.span()],
                TopLevelItem::Struct(struct_def) => vec![struct_def.value().name.span()],
                TopLevelItem::Const(const_def) => vec![const_def.value().name.span()],
                TopLevelItem::Use(use_stmt) => use_stmt.value().items.spans(),
            };

            for (item_name, span) in names.iter().zip(spans) {
                if !all_item_names.insert(*item_name) {
                    Self::add_error(
                        ctx,
                        Diagnostic {
                            severity: cairo_m_compiler_diagnostics::DiagnosticSeverity::Error,
                            code: DiagnosticCode::DuplicateDefinition,
                            message: format!("`{item_name}` defined more than once"),
                            file_path: ctx.path().to_string(),
                            span,
                            related_spans: vec![],
                        },
                    );
                }
            }
        }
    }
}
