# Bitwise Operations

Cairo-M supports bitwise AND, OR, and XOR operations on the `u32` type, both on
register-based values and immediate operands.

## Bitwise And

```cairo-m
fn test_bitwise_and(x: u32, y: u32) -> u32 {
    let result = x & y;
    return result;
}
```

```cairo-m
fn test_bitwise_and_immediates() -> u32 {
    let result = 69u32 & 420u32;
    return result;
}
```

## Bitwise Or

```cairo-m
fn test_bitwise_or(x: u32, y: u32) -> u32 {
    let result = x | y;
    return result;
}
```

```cairo-m
fn test_bitwise_or_immediates() -> u32 {
    let result = 69u32 | 420u32;
    return result;
}
```

## Bitwise Xor

```cairo-m
fn test_bitwise_xor(x: u32, y: u32) -> u32 {
    let result = x ^ y;
    return result;
}
```

```cairo-m
fn test_bitwise_xor_immediates() -> u32 {
    let result = 69u32 ^ 420u32;
    return result;
}
```
