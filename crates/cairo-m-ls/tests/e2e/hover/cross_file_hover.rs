use super::super::support::{extract_cursors, Fixture};
use crate::hover::HoverTransformer;
use crate::support::insta::test_transform;

#[tokio::test]
async fn test_hover_on_imported_function() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "src/main.cm",
        r#"
use utils::helper_foo;

fn main() {
    let result = helper_foo(42);
}
"#,
    );
    fixture.add_file(
        "src/utils.cm",
        r#"
fn helper_foo(x: felt) -> felt {
    return x * 2;
}
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use utils::helper_foo;

fn main() {
    let result = helper_<caret>foo(42);
}
"#,
    );

    test_transform!(HoverTransformer, fixture, cursors, |result: &str| {
        assert!(result.contains("function"), "{}", result);
    });
}

#[ignore = "TODO: Type info not available for user-defined types"]
#[tokio::test]
async fn test_hover_on_imported_type() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "src/main.cm",
        r#"
use types::CustomType;

fn main() {
    let value: CustomType = CustomType { value: 42 };
}
"#,
    );
    fixture.add_file(
        "src/types.cm",
        r#"
struct CustomType {
    value: felt,
}
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use types::CustomType;

fn main() {
    let value: Custom<caret>Type = CustomType { value: 42 };
}
"#,
    );

    test_transform!(HoverTransformer, fixture, cursors, |result: &str| {
        assert!(result.contains("felt"), "{}", result);
    });
}

#[tokio::test]
async fn test_hover_on_module_name() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "src/main.cm",
        r#"
use utils::calculate;

fn main() {
    let result = calculate(10, 20);
}
"#,
    );
    fixture.add_file(
        "src/utils.cm",
        r#"
fn calculate(a: felt, b: felt) -> felt {
    a + b
}

fn multiply(a: felt, b: felt) -> felt {
    a * b
}
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use util<caret>s::calculate;

fn main() {
    let result = calculate(10, 20);
}
"#,
    );

    test_transform!(HoverTransformer, fixture, cursors, |result: &str| {
        assert!(result.contains("No hover info"), "{}", result);
    });
}

#[tokio::test]
async fn test_hover_on_imported_constant() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "src/main.cm",
        r#"
use constants::MAX_VALUE;

fn main() {
    let limit = MAX_VALUE;
    return();
}
"#,
    );
    fixture.add_file(
        "src/constants.cm",
        r#"
const MAX_VALUE = 1000;
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use constants::MAX_VALUE;

fn main() {
    let limit = MAX_<caret>VALUE;
    return();
}
"#,
    );

    test_transform!(HoverTransformer, fixture, cursors, |result: &str| {
        assert!(result.contains("felt"), "{}", result);
    });
}
