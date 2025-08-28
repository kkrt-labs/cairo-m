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
//! ignore: Compiler bug with shadowed variables.
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

## Constants

Constants are declared using the `const` keyword. They can be assigned an
optional type; otherwise, the type is inferred from the expression.

```cairo-m
const POW2: [u32; 3] = [1, 2, 4];

fn test_const() -> u32 {
    return POW2[0] + POW2[1] + POW2[2];
}
```
