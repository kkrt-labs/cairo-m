//! Built-in functions registry and helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinFn {
    Assert,
}

/// Return true if the given identifier is a recognized built-in function name.
pub fn is_builtin_function_name(name: &str) -> Option<BuiltinFn> {
    match name {
        "assert" => Some(BuiltinFn::Assert),
        _ => None,
    }
}
