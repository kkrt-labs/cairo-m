use super::super::support::{Fixture, extract_cursors};
use crate::hover::HoverTransformer;
use crate::support::insta::test_transform;

#[tokio::test]
async fn test_hover_on_imported_function() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "main.cm",
        r#"
use utils::helper_foo;

func main() {
    let result = helper_foo(42);
}
"#,
    );
    fixture.add_file(
        "utils.cm",
        r#"
func helper_foo(x: felt) -> felt {
    return x * 2;
}
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use utils::helper_foo;

func main() {
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
        "main.cm",
        r#"
use types::CustomType;

func main() {
    let value: CustomType = CustomType { value: 42 };
}
"#,
    );
    fixture.add_file(
        "types.cm",
        r#"
struct CustomType {
    value: felt,
}
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use types::CustomType;

func main() {
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
        "main.cm",
        r#"
use utils::calculate;

func main() {
    let result = calculate(10, 20);
}
"#,
    );
    fixture.add_file(
        "utils.cm",
        r#"
func calculate(a: felt, b: felt) -> felt {
    a + b
}

func multiply(a: felt, b: felt) -> felt {
    a * b
}
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use util<caret>s::calculate;

func main() {
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
        "main.cm",
        r#"
use constants::MAX_VALUE;

func main() {
    let limit = MAX_VALUE;
}
"#,
    );
    fixture.add_file(
        "constants.cm",
        r#"
const MAX_VALUE = 1000;
"#,
    );

    let (_, cursors) = extract_cursors(
        r#"
use constants::MAX_VALUE;

func main() {
    let limit = MAX_<caret>VALUE;
}
"#,
    );

    test_transform!(HoverTransformer, fixture, cursors, |result: &str| {
        assert!(result.contains("felt"), "{}", result);
    });
}
