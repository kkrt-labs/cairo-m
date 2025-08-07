# Structs in Cairo-M

Structs allow grouping related data together into custom types.

## Basic Struct Definition

Simple struct with fields:

```cairo-m
struct Point {
    x: felt,
    y: felt,
}

fn test_struct_basic() -> felt {
    let p = Point { x: 10, y: 20 };
    return p.x + p.y;
}
```

```cairo-m
//! ignore: CodeGenerationFailed("Layout error: No layout found for value 3")
struct PointU32 {
    x: u32,
    y: u32,
}

fn test_struct_basic_u32() -> u32 {
    let p = PointU32 { x: 10, y: 20 };
    return p.x + p.y;
}
```

## Struct Field Access

Accessing and modifying struct fields:

```cairo-m
struct Rectangle {
    width: felt,
    height: felt,
}

fn calculate_area() -> felt {
    let rect = Rectangle { width: 5, height: 10 };
    rect.width = 7;
    return rect.width * rect.height;  // Returns 70
}
```

```cairo-m
//! ignore: CodeGenerationFailed("Layout error: No layout found for value 3")
struct RectangleU32 {
    width: u32,
    height: u32,
}

fn calculate_area_u32() -> u32 {
    let rect = RectangleU32 { width: 5, height: 10 };
    rect.width = 7u32;
    return rect.width * rect.height;
}
```

## Nested Structs

Structs containing other structs:

```cairo-m
//! ignore: CodeGenerationFailed("Layout error: No layout found for value 3")
struct Point {
    x: felt,
    y: felt,
}

struct Line {
    start: Point,
    end: Point,
}

fn line_length_squared() -> felt {
    let line = Line {
        start: Point { x: 0, y: 0 },
        end: Point { x: 3, y: 4 }
    };
    let dx = line.end.x - line.start.x;
    let dy = line.end.y - line.start.y;
    return dx * dx + dy * dy;  // Returns 25
}
```

## Struct as Function Parameter

Passing structs to functions:

//TODO: fix

```cairo-m
//! ignore: true
struct Vector {
    x: felt,
    y: felt,
    z: felt,
}

fn dot_product(v1: Vector, v2: Vector) -> felt {
    return v1.x * v2.x + v1.y * v2.y + v1.z * v2.z;
}

fn test_struct_param() -> felt {
    let a = Vector { x: 1, y: 2, z: 3 };
    let b = Vector { x: 4, y: 5, z: 6 };
    return dot_product(a, b);  // Returns 32
}
```
