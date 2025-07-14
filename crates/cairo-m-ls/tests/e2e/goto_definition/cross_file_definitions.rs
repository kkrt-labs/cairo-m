use super::{GotoDefinition, NO_DEFINITION_FOUND};
use crate::support::{Transformer, extract_cursors, sandbox};

#[tokio::test]
async fn goto_imported_function_definition() {
    let mut ls = sandbox! {
        files {
            "cairom.toml" => r#"
[project]
name = "test_project"
"#,
            "main.cm" => r#"
use math::add;

func main() {
    let result = <caret>add(3, 4);
    return result;
}
"#,
            "math.cm" => r#"
func add(a: felt, b: felt) -> felt {
    return a + b;
}
"#
        }
    };

    ls.open_and_wait_for_analysis("main.cm").await.unwrap();
    let (_code, cursors) = extract_cursors(
        r#"
use math::add;

func main() {
    let result = <caret>add(3, 4);
    return result;
}
"#,
    );

    let result = GotoDefinition::transform(&mut ls, cursors, None)
        .await
        .unwrap();
    // assert_ne!(result, NO_DEFINITION_FOUND);
    ::insta::assert_snapshot!(result);
}

#[tokio::test]
async fn goto_module_definition() {
    let mut ls = sandbox! {
        files {
            "cairom.toml" => r#"
[project]
name = "test_project"
"#,
            "main.cm" => r#"
use <caret>utils::helper;

func main() {
    helper();
}
"#,
            "utils.cm" => r#"
func helper() -> felt {
    42
}
"#
        }
    };

    ls.open_and_wait_for_analysis("main.cm").await.unwrap();
    let (_code, cursors) = extract_cursors(
        r#"
use <caret>utils;

func main() {
    helper();
}
"#,
    );

    let result = GotoDefinition::transform(&mut ls, cursors, None)
        .await
        .unwrap();
    // assert_ne!(result, NO_DEFINITION_FOUND);
    ::insta::assert_snapshot!(result);
}

//TODO: fix this as GOTO is not working for external types
#[ignore = "GOTO is not working for external types"]
#[tokio::test]
async fn goto_external_type_definition() {
    let mut ls = sandbox! {
        files {
            "cairom.toml" => r#"
[project]
name = "test_project"
"#,
            "main.cm" => r#"
use types::Point;

func main() {
    let p: <caret>Point = Point { x: 1, y: 2 };
}
"#,
            "types.cm" => r#"
struct Point {
    x: felt,
    y: felt,
}
"#
        }
    };

    ls.open_and_wait_for_analysis("main.cm").await.unwrap();
    let (_code, cursors) = extract_cursors(
        r#"
use types::Point;

func main() {
    let p: <caret>Point = Point { x: 1, y: 2 };
}
"#,
    );

    let result = GotoDefinition::transform(&mut ls, cursors, None)
        .await
        .unwrap();
    // assert_ne!(result, NO_DEFINITION_FOUND);
    ::insta::assert_snapshot!(result);
}

//TODO: fix this as project discovery is not working with folders yet
#[ignore = "project discovery is not working with folders yet"]
#[tokio::test]
async fn goto_deeply_nested_import() {
    let mut ls = sandbox! {
        files {
            "cairom.toml" => r#"
[project]
name = "test_project"
"#,
            "main.cm" => r#"
use math::ops::add;

func main() {
    let result = <caret>add(1, 2);
}
"#,
    "math/ops.cm" => r#"
func add(a: felt, b: felt) -> felt {
    a + b
}
"#
        }
    };

    ls.open_and_wait_for_analysis("main.cm").await.unwrap();
    let (_code, cursors) = extract_cursors(
        r#"
use math::ops::add;

func main() {
    let result = <caret>add(1, 2);
}
"#,
    );

    let result = GotoDefinition::transform(&mut ls, cursors, None)
        .await
        .unwrap();
    assert_ne!(result, NO_DEFINITION_FOUND);
    ::insta::assert_snapshot!(result);
}

#[tokio::test]
async fn no_definition_cross_file_unresolved() {
    let mut ls = sandbox! {
        files {
            "cairom.toml" => r#"
[project]
name = "test_project"
"#,
            "main.cm" => r#"
use nonexistent::function;

func main() {
    <caret>function();
}
"#
        }
    };

    ls.open_and_wait_for_analysis("main.cm").await.unwrap();
    let (_code, cursors) = extract_cursors(
        r#"
use nonexistent::function;

func main() {
    <caret>function();
}
"#,
    );

    let result = GotoDefinition::transform(&mut ls, cursors, None)
        .await
        .unwrap();
    assert_eq!(result, NO_DEFINITION_FOUND);
    ::insta::assert_snapshot!(result);
}
