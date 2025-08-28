use cairo_m_compiler_parser::{ParserDatabaseImpl, SourceFile};
use cairo_m_formatter::{format_source_file, FormatterConfig};

fn format_code(source: &str) -> String {
    let db = ParserDatabaseImpl::default();
    let file = SourceFile::new(&db, source.to_string(), "test.cm".to_string());
    let config = FormatterConfig::default();
    format_source_file(&db, file, &config)
}

// Parentheses preservation and precedence-sensitive formatting
#[test]
fn test_parentheses_preserved_in_binary_expression() {
    let input = r#"fn test(x: felt) -> felt { let y = (x+1)*2; return y; }"#;
    let expected = "fn test(x: felt) -> felt {\n    let y = (x + 1) * 2;\n    return y;\n}\n";
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_nested_parentheses_and_casts_preserved() {
    let input =
        r#"fn test() -> felt { let z = ((((a+b) as felt)*c as u32)-d) as felt; return z; }"#;
    let expected = "fn test() -> felt {\n    let z = ((((a + b) as felt) * c as u32) - d) as felt;\n    return z;\n}\n";
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_if_condition_parentheses_removed() {
    let input = r#"fn test() -> felt { if (x==y) { return 1; } else { return 0; } return 0; }"#;
    let expected = "fn test() -> felt {\n    if x == y {\n        return 1;\n    } else {\n        return 0;\n    }\n    return 0;\n}\n";
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_if_condition_no_parentheses() {
    let input = r#"fn test() -> felt { if x==y { return 1; } else { return 0; } return 0; }"#;
    let expected = "fn test() -> felt {\n    if x == y {\n        return 1;\n    } else {\n        return 0;\n    }\n    return 0;\n}\n";
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_idempotence() {
    let input = r#"fn test(x:felt)->felt{let y=x+1;return y;}"#;
    let formatted_once = format_code(input);
    let formatted_twice = format_code(&formatted_once);
    assert_eq!(
        formatted_once, formatted_twice,
        "Formatting should be idempotent"
    );
}

#[test]
fn test_while_condition_with_and_without_parentheses() {
    let with_parens = r#"fn test(){while(i!=n){i=i+1;}}"#;
    let without_parens = r#"fn test(){while i!=n{i=i+1;}}"#;
    let expected_with = "fn test() -> () {\n    while i != n {\n        i = i + 1;\n    }\n}\n";
    let expected_without = "fn test() -> () {\n    while i != n {\n        i = i + 1;\n    }\n}\n";
    assert_eq!(format_code(with_parens), expected_with);
    assert_eq!(format_code(without_parens), expected_without);
}

#[test]
fn test_should_not_format_unparsable_code() {
    let input = r#"fn test(x:felt)->felt{let z; let y=x+1;return y;}"#;
    let formatted = format_code(input);
    assert_eq!(input, formatted);
}

#[test]
fn test_keeping_operator_parentheses_nested() {
    let input = r#"
/// SHA-256 Maj (majority) function: majority vote of x, y, z.
fn maj(x: u32, y: u32, z: u32) -> u32 {
    return (x & y) ^ (x & z) ^ (y & z);
}
"#;
    let expected = r#"/// SHA-256 Maj (majority) function: majority vote of x, y, z.
fn maj(x: u32, y: u32, z: u32) -> u32 {
    return (x & y) ^ (x & z) ^ (y & z);
}
"#;
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_for_in_loop_parentheses_preserved() {
    let input = r#"fn test() -> () { for (i in 0..10) { let x = i; } }"#;
    let expected = "fn test() -> () {\n    for (i in 0..10) {\n        let x = i;\n    }\n}\n";
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_for_in_loop_parentheses_preserved_nested() {
    let input = r#"fn test() -> () { for (i in 0..10) { for (j in 0..5) { let x = i + j; } } }"#;
    let expected = "fn test() -> () {\n    for (i in 0..10) {\n        for (j in 0..5) {\n            let x = i + j;\n        }\n    }\n}\n";
    assert_eq!(format_code(input), expected);
}

#[test]
fn test_classic_for_loop_parentheses_preserved() {
    let input = r#"fn test() -> () { for (let i = 0; i < 10; i = i + 1) { let x = i; } }"#;
    let expected = "fn test() -> () {\n    for (let i = 0; i < 10; i = i + 1;) {\n        let x = i;\n    }\n}\n";
    assert_eq!(format_code(input), expected);
}
