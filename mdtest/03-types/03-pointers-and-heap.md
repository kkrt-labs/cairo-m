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
struct Point {
    x: u32,
    y: u32,
}

fn alloc_points() -> (Point*, Point*) {
    let p1: Point* = new Point[2];
    p1[0] = Point { x: 1, y: 2 };
    p1[1] = Point { x: 3, y: 4 };
    let p2: Point* = new Point[1];
    p2[0] = Point { x: 5, y: 6 };
    return (p1, p2);
}

fn test_main() -> u32 {
    let (p1, p2) = alloc_points();
    return p1[0].x + p1[1].y + p2[0].x + p2[0].y;
}
```

```rust
struct Point { x: i32, y: i32 }

fn alloc_points() -> Vec<Point> {
    let mut ps: Vec<Point> = Vec::with_capacity(3);
    ps.push(Point { x: 1, y: 2 });
    ps.push(Point { x: 3, y: 4 });
    ps.push(Point { x: 5, y: 6 });
    return ps;
}

fn test_main() -> i32 {
    let ps = alloc_points();
    return ps[0].x + ps[1].y + ps[2].x + ps[2].y;
}
```
