use crate::doc::Doc;

/// Helper function to create a space-separated list
pub fn space_separated(items: Vec<Doc>) -> Doc {
    Doc::join(Doc::text(" "), items)
}

/// Helper function to create a comma-separated list
pub fn comma_separated(items: Vec<Doc>) -> Doc {
    Doc::join(Doc::text(", "), items)
}

/// Helper function to wrap in parentheses
pub fn parens(inner: Doc) -> Doc {
    Doc::concat(vec![Doc::text("("), inner, Doc::text(")")])
}

/// Helper function to wrap in braces
pub fn braces(inner: Doc) -> Doc {
    Doc::concat(vec![Doc::text("{"), inner, Doc::text("}")])
}

/// Helper function to wrap in brackets
pub fn brackets(inner: Doc) -> Doc {
    Doc::concat(vec![Doc::text("["), inner, Doc::text("]")])
}
