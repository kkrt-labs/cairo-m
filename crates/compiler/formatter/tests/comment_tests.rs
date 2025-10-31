#[cfg(test)]
mod tests {
    use cairo_m_compiler_parser::{ParserDatabaseImpl, SourceFile};
    use cairo_m_formatter::{FormatterConfig, format_source_file};

    fn format_code(input: &str) -> String {
        let db = ParserDatabaseImpl::default();
        let source = SourceFile::new(&db, input.to_string(), "test.cm".to_string());
        let config = FormatterConfig::default();
        format_source_file(&db, source, &config)
    }

    #[test]
    fn test_preserve_file_level_comments() {
        let input = r#"// This is a function that adds two numbers
fn add(x: felt, y: felt) -> felt {
    return x + y;
}"#;

        let expected = r#"// This is a function that adds two numbers
fn add(x: felt, y: felt) -> felt {
    return x + y;
}
"#;

        let formatted = format_code(input);
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_preserve_inline_comments() {
        let input = r#"// This is a function that adds two numbers
fn add(x: felt, y: felt) -> felt {
    // Add the two values
    return x + y; // Return the sum
}"#;

        let expected = r#"// This is a function that adds two numbers
fn add(x: felt, y: felt) -> felt {
    // Add the two values
    return x + y; // Return the sum
}
"#;

        let formatted = format_code(input);
        assert_eq!(formatted, expected);
    }

    #[test]
    #[ignore = "Full comment support requires AST spans"]
    fn test_preserve_struct_comments() {
        let input = r#"// A point in 2D space
struct Point {
    x: felt, // X coordinate
    y: felt, // Y coordinate
}"#;

        let expected = r#"// A point in 2D space
struct Point {
    x: felt, // X coordinate
    y: felt, // Y coordinate
}
"#;

        let formatted = format_code(input);
        assert_eq!(formatted, expected);
    }

    #[test]
    #[ignore = "Full comment support requires AST spans"]
    fn test_preserve_standalone_comments() {
        let input = r#"fn main() -> felt {
    // Initialize variable
    let x = 5;

    // Do some computation
    // This is complex
    let y = x * 2;

    return y;
}"#;

        let expected = r#"fn main() -> felt {
    // Initialize variable
    let x = 5;
    // Do some computation
    // This is complex
    let y = x * 2;
    return y;
}
"#;

        let formatted = format_code(input);
        assert_eq!(formatted, expected);
    }
}
