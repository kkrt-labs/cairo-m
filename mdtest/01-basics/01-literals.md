# Literals in Cairo-M

Cairo-M supports `felt` literals that represent field elements in the M31 field
(2^31 - 1).

## Integer Literals

Basic integer literals are written as decimal numbers:

```cairo-m
fn test_integer() -> felt {
    return 42;
}
```

## Negative Numbers

Negative numbers are supported and follow field arithmetic rules:

```cairo-m
fn test_negative() -> felt {
    let x = -5;
    return x + 10;
}
```
