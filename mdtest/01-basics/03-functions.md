# Functions in Cairo-M

Functions are the building blocks of Cairo-M programs. They can take parameters
and return values.

## Simple Function

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

## Function Calls

Functions can call other functions:

```cairo-m
fn double(x: felt) -> felt {
    return x * 2;
}

fn quadruple(x: felt) -> felt {
    return double(double(x));
}

fn test_calls() -> felt {
    return quadruple(5);
}
```
