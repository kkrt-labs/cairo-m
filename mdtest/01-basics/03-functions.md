# Functions in Cairo-M

Functions are the building blocks of Cairo-M programs. They can take parameters
and return values.

## Simple Function

A basic function without parameters:

```cairo-m
fn foo() {
    return();
}
```

If no return value is specified, the function returns `()` - the Unit type. It
can be omitted for simplicity.

```cairo-m
fn foo() {
    return;
}
```

## Function with Return Values

A basic function without parameters:

```cairo-m
fn simple() -> felt {
    return 42;
}
```

## Function with Parameters

Functions can accept parameters:

```cairo-m
fn add(a: felt, b: felt) -> felt {
    return a + b;
}
```

```rust
fn add(a: i64, b: i64) -> i64 {
    return a.wrapping_add(b);
}
```

## Function Calls

Functions can call other functions:

```cairo-m
fn test_calls() -> felt {
    return double(double(5));
}
fn double(x: felt) -> felt {
    return x * 2;
}
```

```rust
fn test_calls() -> i64 {
    return double(double(5));
}
fn double(x: i64) -> i64 {
    return x.wrapping_mul(2);
}
```
