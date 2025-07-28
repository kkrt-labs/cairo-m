use cairo_m_compiler_parser::parser::{
    ConstDef, FunctionDef, Namespace, Parameter, ParsedModule, Spanned, StructDef, TopLevelItem,
    UseItems, UseStmt,
};

use crate::Format;
use crate::comment_attachment::HasSpan;
use crate::context::FormatterCtx;
use crate::doc::Doc;
use crate::utils::*;

impl Format for ParsedModule {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        let mut docs = vec![];

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                // Add blank line between top-level items
                docs.push(Doc::line());
                docs.push(Doc::line());
            }
            docs.push(item.format(ctx));
        }

        // Add final newline
        if !self.items.is_empty() {
            docs.push(Doc::line());
        }

        Doc::concat(docs)
    }
}

impl Format for TopLevelItem {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        let (span, inner_doc) = match self {
            Self::Function(f) => (f.span(), f.value().format(ctx)),
            Self::Struct(s) => (s.span(), s.value().format(ctx)),
            Self::Namespace(n) => (n.span(), n.value().format(ctx)),
            Self::Const(c) => (c.span(), c.value().format(ctx)),
            Self::Use(u) => (u.span(), u.value().format(ctx)),
        };

        // Add leading comments
        let mut doc = inner_doc;
        if let Some(leading) = ctx.get_leading_comments(span) {
            let comments: Vec<String> = leading.iter().map(|c| c.text.clone()).collect();
            doc = Doc::with_leading_comments(comments, doc);
        }

        // Add trailing comments
        if let Some(trailing) = ctx.get_trailing_comments(span) {
            let comments: Vec<String> = trailing.iter().map(|c| c.text.clone()).collect();
            doc = Doc::with_trailing_comments(doc, comments);
        }

        doc
    }
}

impl Format for FunctionDef {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        let mut parts = vec![
            Doc::text("fn"),
            Doc::text(" "),
            Doc::text(self.name.value()),
        ];

        // Parameters
        let params = self
            .params
            .iter()
            .map(|p| p.format(ctx))
            .collect::<Vec<_>>();
        parts.push(parens(comma_separated(params)));

        // Return type
        parts.push(Doc::text(" -> "));
        parts.push(self.return_type.value().format(ctx));

        // Body
        parts.push(Doc::text(" {"));

        if !self.body.is_empty() {
            let stmts = self
                .body
                .iter()
                .map(|s| Doc::concat(vec![Doc::line(), s.format(ctx)]))
                .collect::<Vec<_>>();

            parts.push(Doc::indent(ctx.cfg.indent_width, Doc::concat(stmts)));
            parts.push(Doc::line());
        }

        parts.push(Doc::text("}"));

        Doc::concat(parts)
    }
}

impl Format for Parameter {
    fn format(&self, _ctx: &mut FormatterCtx) -> Doc {
        let mut parts = vec![Doc::text(self.name.value())];
        parts.push(Doc::text(": "));
        parts.push(self.type_expr.value().format(_ctx));
        Doc::concat(parts)
    }
}

impl Format for StructDef {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        let mut parts = vec![
            Doc::text("struct"),
            Doc::text(" "),
            Doc::text(self.name.value()),
        ];

        parts.push(Doc::text(" {"));

        if !self.fields.is_empty() {
            let fields = self
                .fields
                .iter()
                .map(|(name, ty)| {
                    Doc::concat(vec![
                        Doc::line(),
                        Doc::text(name.value()),
                        Doc::text(": "),
                        ty.value().format(ctx),
                        Doc::text(","),
                    ])
                })
                .collect::<Vec<_>>();

            parts.push(Doc::indent(ctx.cfg.indent_width, Doc::concat(fields)));
            parts.push(Doc::line());
        }

        parts.push(Doc::text("}"));

        Doc::concat(parts)
    }
}

impl Format for Namespace {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        let mut parts = vec![
            Doc::text("namespace"),
            Doc::text(" "),
            Doc::text(self.name.value()),
        ];

        parts.push(Doc::text(" {"));

        if !self.body.is_empty() {
            let items = self
                .body
                .iter()
                .enumerate()
                .flat_map(|(i, item)| {
                    let mut item_docs = vec![];
                    if i > 0 {
                        item_docs.push(Doc::line());
                    }
                    item_docs.push(Doc::line());
                    item_docs.push(item.format(ctx));
                    item_docs
                })
                .collect::<Vec<_>>();

            parts.push(Doc::indent(ctx.cfg.indent_width, Doc::concat(items)));
            parts.push(Doc::line());
        }

        parts.push(Doc::text("}"));

        Doc::concat(parts)
    }
}

impl Format for ConstDef {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        Doc::concat(vec![
            Doc::text("const"),
            Doc::text(" "),
            Doc::text(self.name.value()),
            Doc::text(" = "),
            self.value.value().format(ctx),
            Doc::text(";"),
        ])
    }
}

impl Format for UseStmt {
    fn format(&self, _ctx: &mut FormatterCtx) -> Doc {
        let mut parts = vec![Doc::text("use"), Doc::text(" ")];

        // Format the path
        let path_str = self
            .path
            .iter()
            .map(|p| p.value().as_str())
            .collect::<Vec<_>>()
            .join("::");
        parts.push(Doc::text(path_str));
        parts.push(Doc::text("::"));

        // Format the items
        match &self.items {
            UseItems::Single(item) => {
                parts.push(Doc::text(item.value()));
            }
            UseItems::List(items) => {
                let item_docs = items
                    .iter()
                    .map(|i| Doc::text(i.value()))
                    .collect::<Vec<_>>();
                parts.push(braces(comma_separated(item_docs)));
            }
        }

        parts.push(Doc::text(";"));
        Doc::concat(parts)
    }
}
