use cairo_m_compiler_parser::{ParserDatabaseImpl, SourceFile};
use cairo_m_formatter::{FormatterConfig, format_source_file};
use insta::assert_snapshot;

fn format_code(source: &str) -> String {
    let db = ParserDatabaseImpl::default();
    let file = SourceFile::new(&db, source.to_string(), "test.cm".to_string());
    let config = FormatterConfig::default();
    format_source_file(&db, file, &config)
}

#[test]
fn test_format_simple_function() {
    let input = r#"fn   add(x:felt,y:felt)->felt{let result=x+y;return result;}"#;
    assert_snapshot!(format_code(input));
}

#[test]
fn test_format_struct() {
    let input = r#"struct Point{x:felt,y:felt,}"#;
    assert_snapshot!(format_code(input));
}

#[test]
fn test_format_if_statement() {
    let input = r#"fn test(x:felt)->felt{if x==0{return 1;}else{return x;}}"#;
    assert_snapshot!(input, format_code(input));
}

#[test]
fn test_format_namespace() {
    let input = r#"namespace math{fn square(x:felt)->felt{return x*x;}}"#;
    assert_snapshot!(format_code(input));
}

#[test]
fn test_format_const() {
    let input = r#"const PI=314;"#;
    assert_snapshot!(format_code(input));
}

#[test]
fn test_format_use_statement() {
    let input = r#"use std::math::sqrt;"#;
    assert_snapshot!(format_code(input));
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
fn test_should_not_format_unparseable_code() {
    let input = r#"fn test(x:felt)->felt{let z; let y=x+1;return y;}"#;
    let formatted = format_code(input);
    assert_eq!(input, formatted);
}
