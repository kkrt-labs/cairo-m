#[derive(Clone, Debug)]
pub struct FormatterConfig {
    /// Maximum line width
    pub max_width: usize,
    /// Indentation width in spaces
    pub indent_width: u32,
    /// Whether to use trailing commas
    pub trailing_comma: bool,
    /// Line ending style
    pub newline_style: NewlineStyle,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            max_width: 100,
            indent_width: 4,
            trailing_comma: false,
            newline_style: NewlineStyle::Auto,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum NewlineStyle {
    /// Use the existing line endings
    Auto,
    /// Unix-style line endings (\n)
    Unix,
    /// Windows-style line endings (\r\n)
    Windows,
}
