# If-Else Statements in Cairo-M

Cairo-M supports conditional execution with if-else statements.

## Simple If

Basic if statement that returns a value:

```cairo-m
fn test_simple_if(x: felt) -> felt {
    if x == 15 {
        return 1;
    }
    return 0;
}
```

Using a boolean literal in a statement simplifies the MIR CFG:

```cairo-m
fn test_simple_if(x: felt) -> felt {
    if true {
        return 2;
    }
    return 0;
}
```

## If-Else

If-else for choosing between two values:

```cairo-m
fn choose(selector: felt) -> felt {
    if selector == 0 {
        return 10;
    } else {
        return 20;
    }
}
```

```rust
fn choose(selector: i64) -> i64 {
    if selector == 0 {
        return 10;
    } else {
        return 20;
    }
}
```

## Nested If

If statements can be nested:

```cairo-m
//! expected: 2
fn classify(x: felt) -> felt {
    if x == 0 {
        return 0;
    } else {
        if x == 5 {
            return 1;
        } else {
            return 2;
        }
    }
}
```
