<!-- # Arrays in Cairo-M

Arrays in Cairo-M are represented using pointer types (`felt*`) and support index-based access for both reading and writing values.

## Basic Array Creation

Arrays are created using pointer types with a base address:

```cairo-m
fn create_array() -> felt {
    let arr: felt* = 100;  // Base address for array
    return arr[0];         // Access first element
}
```

```rust
fn create_array() -> u32 {
    let arr = [42, 10, 20, 30]; // Rust array
    arr[0]  // Returns 42, but Cairo-M will return whatever is at memory location 100
}
```

## Array Element Assignment

Setting values at specific array indices:

```cairo-m
fn assign_array_element() -> felt {
    let arr: felt* = 100;  // Base address
    arr[0] = 42;           // Set first element
    arr[1] = 24;           // Set second element
    return arr[0];         // Return first element
}
```

```rust
fn assign_array_element() -> u32 {
    let mut arr = [0; 10];  // Array with 10 elements
    arr[0] = 42;
    arr[1] = 24;
    arr[0]  // Returns 42
}
```

## Array Access with Variable Index

Using variables as array indices:

```cairo-m
fn variable_index_access(index: felt) -> felt {
    let arr: felt* = 200;  // Base address
    arr[0] = 10;
    arr[1] = 20;
    arr[2] = 30;
    return arr[index];     // Access element at variable index
}
```

```rust
fn variable_index_access(index: usize) -> u32 {
    let arr = [10, 20, 30, 40, 50];
    if index < arr.len() {
        arr[index]
    } else {
        0  // Cairo-M behavior may differ based on memory contents
    }
}
```

## Array Element Updates

Modifying array elements and reading them back:

```cairo-m
fn update_array_elements() -> felt {
    let arr: felt* = 300;  // Base address
    arr[0] = 5;
    arr[1] = 10;
    arr[2] = 15;

    // Update elements
    arr[0] = arr[0] + 1;   // Increment first element
    arr[1] = arr[1] * 2;   // Double second element
    arr[2] = arr[0] + arr[1]; // Set third to sum of first two

    return arr[2];         // Return modified third element
}
```

```rust
fn update_array_elements() -> u32 {
    let mut arr = [5, 10, 15, 0, 0];

    arr[0] = arr[0] + 1;   // arr[0] = 6
    arr[1] = arr[1] * 2;   // arr[1] = 20
    arr[2] = arr[0] + arr[1]; // arr[2] = 26

    arr[2]  // Returns 26
}
```

## Array Iteration Pattern

Using arrays in loop structures:

```cairo-m
fn array_sum_loop() -> felt {
    let arr: felt* = 400;  // Base address
    arr[0] = 1;
    arr[1] = 2;
    arr[2] = 3;
    arr[3] = 4;
    arr[4] = 5;

    let sum = 0;
    let i = 0;
    loop {
        if (i == 5) {
            break;
        }
        sum = sum + arr[i];
        i = i + 1;
    }
    return sum;
}
```

```rust
fn array_sum_loop() -> u32 {
    let arr = [1, 2, 3, 4, 5];
    let mut sum = 0;
    let mut i = 0;

    while i < arr.len() {
        sum += arr[i];
        i += 1;
    }
    sum  // Returns 15
}
```

## Array as Function Parameter

Passing arrays to functions:

```cairo-m
fn process_array(arr: felt*, size: felt) -> felt {
    let sum = 0;
    let i = 0;
    loop {
        if (i == size) {
            break;
        }
        sum = sum + arr[i];
        i = i + 1;
    }
    return sum;
}

fn use_array_parameter() -> felt {
    let my_array: felt* = 500;  // Base address
    my_array[0] = 10;
    my_array[1] = 20;
    my_array[2] = 30;
    return process_array(my_array, 3);
}
```

```rust
fn process_array(arr: &[u32]) -> u32 {
    arr.iter().sum()
}

fn use_array_parameter() -> u32 {
    let my_array = [10, 20, 30];
    process_array(&my_array)  // Returns 60
}
```

## Nested Array Access

Working with multi-dimensional array-like structures:

```cairo-m
fn nested_array_access() -> felt {
    let matrix: felt* = 600;  // Base address for 2D array

    // Set up a 3x3 matrix (flattened)
    // Row 0: [1, 2, 3]
    matrix[0] = 1; matrix[1] = 2; matrix[2] = 3;
    // Row 1: [4, 5, 6]
    matrix[3] = 4; matrix[4] = 5; matrix[5] = 6;
    // Row 2: [7, 8, 9]
    matrix[6] = 7; matrix[7] = 8; matrix[8] = 9;

    // Access element at row 1, column 2 (0-indexed)
    let row = 1;
    let col = 2;
    let width = 3;
    let index = row * width + col;  // Calculate flat index
    return matrix[index];           // Should return 6
}
```

```rust
fn nested_array_access() -> u32 {
    let matrix = [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9]
    ];

    let row = 1;
    let col = 2;
    matrix[row][col]  // Returns 6
}
```

## Array Bounds and Memory Safety

Arrays in Cairo-M don't have built-in bounds checking:

```cairo-m
//! expected: varies based on memory contents
fn array_bounds_example() -> felt {
    let arr: felt* = 700;  // Base address
    arr[0] = 100;
    arr[1] = 200;

    // This may access uninitialized memory
    // Behavior depends on what's at that memory location
    return arr[10];        // Out of bounds access
}
```

```rust
fn array_bounds_example() -> u32 {
    let arr = [100, 200, 0, 0, 0];  // Fixed size array
    // arr[10] would panic in safe Rust
    // Returning a safe default instead
    0  // Safe fallback value
}
``` -->
