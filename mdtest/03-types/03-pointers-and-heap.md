# Pointers and Heap Allocation (new)

This page demonstrates the `new` keyword for heap allocation and pointer-based
indexing semantics.

## Allocate felt and read back

```cairo-m
fn alloc_felt() -> felt {
    let p: felt* = new felt[3];
    // Initialize via pointer indexing
    p[0] = 7;
    p[1] = 8;
    p[2] = 9;
    return p[0] + p[1] + p[2];
}
```

```rust
fn alloc_felt() -> i32 {
    let mut p: Vec<i32> = Vec::with_capacity(3);
    p.push(7);
    p.push(8);
    p.push(9);
    return p[0] + p[1] + p[2];
}
```

## Allocate u32 with dynamic size

```cairo-m
fn alloc_u32() -> u32 {
    let p: u32* = new u32[3];
    // Single element write/read
    p[0] = 42u32;
    return p[0];
}
```

```rust
fn alloc_u32() -> i32 {
    let mut p: Vec<i32> = Vec::with_capacity(3);
    p.push(42);
    return p[0];
}
```

## Allocate struct pointer and access fields

```cairo-m
struct Point { x: felt, y: felt }

fn alloc_struct() -> felt {
    let ps: Point* = new Point[2];
    // Write fields via pointer indexing then field
    (ps[0]).x = 3;
    (ps[0]).y = 4;
    (ps[1]).x = 5;
    (ps[1]).y = 6;
    return ps[0].x + ps[1].y;
}
```

```rust
struct Point { x: i32, y: i32 }

fn alloc_struct() -> i32 {
    let mut ps: Vec<Point> = Vec::with_capacity(2);
    ps.push(Point { x: 3, y: 4 });
    ps.push(Point { x: 5, y: 6 });
    return ps[0].x + ps[1].y;
}
```
