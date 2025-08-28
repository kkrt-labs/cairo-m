# Primitive Types in Cairo-M

Cairo-M has two primitive types: `felt` and `u32`.

## Felt Type

The `felt` type represents field elements in the M31 field:

```cairo-m
fn base_type() -> felt {
    let x: felt = 100;
    let y: felt = 200;
    return x + y;
}
```

The `felt` type cannot be used for comparison and bitwise operations. Most of
the time, you will want to use the `u32` type instead - unless you are looking
for specific performances using native field arithmetics.

## U32 Type

The `u32` type represents 32-bit unsigned integers:

```cairo-m
fn test_u32() -> u32 {
    let x: u32 = 100;
    let y: u32 = 200;
    return x + y;
}
```

### U32 Addition

Arithmetic operation on `u32` are wrapped in the `u32` type.

```cairo-m
fn test_u32_add() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x + y;
}
```

```rust
use std::num::Wrapping;
fn test_u32_add() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x.wrapping_add(y);
}
```

### U32 Subtraction

```cairo-m
fn test_u32_sub() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x - y;
}
```

```rust
use std::num::Wrapping;
fn test_u32_sub() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x.wrapping_sub(y);
}
```

```cairo-m
fn test_u32_sub_underflow() -> u32 {
    let x: u32 = 0;
    let y: u32 = 1;
    return x - y;
}
```

```rust
fn test_u32_sub_underflow() -> u32 {
    let x: u32 = 0;
    let y: u32 = 1;
    return x.wrapping_sub(y);
}
```

### U32 Multiplication

```cairo-m
fn test_u32_mul() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x * y;
}
```

```rust
use std::num::Wrapping;
fn test_u32_mul() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x.wrapping_mul(y);
}
```

### U32 Division

```cairo-m
fn test_u32_div() -> u32 {
    let x: u32 = 4294967295;
    let y: u32 = 10;
    return x / y;
}
```

Division by zero panics.

## Type Inference

Types can often be inferred. By default, a literal is inferred as `felt`. We can
also add a type suffix to a literal to force it to a specific type.

```cairo-m
fn test_inference() -> u32 {
    let x = 42;  // Type inferred as felt
    let y: u32 = 100;
    let z = 200u32;
    return y + z;
}
```

```rust
fn test_inference() -> u32 {
    let x = 42;  // Type inferred as i64
    let y: u32 = 100;
    let z = 200u32;
    return y + z;
}
```
