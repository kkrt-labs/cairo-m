# Complex Expressions in Cairo-M

Cairo-M supports complex expressions combining arithmetic, logical operations,
comparisons, and precedence rules.

## Binary Operations

Basic binary operations between operands:

```cairo-m
fn test_binary_ops() -> felt {
    let a = 14;
    let b = 7;

    let add_result = a + b;        // 22
    let sub_result = a - b;        // 8
    let mul_result = a * b;        // 105
    let div_result = a / b;        // Field division

    return add_result + sub_result + mul_result + div_result;
}
```

```rust
fn test_binary_ops() -> i32 {
    let a = 14;
    let b = 7;

    let add_result = a + b;        // 22
    let sub_result = a - b;        // 8
    let mul_result = a * b;        // 105
    let div_result = a / b;        // Integer division

    return add_result + sub_result + mul_result + div_result;
}
```

## Operator Precedence

Mathematical operator precedence is respected:

```cairo-m
fn test_precedence() -> felt {
    let a = 5;
    let b = 3;
    let c = 2;

    // Multiplication happens before addition: 5 + (3 * 2) = 11
    let result1 = a + b * c;

    // Parentheses override precedence: (5 + 3) * 2 = 16
    let result2 = (a + b) * c;

    // Complex precedence: 5 * 3 + 2 * 4 = 23
    let result3 = a * b + c * 4;

    return result1 + result2 + result3;  // 11 + 16 + 23 = 50
}
```

```rust
fn test_precedence() -> i32 {
    let a = 5;
    let b = 3;
    let c = 2;

    // Multiplication happens before addition: 5 + (3 * 2) = 11
    let result1 = a + b * c;

    // Parentheses override precedence: (5 + 3) * 2 = 16
    let result2 = (a + b) * c;

    // Complex precedence: 5 * 3 + 2 * 4 = 23
    let result3 = a * b + c * 4;

    return result1 + result2 + result3;  // 11 + 16 + 23 = 50
}
```

## Unary Operations

### Negation

```cairo-m
fn test_unary_ops() -> felt {
    let x = 10;
    let neg_x = -x;           // -10 in field arithmetic
    let double_neg = --x;     // 10 (double negation)
    let neg_lit = -10;
    return double_neg + neg_lit;
}
```

### Logical Not

```cairo-m
fn test_logical_not() -> bool {
    let x = true;
    let not_x = !x;           // false
    let double_not = !!x;     // true
    let not_zero = !false;        // true
    return not_zero && double_not;
}
```

## Comparison Operations

Comparison operators in expressions:

```cairo-m
fn test_comparisons() -> u32 {
    let a = 10u32;
    let b = 7u32;
    let c = 10u32;

    let eq = (a == c);        // true
    let ne = (a != b);        // true
    let lt = (b < a);         // true
    let le = (a <= c);        // true
    let gt = (a > b);         // true
    let ge = (c >= a);        // true

    let res = 0u32;
    if eq {
        res = res + 1u32;
    }
    if ne {
        res = res + 1u32;
    }
    if lt {
        res = res + 1u32;
    }
    if le {
        res = res + 1u32;
    }
    if gt {
        res = res + 1u32;
    }
    if ge {
        res = res + 1u32;
    }
    return res;
}
```

## Logical Operations

Logical AND and OR operations:

```cairo-m
fn test_logical_ops() -> felt {
    let a = 5;
    let b = 0;
    let c = 3;

    // Logical AND: both operands must be non-zero
    let and_result1 = (a != 0) && (c != 0);  // 1 (true && true)
    let and_result2 = (a != 0) && (b != 0);  // 0 (true && false)

    // Logical OR: at least one operand must be non-zero
    let or_result1 = (a != 0) || (b != 0);   // 1 (true || false)
    let or_result2 = (b != 0) || (b != 0);   // 0 (false || false)

    let res = 0;
    if and_result1 {
        res = res + 1;
    }
    if and_result2 {
        res = res + 1;
    }
    if or_result1 {
        res = res + 1;
    }
    if or_result2 {
        res = res + 1;
    }
    return res;
}
```

## Compound Expressions

Complex expressions with multiple operations:

```cairo-m
fn test_compound_expressions() -> u32 {
    let x: u32 = 8;
    let y: u32 = 3;
    let z: u32 = 12;

    // Complex arithmetic with precedence
    let expr1 = x * y + z / 4u32;                    // 8 * 3 + 12 / 4 = 24 + 3 = 27

    // Nested parentheses
    let expr2 = (x + y) * (z - y * 2u32);            // (8 + 3) * (12 - 3 * 2) = 11 * 6 = 66

    // Mixed arithmetic and comparison
    let expr3 = (x > y) && ((z + x) < 25u32);        // (8 > 3) && ((12 + 8) < 25) = 1 && 1 = 1

    let res = expr1 + expr2;
    if expr3 {
        res = res + 1u32;
    }
    return res;
}
```

## Nested Function Calls in Expressions

Expressions with function calls:

```cairo-m
fn compute_base(x: felt) -> felt {
    return x * 2;
}

fn compute_offset(y: felt) -> felt {
    return y + 5;
}

fn test_function_expressions() -> felt {
    let a = 7;
    let b = 3;

    // Function calls in expressions
    let result1 = compute_base(a) + compute_offset(b);     // (7 * 2) + (3 + 5) = 14 + 8 = 22
    let result2 = compute_base(a + b) - compute_offset(1); // (10 * 2) - (1 + 5) = 20 - 6 = 14

    // Nested function calls
    let result3 = compute_base(compute_offset(a));         // compute_base(7 + 5) = compute_base(12) = 24

    return result1 + result2 + result3;                    // 22 + 14 + 24 = 60
}
```

## Expression Evaluation Order

Left-to-right evaluation with proper precedence:

```cairo-m
fn test_evaluation_order() -> felt {
    let a = 2;
    let b = 3;
    let c = 4;
    let d = 5;

    // Test associativity: left-to-right for same precedence
    let left_assoc = a + b + c + d;               // ((2 + 3) + 4) + 5 = 14
    let mult_assoc = a * b * c;                   // (2 * 3) * 4 = 24

    // Mixed precedence evaluation
    let mixed = a + b * c + d;                    // 2 + (3 * 4) + 5 = 2 + 12 + 5 = 19
    let complex_mixed = a * b + c * d;            // (2 * 3) + (4 * 5) = 6 + 20 = 26

    return left_assoc + mult_assoc + mixed + complex_mixed; // 14 + 24 + 19 + 26 = 83
}
```

```rust
fn test_evaluation_order() -> i32 {
    let a = 2;
    let b = 3;
    let c = 4;
    let d = 5;

    // Test associativity: left-to-right for same precedence
    let left_assoc = a + b + c + d;               // ((2 + 3) + 4) + 5 = 14
    let mult_assoc = a * b * c;                   // (2 * 3) * 4 = 24

    // Mixed precedence evaluation
    let mixed = a + b * c + d;                    // 2 + (3 * 4) + 5 = 2 + 12 + 5 = 19
    let complex_mixed = a * b + c * d;            // (2 * 3) + (4 * 5) = 6 + 20 = 26

    return left_assoc + mult_assoc + mixed + complex_mixed; // 14 + 24 + 19 + 26 = 83
}
```

## Expression Side Effects

Expressions should not have side effects in Cairo-M:

```cairo-m
fn pure_expression() -> felt {
    let x = 10;
    let y = 20;

    // All operations here are pure - no side effects
    let result = (x + y) * (x - y) / 2 + x * y;  // (10 + 20) * (10 - 20) / 2 + 10 * 20
                                                   // = 30 * (-10) / 2 + 200
                                                   // = -300 / 2 + 200
                                                   // = -150 + 200 = 50
    return result;
}
```

```rust
fn pure_expression() -> i32 {
    let x = 10;
    let y = 20;

    // All operations here are pure - no side effects
    let result = (x + y) * (x - y) / 2 + x * y;  // (10 + 20) * (10 - 20) / 2 + 10 * 20
                                                   // = 30 * (-10) / 2 + 200
                                                   // = -300 / 2 + 200
                                                   // = -150 + 200 = 50
    return result;
}
```
