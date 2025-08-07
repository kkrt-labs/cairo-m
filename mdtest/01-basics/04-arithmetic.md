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

Division uses multiplicative inverse:

```cairo-m
fn field_division() -> felt {
    let a = 100;
    let b = 10;
    return a / b;  // Returns 10 (uses field inverse)
}
```

## Field Wraparound

Numbers wrap around at field prime:

```cairo-m
//! expected: 0
fn test_wraparound() -> felt {
    let max = 2147483646;  // 2^31 - 2
    return max + 1;  // Wraps to 2^31 - 1
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
//! expected: 5
fn test_overflow() -> felt {
    let large = 2147483640;  // Close to 2^31 - 1
    return large + 12;        // Wraps around
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
