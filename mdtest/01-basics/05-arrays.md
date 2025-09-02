# Arrays in Cairo-M

Arrays in Cairo-M exist through the `FixedArray` type. Indexing is only
supported for the `felt` type.

## Basic Array Creation

```cairo-m
fn create_array() -> felt {
    let arr: [felt; 3] = [1, 2, 3];
    return arr[0];
}
```

```cairo-m
fn create_array_u32() -> u32 {
    let arr: [u32; 3] = [1, 2, 3];
    return arr[0];
}
```

## Array Repetition Syntax

You can create a fixed-size array by repeating a single element using the
`[elem; N]` syntax (similar to Rust):

```cairo-m
fn test_main() -> u32 {
    let (arr1, arr2) = repeat_array_u32_one();
    return arr1[1] + arr2[2];
}

fn repeat_array_u32_one() -> ([u32;3], [u32; 4]) {
    return ([0u32; 3], [1u32; 4]);
}
```

Note: when the repeated element is zero, the compiler avoids writing each
element explicitly as the stack is zero-initialized; it only reserves the space
and stores the pointer.

## Array Element Assignment

```cairo-m
fn assign_array_element() -> felt {
    let arr: [felt; 3] = [1, 2, 3];
    arr[0] = 42;           // Set first element
    arr[1] = 24;           // Set second element
    return arr[0];         // Return first element
}
```

## Array Access with Variable Index

```cairo-m
fn variable_index_access(index: felt) -> felt {
    let arr: [felt; 3] = [1, 2, 3];
    arr[0] = 10;
    arr[1] = 20;
    arr[2] = 30;
    if index == 0 || index == 1 || index == 2 {
        return arr[index];
    } else {
        return 0;
    }
}
```

## Array Iteration Pattern

```cairo-m
fn array_sum_loop() -> u32{
    let arr: [u32; 5] = [1, 2, 3, 4, 5];
    let sum: u32 = 0;
    let i = 0;
    while i != 5 {
        sum = sum + arr[i];
        i = i + 1;
    }
    return sum;
}
```

## Array as Function Parameter

When passed as function parameter, only a pointer to the array is passed. Which
means that for an array of size `n`, the receiving function only receives a
pointer of size `1` to the first element of the array.

```cairo-m
fn test_array(arr: [u32; 3]) -> u32 {
    return arr[0];
}
```

```cairo-m
fn use_array_parameter() -> u32 {
    let my_array: [u32; 3] = [1, 2, 3];
    return process_array(my_array, 3felt);
}

fn process_array(arr: [u32; 3], size: felt) -> u32 {
    let sum: u32 = 0;
    let i = 0;
    loop {
        if i == size {
            break;
        }
        sum = sum + arr[i];
        arr[i] = 0;
        i = i + 1;
    }
    return sum;
}
```

```rust
fn use_array_parameter() -> u32 {
    let my_array: [u32; 3] = [1, 2, 3];
    return process_array(my_array, 3u32);
}

fn process_array(mut arr: [u32; 3], size: u32) -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0u32;
    loop {
        if i == size {
            break;
        }
        sum = sum + arr[i as usize];
        arr[i as usize] = 0;
        i = i + 1;
    }
    return sum;
}
```

## Array Bounds and Memory Safety

Arrays in Cairo-M don't have built-in bounds checking:

```cairo-m
//! error: compilation
fn array_bounds_example() -> felt {
    let arr: [felt; 3] = [1, 2, 3];
    // This may access uninitialized memory
    // Behavior depends on what's at that memory location
    return arr[10];        // Out of bounds access
}
```

## Array in Structs and Tuples

Fixed size arrays can be members of structs and tuples.

```cairo-m
struct Result {
    values: [felt; 2],
    sum: felt,
}

fn compute_result(a: felt, b: felt) -> felt {
    let tuple_ : (felt, [felt; 2]) = (a, [a, b]);
    let struct_ : Result = Result {
        values: [a, b],
        sum: a + b,
    };
    return tuple_.1[0] + struct_.values[1];
}
```

```rust
struct Result {
    values: [i32; 2],
    sum: i32,
}

fn compute_result(a: i32, b: i32) -> i32 {
    let tuple_ : (i32, [i32; 2]) = (a, [a, b]);
    let struct_ : Result = Result {
        values: [a, b],
        sum: a + b,
    };
    return tuple_.1[0] + struct_.values[1];
}
```

## Constant Arrays

Arrays can be declared as constants. In that case, an access to the array with a
compile-time known constant will simply replace the access with the constant
value.

```cairo-m
const POW2: [u32; 3] = [1, 2, 4];

fn const_arr_access_const_value() -> u32 {
    return POW2[0] + POW2[1] + POW2[2]; // Replaced with 1 + 2 + 4 = 7
}
```

When the index is not known at compile-time, the array will be fully
materialized every time it's called.

<!-- TODO: optimize this by pre-materializing the array -->

```cairo-m
const POW2: [u32; 3] = [1, 2, 4];

fn test_main() -> u32 {
    return const_arr_access_const_value(0);
}

fn const_arr_access_const_value(i: felt) -> u32 {
    return POW2[i];
}
```
