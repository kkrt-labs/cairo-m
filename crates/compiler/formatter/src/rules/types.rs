use cairo_m_compiler_parser::parser::{NamedType, TypeExpr};

use crate::context::FormatterCtx;
use crate::doc::Doc;
use crate::utils::*;
use crate::Format;

impl Format for TypeExpr {
    fn format(&self, ctx: &mut FormatterCtx) -> Doc {
        match self {
            Self::Named(named) => named.value().format(ctx),
            Self::Pointer(inner) => Doc::concat(vec![inner.value().format(ctx), Doc::text("*")]),
            Self::Tuple(types) => {
                let type_docs = types
                    .iter()
                    .map(|t| t.value().format(ctx))
                    .collect::<Vec<_>>();
                parens(comma_separated(type_docs))
            }
            Self::FixedArray { element_type, size } => Doc::concat(vec![
                Doc::text("["),
                element_type.value().format(ctx),
                Doc::text("; "),
                Doc::text(size.value().to_string()),
                Doc::text("]"),
            ]),
        }
    }
}

impl Format for NamedType {
    fn format(&self, _ctx: &mut FormatterCtx) -> Doc {
        match self {
            Self::Felt => Doc::text("felt"),
            Self::Bool => Doc::text("bool"),
            Self::U32 => Doc::text("u32"),
            Self::Custom(name) => Doc::text(name),
        }
    }
}
