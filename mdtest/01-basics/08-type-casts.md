# Type Casts

Cairo-M supports type casts between compatible types. Currently, only casts from
`u32` to `felt` are supported.

## Casting From `felt` to `u32`

> Note: casting from `felt` to `u32` is generating a `i64` in the MIR.

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
    return y;
}
```

## Casting a `u32` that doesn't fit in a `felt`

Casting a `u32` in a `felt` checks that the `u32` value is _strictly_ less than
`P == 2^31 - 1`.

> KO

```cairo-m
//! error: VmError(InstructionExecution(Instruction(AssertionFailed(M31(0), M31(1)))))
fn test_u32_to_felt_overflow() -> felt {
    let x: u32 = 2147483647;
    let y: felt = x as felt;
    return y;
}
```

> OK

```cairo-m
//! expected: 2147483646
fn test_u32_to_felt_limit() -> felt {
    let x: u32 = 2147483646;
    let y: felt = x as felt;
    return y;
}
```
