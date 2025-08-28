use cairo_m_compiler_parser::parser::{BinaryOp, Expression, UnaryOp};

use crate::context::FormatterCtx;
use crate::doc::Doc;
use crate::utils::*;
use crate::Format;

impl Format for Expression {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        match self {
            Self::Literal(n, suffix) => {
                let mut text = n.to_string();
                if let Some(s) = suffix {
                    text.push_str(s);
                }
                Doc::text(text)
            }
            Self::BooleanLiteral(b) => Doc::text(if *b { "true" } else { "false" }),
            Self::Identifier(id) => Doc::text(id.value()),
            Self::UnaryOp { op, expr } => {
                Doc::concat(vec![op.format(ctx), expr.value().format(ctx)])
            }
            Self::BinaryOp { op, left, right } => Doc::group(Doc::concat(vec![
                left.value().format(ctx),
                Doc::softline(),
                op.format(ctx),
                Doc::softline(),
                right.value().format(ctx),
            ])),
            Self::FunctionCall { callee, args } => {
                let arg_docs = args
                    .iter()
                    .map(|a| a.value().format(ctx))
                    .collect::<Vec<_>>();

                Doc::concat(vec![
                    callee.value().format(ctx),
                    parens(comma_separated(arg_docs)),
                ])
            }
            Self::IndexAccess { array, index } => Doc::concat(vec![
                array.value().format(ctx),
                Doc::text("["),
                index.value().format(ctx),
                Doc::text("]"),
            ]),
            Self::MemberAccess { object, field } => Doc::concat(vec![
                object.value().format(ctx),
                Doc::text("."),
                Doc::text(field.value()),
            ]),
            Self::Tuple(elements) => {
                let elem_docs = elements
                    .iter()
                    .map(|e| e.value().format(ctx))
                    .collect::<Vec<_>>();
                parens(comma_separated(elem_docs))
            }
            Self::StructLiteral { name, fields } => {
                let field_docs = fields
                    .iter()
                    .map(|(field_name, field_value)| {
                        Doc::concat(vec![
                            Doc::text(field_name.value()),
                            Doc::text(": "),
                            field_value.value().format(ctx),
                        ])
                    })
                    .collect::<Vec<_>>();

                Doc::concat(vec![
                    Doc::text(name.value()),
                    Doc::text(" "),
                    braces(comma_separated(field_docs)),
                ])
            }
            Self::TupleIndex { tuple, index } => Doc::concat(vec![
                tuple.value().format(ctx),
                Doc::text("."),
                Doc::text(index.to_string()),
            ]),
            Self::ArrayLiteral(elements) => {
                let elem_docs = elements
                    .iter()
                    .map(|e| e.value().format(ctx))
                    .collect::<Vec<_>>();
                Doc::concat(vec![
                    Doc::text("["),
                    comma_separated(elem_docs),
                    Doc::text("]"),
                ])
            }
            Self::Cast { expr, target_type } => Doc::concat(vec![
                expr.value().format(ctx),
                Doc::text(" as "),
                target_type.value().format(ctx),
            ]),
            Self::Range { start, end } => Doc::concat(vec![
                start.value().format(ctx),
                Doc::text(".."),
                end.value().format(ctx),
            ]),
            Self::Parenthesized(inner) => parens(inner.value().format(ctx)),
        }
    }
}

impl Format for BinaryOp {
    fn format(&self, _ctx: &mut FormatterCtx) -> Doc {
        let op_text = match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Less => "<",
            Self::Greater => ">",
            Self::LessEqual => "<=",
            Self::GreaterEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
            Self::BitwiseAnd => "&",
            Self::BitwiseOr => "|",
            Self::BitwiseXor => "^",
        };
        Doc::text(op_text)
    }
}

impl Format for UnaryOp {
    fn format(&self, _ctx: &mut FormatterCtx) -> Doc {
        let op_text = match self {
            Self::Not => "!",
            Self::Neg => "-",
        };
        Doc::text(op_text)
    }
}
