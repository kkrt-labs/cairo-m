# Multiple Functions in Cairo-M

Complex function interactions, calling patterns, and multi-function programs.

## Basic Multiple Functions

Multiple functions in a single module:

```cairo-m
fn test_multiple_functions() -> felt {
    return first() + second() + third();
}

fn first() -> felt {
    return 10;
}

fn second() -> felt {
    return 20;
}

fn third() -> felt {
    return 30;
}
```

```rust
fn test_multiple_functions() -> i32 {
    return first() + second() + third();
}

fn first() -> i32 {
    return 10;
}

fn second() -> i32 {
    return 20;
}

fn third() -> i32 {
    return 30;
}
```

## Function Call Chain

Functions calling other functions in a chain:

```cairo-m
fn test_call_chain() -> felt {
    return compute(7);
}

fn compute(x: felt) -> felt {
    return add(mul(x, 3), mul(x, 5));
}

fn add(a: felt, b: felt) -> felt {
    return a + b;
}

fn mul(a: felt, b: felt) -> felt {
    return a * b;
}
```

```rust
fn test_call_chain() -> i32 {
    return compute(7);
}

fn compute(x: i32) -> i32 {
    return add(mul(x, 3), mul(x, 5));
}

fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn mul(a: i32, b: i32) -> i32 {
    return a * b;
}
```

## Helper Functions

Functions that work together to solve a problem:

```cairo-m
fn test_helpers() -> felt {
    let x = 5;
    let y = 3;
    return calculate_result(x, y);
}

fn calculate_result(a: felt, b: felt) -> felt {
    let sum = add_values(a, b);
    let product = multiply_values(a, b);
    return combine_results(sum, product);
}

fn add_values(x: felt, y: felt) -> felt {
    return x + y;
}

fn multiply_values(x: felt, y: felt) -> felt {
    return x * y;
}

fn combine_results(sum: felt, product: felt) -> felt {
    return sum * 10 + product;
}
```

```rust
fn test_helpers() -> i32 {
    let x = 5;
    let y = 3;
    return calculate_result(x, y);
}

fn calculate_result(a: i32, b: i32) -> i32 {
    let sum = add_values(a, b);
    let product = multiply_values(a, b);
    return combine_results(sum, product);
}

fn add_values(x: i32, y: i32) -> i32 {
    return x + y;
}

fn multiply_values(x: i32, y: i32) -> i32 {
    return x * y;
}

fn combine_results(sum: i32, product: i32) -> i32 {
    return sum * 10 + product;
}
```

## Mutual Recursion

Functions that call each other recursively:

```cairo-m
fn test_mutual_recursion() -> felt {
    let n = 5;
    return is_even(n) * 10 + is_odd(n);
}

fn is_even(n: felt) -> felt {
    if n == 0 {
        return 1;
    }
    return is_odd(n - 1);
}

fn is_odd(n: felt) -> felt {
    if n == 0 {
        return 0;
    }
    return is_even(n - 1);
}
```

```rust
fn test_mutual_recursion() -> i32 {
    let n = 5;
    return is_even(n) * 10 + is_odd(n);
}

fn is_even(n: i32) -> i32 {
    if n == 0 {
        return 1;
    }
    return is_odd(n - 1);
}

fn is_odd(n: i32) -> i32 {
    if n == 0 {
        return 0;
    }
    return is_even(n - 1);
}
```

## Complex Call Patterns

Functions with multiple parameters and complex interactions:

<!-- TODO: implement U32Eq -->

```cairo-m
//! ignore: U32Eq not implemented yet
fn test_complex_calls() -> u32 {
    let base = get_base_value();
    let multiplier = calculate_multiplier(base);
    return apply_transformation(base, multiplier);
}

fn get_base_value() -> u32 {
    return 15;
}

fn calculate_multiplier(base: u32) -> u32 {
    if base > 10u32 {
        return process_large(base);
    } else {
        return process_small(base);
    }
}

fn process_large(value: u32) -> u32 {
    return value / 3u32;
}

fn process_small(value: u32) -> u32 {
    return value * 2u32;
}

fn apply_transformation(base: u32, multiplier: u32) -> u32 {
    let intermediate = base + multiplier;
    return finalize_result(intermediate);
}

fn finalize_result(value: u32) -> u32 {
    return value * value;
}
```

## Mathematical Operations

Functions working together to perform complex mathematical operations:

```cairo-m
fn test_math_operations() -> felt {
    let x = 4;
    let y = 3;
    return compute_expression(x, y);
}

fn compute_expression(a: felt, b: felt) -> felt {
    let power_result = power(a, b);
    let factorial_result = factorial(b);
    return power_result + factorial_result;
}

fn power(base: felt, exp: felt) -> felt {
    if exp == 0 {
        return 1;
    }
    return base * power(base, exp - 1);
}

fn factorial(n: felt) -> felt {
    if n == 0 {
        return 1;
    }
    return n * factorial(n - 1);
}
```

```rust
fn test_math_operations() -> i32 {
    let x = 4;
    let y = 3;
    return compute_expression(x, y);
}

fn compute_expression(a: i32, b: i32) -> i32 {
    let power_result = power(a, b);
    let factorial_result = factorial(b);
    return power_result + factorial_result;
}

fn power(base: i32, exp: i32) -> i32 {
    if exp == 0 {
        return 1;
    }
    return base * power(base, exp - 1);
}

fn factorial(n: i32) -> i32 {
    if n == 0 {
        return 1;
    }
    return n * factorial(n - 1);
}
```
