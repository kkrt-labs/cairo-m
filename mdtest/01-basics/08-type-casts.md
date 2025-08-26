# Type Casts

Cairo-M supports type casts between compatible types. Currently, only casts from
U32 to Felt are supported.

## Casting between Felt and U32

```cairo-m
fn test_u32_to_felt(input: u32) -> felt {
    let y: felt = input as felt;
    return y;
}
```

```rust
fn test_u32_to_felt(input: u32) -> i64 {
    let y: i64 = input as i64;
    return y;
}
```

```cairo-m
//! error: compilation
fn test_felt_to_u32() -> u32 {
    let x: felt = 10;
    let y: u32 = x as u32;
}
```
