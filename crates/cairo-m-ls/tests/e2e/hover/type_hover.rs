use super::super::support::insta::test_transform;
use super::HoverTransformer;

#[tokio::test]
async fn test_variable_type_hover() {
    test_transform!(
        HoverTransformer,
        r#"
func main() {
    let x: felt = 42;
    let y = x<caret>;
    return y;
}
"#
    );
}

#[tokio::test]
async fn test_function_signature_hover() {
    test_transform!(
        HoverTransformer,
        r#"
func add(a: felt, b: felt) -> felt {
    return a + b;
}

func main() {
    let sum = ad<caret>d(1, 2);
    return sum;
}
"#
    );
}

#[tokio::test]
async fn test_parameter_type_hover() {
    test_transform!(
        HoverTransformer,
        r#"
func process(value<caret>: felt) -> felt {
    return value * 2;
}
"#
    );
}

#[tokio::test]
async fn test_return_type_hover() {
    test_transform!(
        HoverTransformer,
        r#"
func calculate() -> fe<caret>lt {
    return 42;
}
"#
    );
}

#[tokio::test]
async fn test_hover_on_type_annotation() {
    test_transform!(
        HoverTransformer,
        r#"
func main() {
    let x: fe<caret>lt = 100;
    return x;
}
"#
    );
}

#[tokio::test]
async fn test_hover_on_function_call_result() {
    test_transform!(
        HoverTransformer,
        r#"
func get_value() -> felt {
    42
}

func main() {
    let result = get_va<caret>lue();
}
"#
    );
}

#[tokio::test]
async fn test_hover_no_info() {
    test_transform!(
        HoverTransformer,
        r#"
func main() {
    // Hovering on whitespace should return no info
    let x = 42;   <caret>
}
"#
    );
}

#[tokio::test]
async fn test_hover_on_binary_expression() {
    test_transform!(
        HoverTransformer,
        r#"
func main() {
    let a: felt = 10;
    let b: felt = 20;
    let sum = a + <caret>b;
}
"#
    );
}

#[tokio::test]
async fn test_hover_on_struct_field() {
    test_transform!(
        HoverTransformer,
        r#"
struct Point {
    x: felt,
    y: felt,
}

func main() {
    let p = Point { x: 10, y: 20 };
    let x_val = p.x<caret>;
}
"#
    );
}
