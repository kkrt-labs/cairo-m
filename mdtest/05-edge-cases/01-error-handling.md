# Error Handling and Edge Cases in Cairo-M

Cairo-M's M31 field arithmetic and runtime behavior create unique edge cases
that developers need to understand. This document covers boundary conditions,
overflow behavior, and error scenarios.

## Division by Zero

Field division by zero behavior in M31:

```cairo-m
//! ignore: 0 has no inverse
fn division_by_zero() -> felt {
    let a = 42;
    let b = 0;
    return a / b;  // Field inverse of 0 is undefined - runtime panic
}
```

## Field Boundary Values

Testing M31 field boundaries (2^31 - 1):

```cairo-m
//! expected: 0
fn field_max_value() -> felt {
    let max_val = 2147483646;  // 2^31 - 2
    let max_plus_one = max_val + 1;  // Should equal 2^31 - 1 which is (0 modulo 2^31 - 1)
    return max_plus_one;  // Returns 0
}
```

## Negative Number Wraparound

Field arithmetic with negative numbers:

```cairo-m
fn negative_wraparound() -> felt {
    let zero = 0;
    let minus_one = zero - 1;  // Should wrap to 2^31 - 2
    let large_negative = -1000000;
    return minus_one + large_negative;
}
```

```rust
fn negative_wraparound() -> u32 {
    const M31_PRIME: u64 = (1u64 << 31) - 1;
    let zero = 0u64;
    let minus_one = (zero + M31_PRIME - 1) % M31_PRIME;  // 2^31 - 2
    let large_negative = (M31_PRIME - 1000000) % M31_PRIME;
    ((minus_one + large_negative) % M31_PRIME) as u32
}
```

## U32 Overflow Behavior

Testing u32 type overflow in operations:

```cairo-m
fn u32_overflow() -> u32 {
    let max_u32: u32 = 4294967295;  // 2^32 - 1
    let overflow_add = max_u32 + 1;  // Wraps to 0
    let underflow_sub: u32 = 0;
    let underflow = underflow_sub - 1;  // Wraps to max u32
    return overflow_add + underflow;  // 0 + 4294967295
}
```

```rust
fn u32_overflow() -> u32 {
    let max_u32: u32 = u32::MAX;
    let overflow_add = max_u32.wrapping_add(1);  // 0
    let underflow_sub: u32 = 0;
    let underflow = underflow_sub.wrapping_sub(1);  // u32::MAX
    overflow_add.wrapping_add(underflow)
}
```

## Zero Conditions and Truthiness

Testing zero/non-zero conditions:

```cairo-m
fn zero_truthiness(x: felt) -> felt {
    // Test various zero conditions
    if (x == 0) {
        return 1;
    }
    if (x != 0) {
        return 3;
    }
    return 4;  // Should never reach here
}
```

## Unreachable Code After Return

Dead code elimination:

```cairo-m
fn unreachable_code() -> felt {
    let x = 42;
    return x;

    // This code should be eliminated
    let dead_var = 99;
    let another_dead = dead_var * 2;
    return another_dead;  // Unreachable
}
```

## Infinite Loop with Break Condition

Loop that appears infinite but has exit condition:

```cairo-m
fn loop_with_escape() -> u32 {
    let counter = 0u32;

    loop {
        counter = counter + 1u32;

        // Multiple exit conditions
        if (counter > 1000u32) {
            break;  // Prevent actual infinite loop
        }

        if (counter == 10u32) {
            break;
        }
    }

    return counter;  // Should return 100
}
```

## Complex Expression Edge Cases

Operator precedence with edge values:

```cairo-m
fn complex_edge_expression() -> felt {
    let a = 2147483646;  // Near field max
    let b = 0;
    let c = 1;

    // Complex expression that might cause issues
    let result = (a + c) / (c + b) * (a - b) + b;
    return result;
}
```

```rust
fn complex_edge_expression() -> u32 {
    const M31_PRIME: u64 = (1u64 << 31) - 1;
    let a = (M31_PRIME - 1) as u32;  // Near field max
    let b = 0u32;
    let c = 1u32;

    // Simulate field arithmetic
    let a_plus_c = ((a as u64 + c as u64) % M31_PRIME) as u32;
    let c_plus_b = c + b;
    let div_result = if c_plus_b != 0 { a_plus_c / c_plus_b } else { 0 };
    let a_minus_b = ((a as u64 - b as u64 + M31_PRIME) % M31_PRIME) as u32;
    let mul_result = ((div_result as u64 * a_minus_b as u64) % M31_PRIME) as u32;
    ((mul_result as u64 + b as u64) % M31_PRIME) as u32
}
```

## Memory and Stack Edge Cases

Testing deep recursion limits:

```cairo-m
fn deep_recursion(n: u32, depth: u32) -> u32 {
    if (depth > 1000u32) {  // Prevent stack overflow
        return n;
    }

    if (n <= 1u32) {
        return 1;
    }

    return n + deep_recursion(n - 1, depth + 1);
}
```
