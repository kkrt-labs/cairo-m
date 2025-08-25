# Loops in Cairo-M

Cairo-M supports various loop constructs for iteration and repetition.

## For Loop

Basic for loop with range:

```cairo-m
fn sum_to_n() -> felt {
    let sum = 0;
    for (let i = 0; i != 10; i = i + 1) {
        sum = sum + i;
    }
    return sum;
}
```

```rust
fn sum_to_n() -> i64 {
    let mut sum = 0;
    for i in 0..10 {
        sum = sum + i;
    }
    return sum;
}
```

## While Loop

While loop with condition:

```cairo-m
fn count_down() -> felt {
    let x = 10;
    let count = 0;
    while (x != 0) {
        x = x - 1;
        count = count + 1;
    }
    return count;
}
```

## Loop with Break

Infinite loop with break condition:

```cairo-m
fn loop_with_break() -> felt {
    let i = 10;
    loop {
        if i == 7 || i == 0 {
            break;
        }
        i = i - 1;
    }
    return i;
}
```

## Nested Loops

Loops can be nested:

```cairo-m
fn multiply_table() -> felt {
    let result = 0;
    for (let i = 1; i != 10; i = i + 1) {
        for (let j = 1; j != 10; j = j + 1) {
            result = result + (i * j);
        }
    }
    return result;
}
```

```rust
fn multiply_table() -> i64 {
    let mut result = 0;
    for i in 1..10 {
        for j in 1..10 {
            result = result + (i * j);
        }
    }
    return result;
}
```

## Continue in Loops

Skip iterations with continue:

```cairo-m
fn continue_in_loop() -> felt {
    let sum = 0;
    for (let i = 0; i != 10; i = i + 1) {
        if i == 5 {
            continue;
        }
        sum = sum + i;
    }
    return sum;
}
```

```rust
fn continue_in_loop() -> i64 {
    let mut sum = 0;
    for i in 0..10 {
        if i == 5 {
            continue;
        }
        sum = sum + i;
    }
    return sum;
}
```
