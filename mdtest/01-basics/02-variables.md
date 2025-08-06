# Variables in Cairo-M

Variables in Cairo-M are declared using the `let` keyword.

## Variable Declaration

Variables must be initialized when declared:

```cairo-m
fn test_variable() -> felt {
    let x = 10;
    let y = 20;
    return x + y;
}
```

## Variable Shadowing

Variables can be shadowed by declaring a new variable with the same name:

```cairo-m
// TODO: fix this test
//! ignore: true
fn test_shadowing() -> felt {
    let x = 5;
    let x = x + 1;
    let x = x * 2;
    return x;
}
```

## Multiple Variables

Multiple variables can be used in expressions:

```cairo-m
fn test_multiple_vars() -> felt {
    let a = 10;
    let b = 20;
    let c = 30;
    let d = 40;
    return a + b + c + d;
}
```

## Variable Mutation

Variables can be mutated by default, unlike in Rust.

```cairo-m
fn test_mutation() -> felt {
    let x = 5;
    x = x + 1;
    return x;
}
```

```rust
fn test_mutation() -> i32 {
    let mut x = 5;
    x = x + 1;
    return x;
}
```
