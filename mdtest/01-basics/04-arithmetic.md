# Field Arithmetic in Cairo-M

Cairo-M operates in the M31 field (2^31 - 1), providing unique arithmetic
properties.

## Basic Arithmetic Operations

Standard arithmetic operations:

```cairo-m
fn basic_arithmetic() -> felt {
    let a = 10;
    let b = 20;
    let sum = a + b;
    let diff = b - a;
    let prod = a * b;
    return sum + diff + prod;  // Returns 30 + 10 + 200 = 240
}
```

## Division in Field

Division of field elements (and therefore division in Cairo-M) is not the
integer division you have in many programming languages, where the integral part
of the quotient is returned (so you get 7 / 3 = 2). As long as the numerator is
a multiple of the denominator, it will behave as you expect `(6 / 3 = 2)`. If
this is not the case, for example when we divide 7/3, it will result in a field
element x that will satisfy `3 * x = 7`.

```cairo-m
fn field_division() -> felt {
    let a = 6;
    let b = 3;
    return a / b;  // Returns 2
}
```

If the numerator is not divisible by the denominator, the result of the division
in field arithmetics can be computed as the inverse of the denominator
multiplied by the numerator.

```cairo-m
fn field_division_non_divisible() -> felt {
    let a = 7;
    let b = 3;
    return a / b;  // Returns 1431655767 (uses field inverse)
}
```

```rust
use stwo_prover::core::fields::m31::M31;

fn field_division_non_divisible() -> M31 {
    let a = M31::from(7);
    let b = M31::from(3);
    a / b
}
```

## Field Wraparound

Numbers wrap around at field prime:

```cairo-m
fn test_wrap_around_upper() -> felt {
    let max = 2147483646;  // 2^31 - 2
    return max + 1;  // Wraps to 2^31 - 1 == 0
}
```

```rust
use stwo_prover::core::fields::m31::M31;

fn test_wrap_around_upper() -> M31 {
    let max = M31::from(2147483646);  // 2^31 - 2
    max + M31::from(1)  // Should wrap to M31::from(2^31 - 1) == 0
}
```

```cairo-m
fn test_wraparound_lower() -> felt {
    let min = 0;
    return min - 1;  // Wraps to 2^31 - 2
}
```

```cairo-m
fn test_wraparound_lower() -> felt {
    let min = 0;
    return min - 1;  // Wraps to 2^31 - 2
}
```

## Negative Numbers

Negative numbers in field arithmetic:

```cairo-m
fn negative_arithmetic() -> felt {
    let a = 10;
    let b = -5;
    let c = a + b;  // 10 + (-5) = 5
    let d = b * 2;  // -5 * 2 = -10
    return c - d;   // 5 - (-10) = 15
}
```

## Powers and Exponentiation

Computing powers:

```cairo-m
fn powers() -> felt {
    let base = 3;
    let p2 = base * base;         // 3^2 = 9
    let p3 = p2 * base;           // 3^3 = 27
    let p4 = p3 * base;           // 3^4 = 81
    return p2 + p3 + p4;          // 9 + 27 + 81 = 117
}
```

## Complex Expressions

Combined arithmetic operations:

```cairo-m
fn complex_expression() -> felt {
    let x = 5;
    let y = 3;
    let z = 7;
    return (x * y + z) * (x - y) / 2;  // (15 + 7) * 2 / 2 = 22
}
```

## Overflow Behavior

Testing field overflow:

```cairo-m
fn test_overflow() -> felt {
    let large = 2147483640;  // Close to 2^31 - 1
    return large + 12;        // Wraps around
}
```

```rust
use stwo_prover::core::fields::m31::M31;

fn test_overflow() -> M31 {
    let large = M31::from(2147483640);  // Close to 2^31 - 1
    large + M31::from(12)  // Should wrap to M31::from(2^31 - 1)
}
```

## Zero and One

Identity elements:

```cairo-m
fn identity_elements() -> felt {
    let x = 42;
    let zero_add = x + 0;      // Additive identity
    let one_mult = x * 1;      // Multiplicative identity
    let zero_mult = x * 0;     // Multiplication by zero
    return zero_add + one_mult - zero_mult;  // 42 + 42 - 0 = 84
}
```

## Associativity and Commutativity

Mathematical properties:

```cairo-m
fn test_properties() -> felt {
    let a = 12;
    let b = 8;
    let c = 5;

    // Associativity: (a + b) + c == a + (b + c)
    let assoc1 = (a + b) + c;
    let assoc2 = a + (b + c);

    // Commutativity: a * b == b * a
    let comm1 = a * b;
    let comm2 = b * a;

    // Should be 0 if properties hold
    return (assoc1 - assoc2) + (comm1 - comm2);  // Returns 0
}
```
