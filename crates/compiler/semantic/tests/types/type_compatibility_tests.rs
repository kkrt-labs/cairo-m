//! # Type Compatibility Tests
//!
//! This module contains tests for type compatibility logic and error handling.
use crate::*;

#[test]
fn test_type_compatibility() {
    assert_semantic_parameterized! {
        ok: [
            // felt is compatible with felt
            in_function("let a: felt = 42; let b: felt = a;"),
            // Different structs are not compatible, but this is valid code
            "struct Point { x: felt, y: felt } struct Vector { x: felt, y: felt } fn test() { let p = Point { x: 1, y: 2 }; let v = Vector { x: 3, y: 4 }; return (); }",
            // Functions with same signature
            "fn add(a: felt, b: felt) -> felt { return a + b; } fn mul(a: felt, b: felt) -> felt { return a * b; }",
            // Nested structs
            "struct C {v: felt} struct W {i: C} fn test() { let c = C{v:42}; let w = W{i:c}; return (); }",
            // Complex valid program
            "struct Point { x: felt, y: felt } fn dist(p1: Point, p2: Point) -> felt { return 0; } fn test() { let p1 = Point {x:0, y:0}; let p2 = Point {x:3,y:4}; let d = dist(p1,p2); return (); }",
        ],
        err: [
            // TODO: fix these cases
            // // Error type propagation, should still produce diagnostics
            in_function("let x: BadType = 1; let y = x; let z = y + 1;"),
            // // Invalid type in expression
            "fn test() -> felt { let x: UnknownType = 42; let y: felt = 10; return x + y; }",

            // Direct self-referential struct should be an error
            // TODO: fix this case
            // "struct SelfRef { value: felt, self_field: SelfRef }",
        ]
    }
}
