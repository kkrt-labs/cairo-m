use cairo_m_compiler_parser::parser::{Pattern, Spanned, Statement};

use crate::context::FormatterCtx;
use crate::doc::Doc;
use crate::Format;

// Implement Format for Spanned<Statement> to handle comments
impl Format for Spanned<Statement> {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        let span = self.span();
        let mut doc = self.value().format(ctx);

        // Add leading comments
        if let Some(leading) = ctx.get_leading_comments(span) {
            let comments: Vec<String> = leading.iter().map(|c| c.text.clone()).collect();
            doc = Doc::with_leading_comments(comments, doc);
        }

        // Add trailing comments (end-of-line)
        if let Some(trailing) = ctx.get_trailing_comments(span) {
            if let Some(first_trailing) = trailing.first() {
                doc = doc.with_eol_comment(Some(&first_trailing.text));
            }
        }

        doc
    }
}

impl Format for Statement {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        match self {
            Self::Let {
                pattern,
                statement_type,
                value,
            } => {
                let mut parts = vec![Doc::text("let"), Doc::text(" ")];
                parts.push(pattern.format(ctx));

                if let Some(ty) = statement_type {
                    parts.push(Doc::text(": "));
                    parts.push(ty.value().format(ctx));
                }

                parts.push(Doc::text(" = "));
                parts.push(value.value().format(ctx));
                parts.push(Doc::text(";"));

                Doc::concat(parts)
            }
            Self::Const(const_def) => const_def.format(ctx),
            Self::Assignment { lhs, rhs } => Doc::concat(vec![
                lhs.value().format(ctx),
                Doc::text(" = "),
                rhs.value().format(ctx),
                Doc::text(";"),
            ]),
            Self::Return { value } => {
                let mut parts = vec![Doc::text("return")];
                if let Some(expr) = value {
                    parts.push(Doc::text(" "));
                    parts.push(expr.value().format(ctx));
                }
                parts.push(Doc::text(";"));
                Doc::concat(parts)
            }
            Self::Expression(expr) => Doc::concat(vec![expr.value().format(ctx), Doc::text(";")]),
            Self::If {
                condition,
                then_block,
                else_block,
            } => {
                let mut parts = vec![
                    Doc::text("if"),
                    Doc::text(" "),
                    condition.value().format(ctx),
                    Doc::text(" "),
                    then_block.value().format(ctx),
                ];

                if let Some(else_stmt) = else_block {
                    parts.push(Doc::text(" "));
                    parts.push(Doc::text("else"));
                    parts.push(Doc::text(" "));
                    parts.push(else_stmt.value().format(ctx));
                }

                Doc::concat(parts)
            }
            Self::Block(statements) => {
                let mut parts = vec![Doc::text("{")];

                if !statements.is_empty() {
                    let stmts = statements
                        .iter()
                        .map(|s| Doc::concat(vec![Doc::line(), s.value().format(ctx)]))
                        .collect::<Vec<_>>();

                    parts.push(Doc::indent(ctx.cfg.indent_width, Doc::concat(stmts)));
                    parts.push(Doc::line());
                }

                parts.push(Doc::text("}"));
                Doc::concat(parts)
            }
            Self::Loop { body } => Doc::concat(vec![
                Doc::text("loop"),
                Doc::text(" "),
                body.value().format(ctx),
            ]),
            Self::While { condition, body } => Doc::concat(vec![
                Doc::text("while"),
                Doc::text(" "),
                condition.value().format(ctx),
                Doc::text(" "),
                body.value().format(ctx),
            ]),
            Self::For {
                init,
                condition,
                step,
                body,
            } => {
                let mut parts = vec![Doc::text("for"), Doc::text(" "), Doc::text("(")];

                // Format init without semicolon (we'll add it manually)
                let init_formatted = init.value().format(ctx);
                parts.push(init_formatted);
                parts.push(Doc::text(" "));

                parts.push(condition.value().format(ctx));
                parts.push(Doc::text(";"));
                parts.push(Doc::text(" "));

                // Format step - if it's an expression statement, don't include the semicolon
                match step.value() {
                    Self::Expression(expr) => parts.push(expr.value().format(ctx)),
                    _ => parts.push(step.value().format(ctx)),
                }

                parts.push(Doc::text(")"));
                parts.push(Doc::text(" "));
                parts.push(body.value().format(ctx));

                Doc::concat(parts)
            }
            Self::ForIn {
                variable,
                iterable,
                body,
            } => {
                let mut parts = vec![Doc::text("for"), Doc::text(" "), Doc::text("(")];
                
                parts.push(Doc::text(variable.value()));
                parts.push(Doc::text(" in "));
                parts.push(iterable.value().format(ctx));
                
                parts.push(Doc::text(")"));
                parts.push(Doc::text(" "));
                parts.push(body.value().format(ctx));

                Doc::concat(parts)
            }
            Self::Break => Doc::text("break;"),
            Self::Continue => Doc::text("continue;"),
        }
    }
}

impl Format for Pattern {
    fn format(&self, _ctx: &mut FormatterCtx) -> Doc {
        match self {
            Self::Identifier(name) => Doc::text(name.value()),
            Self::Tuple(patterns) => {
                let pattern_docs = patterns.iter().map(|p| p.format(_ctx)).collect::<Vec<_>>();
                Doc::concat(vec![
                    Doc::text("("),
                    Doc::join(Doc::text(", "), pattern_docs),
                    Doc::text(")"),
                ])
            }
        }
    }
}
