<!-- # Mutual Recursion in Cairo-M

Functions that call each other recursively to solve problems that require alternating logic or state transitions.

## Even and Odd Numbers

Classic mutual recursion to determine if numbers are even or odd:

```cairo-m
fn test_even_odd() -> felt {
    let n = 42;
    let even_result = is_even(n);
    let odd_result = is_odd(n);
    return even_result * 100 + odd_result;
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
fn test_even_odd() -> i64 {
    let n = 42;
    let even_result = is_even(n);
    let odd_result = is_odd(n);
    return even_result * 100 + odd_result;
}

fn is_even(n: i64) -> i64 {
    if n == 0 {
        return 1;
    }
    return is_odd(n - 1);
}

fn is_odd(n: i64) -> i64 {
    if n == 0 {
        return 0;
    }
    return is_even(n - 1);
}
```

## Binary Tree Traversal

Mutual recursion for processing different node types in a tree structure:

```cairo-m
fn test_tree_traversal() -> felt {
    let depth = 3;
    return process_internal_node(depth);
}

fn process_internal_node(depth: felt) -> felt {
    if depth == 0 {
        return process_leaf_node();
    }
    let left = process_leaf_node();
    let right = process_internal_node(depth - 1);
    return left + right + 1;
}

fn process_leaf_node() -> felt {
    return 5;
}
```

```rust
fn test_tree_traversal() -> i64 {
    let depth = 3;
    return process_internal_node(depth);
}

fn process_internal_node(depth: i64) -> i64 {
    if depth == 0 {
        return process_leaf_node();
    }
    let left = process_leaf_node();
    let right = process_internal_node(depth - 1);
    return left + right + 1;
}

fn process_leaf_node() -> i64 {
    return 5;
}
```

## State Machine Simulation

Mutual recursion to simulate a simple state machine with two states:

```cairo-m
fn test_state_machine() -> felt {
    let steps = 4;
    return state_a(steps);
}

fn state_a(remaining: felt) -> felt {
    if remaining == 0 {
        return 1;
    }
    return state_b(remaining - 1) + 10;
}

fn state_b(remaining: felt) -> felt {
    if remaining == 0 {
        return 2;
    }
    return state_a(remaining - 1) + 20;
}
```

```rust
fn test_state_machine() -> i64 {
    let steps = 4;
    return state_a(steps);
}

fn state_a(remaining: i64) -> i64 {
    if remaining == 0 {
        return 1;
    }
    return state_b(remaining - 1) + 10;
}

fn state_b(remaining: i64) -> i64 {
    if remaining == 0 {
        return 2;
    }
    return state_a(remaining - 1) + 20;
}
```

## Hofstadter Sequences

Mutual recursion implementing Hofstadter's Q and R sequences:

```cairo-m
fn test_hofstadter() -> felt {
    let n = 6;
    let q_result = hofstadter_q(n);
    let r_result = hofstadter_r(n);
    return q_result * 10 + r_result;
}

fn hofstadter_q(n: felt) -> felt {
    if n == 1 {
        return 1;
    }
    if n == 2 {
        return 1;
    }
    return hofstadter_q(n - hofstadter_q(n - 1)) + hofstadter_q(n - hofstadter_q(n - 2));
}

fn hofstadter_r(n: felt) -> felt {
    if n == 1 {
        return 1;
    }
    return hofstadter_r(n - 1) + hofstadter_q(n - 1);
}
```

```rust
fn test_hofstadter() -> i64 {
    let n = 6;
    let q_result = hofstadter_q(n);
    let r_result = hofstadter_r(n);
    return q_result * 10 + r_result;
}

fn hofstadter_q(n: i64) -> i64 {
    if n == 1 {
        return 1;
    }
    if n == 2 {
        return 1;
    }
    return hofstadter_q(n - hofstadter_q(n - 1)) + hofstadter_q(n - hofstadter_q(n - 2));
}

fn hofstadter_r(n: i64) -> i64 {
    if n == 1 {
        return 1;
    }
    return hofstadter_r(n - 1) + hofstadter_q(n - 1);
}
```

## Parser-like Mutual Recursion

Mutual recursion simulating a simple expression parser with different precedence levels:

```cairo-m
fn test_expression_parser() -> felt {
    let complexity = 3;
    return parse_expression(complexity);
}

fn parse_expression(level: felt) -> felt {
    if level == 0 {
        return 1;
    }
    let term_value = parse_term(level - 1);
    return term_value + parse_expression(level - 1);
}

fn parse_term(level: felt) -> felt {
    if level == 0 {
        return 2;
    }
    let factor_value = parse_factor(level);
    return factor_value * 2;
}

fn parse_factor(level: felt) -> felt {
    if level <= 1 {
        return 3;
    }
    return parse_expression(level - 2) + 1;
}
```

```rust
fn test_expression_parser() -> i64 {
    let complexity = 3;
    return parse_expression(complexity);
}

fn parse_expression(level: i64) -> i64 {
    if level == 0 {
        return 1;
    }
    let term_value = parse_term(level - 1);
    return term_value + parse_expression(level - 1);
}

fn parse_term(level: i64) -> i64 {
    if level == 0 {
        return 2;
    }
    let factor_value = parse_factor(level);
    return factor_value * 2;
}

fn parse_factor(level: i64) -> i64 {
    if level <= 1 {
        return 3;
    }
    return parse_expression(level - 2) + 1;
}
```

## Multi-Function Mutual Recursion

Complex mutual recursion with three functions calling each other:

```cairo-m
fn test_three_way_recursion() -> felt {
    let n = 2;
    return func_a(n);
}

fn func_a(n: felt) -> felt {
    if n == 0 {
        return 1;
    }
    return func_b(n - 1) + func_c(n - 1);
}

fn func_b(n: felt) -> felt {
    if n == 0 {
        return 2;
    }
    if n == 1 {
        return func_a(0);
    }
    return func_c(n - 1) + 1;
}

fn func_c(n: felt) -> felt {
    if n == 0 {
        return 3;
    }
    return func_a(n - 1) + func_b(n - 2);
}
```

```rust
fn test_three_way_recursion() -> i64 {
    let n = 5;
    return func_a(n);
}

fn func_a(n: i64) -> i64 {
    if n == 0 {
        return 1;
    }
    return func_b(n - 1) + func_c(n - 1);
}

fn func_b(n: i64) -> i64 {
    if n == 0 {
        return 2;
    }
    if n == 1 {
        return func_a(0);
    }
    return func_c(n - 1) + 1;
}

fn func_c(n: i64) -> i64 {
    if n == 0 {
        return 3;
    }
    return func_a(n - 1) + func_b(n - 2);
}
``` -->
