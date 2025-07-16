use super::super::support::insta::test_transform;
use super::GotoDefinition;

#[tokio::test]
async fn goto_variable_definition() {
    test_transform!(
        GotoDefinition,
        r#"
func main() {
    let x = 42;
    let y = <caret>x + 1;
    return y;
}
"#
    );
}

#[tokio::test]
async fn goto_function_definition() {
    test_transform!(
        GotoDefinition,
        r#"
func add(a: felt, b: felt) -> felt {
    return a + b;
}

func main() {
    let result = <caret>add(3, 4);
    return result;
}
"#
    );
}

#[tokio::test]
async fn goto_parameter_definition() {
    test_transform!(
        GotoDefinition,
        r#"
func calculate(value: felt) -> felt {
    return <caret>value * 2;
}
"#
    );
}

#[tokio::test]
async fn goto_type_definition() {
    test_transform!(
        GotoDefinition,
        r#"
struct Point {
    x: felt,
    y: felt,
}

func main() {
    let p: <caret>Point = Point { x: 1, y: 2 };
    return p;
}
"#
    );
}

#[tokio::test]
async fn goto_field_definition() {
    test_transform!(
        GotoDefinition,
        r#"
struct Rectangle {
    width: felt,
    height: felt,
}

func main() {
    let rect = Rectangle { width: 10, height: 20 };
    let w = rect.<caret>width;
    return w;
}
"#
    );
}

#[tokio::test]
async fn goto_local_in_block() {
    test_transform!(
        GotoDefinition,
        r#"
func main() {
    let outer = 1;
    {
        let inner = 2;
        let sum = <caret>outer + inner;
    }
    return outer;
}
"#
    );
}

#[tokio::test]
async fn goto_loop_variable() {
    test_transform!(
        GotoDefinition,
        r#"
func main() {
    for i in 0..10 {
        let doubled = <caret>i * 2;
    }
}
"#
    );
}

#[tokio::test]
async fn goto_shadowed_variable() {
    test_transform!(
        GotoDefinition,
        r#"
func main() {
    let x = 1;
    let x = 2;
    let y = <caret>x + 1;  // Should go to the second definition
    return y;
}
"#
    );
}

#[tokio::test]
async fn no_definition_for_keyword() {
    test_transform!(
        GotoDefinition,
        r#"
func main() {
    <caret>let x = 42;
    return x;
}
"#
    );
}

#[tokio::test]
async fn no_definition_for_literal() {
    test_transform!(
        GotoDefinition,
        r#"
func main() {
    let x = <caret>42;
}
"#
    );
}

#[tokio::test]
async fn goto_function_in_call_chain() {
    test_transform!(
        GotoDefinition,
        r#"
func first() -> felt {
    1
}

func second() -> felt {
    2
}

func main() {
    let x = <caret>first() + second();
}
"#
    );
}

#[tokio::test]
async fn goto_nested_function_definition() {
    test_transform!(
        GotoDefinition,
        r#"
func outer() {
    func inner() -> felt {
        42
    }

    let result = <caret>inner();
}
"#
    );
}
