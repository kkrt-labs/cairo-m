# Cairomlings: Exercises Overview

This tutorial mirrors the spirit of Rustlings with bite‑sized exercises, but adapted to Cairo‑M syntax and semantics.

## Rustlings Reference Topics (fetched)

The upstream Rustlings topics are organized as:

- 00_intro
- 01_variables
- 02_functions
- 03_if
- 04_primitive_types
- 05_vecs
- 06_move_semantics
- 05_structs
- 08_enums
- 09_strings
- 10_modules
- 11_hashmaps
- 12_options
- 13_error_handling
- 14_generics
- 15_traits
- 16_lifetimes
- 17_tests
- 18_iterators
- 19_smart_pointers
- 20_threads
- 21_macros
- 22_clippy
- 23_conversions
- quizzes

Not all of these map to Cairo‑M today. Below is the Cairo‑M adapted track based on the language features documented in `mdtest/`.

## Cairo‑M Adapted Tracks

### 00 Intro
- Hello Cairo‑M: empty `main()` that returns.
- Fix a simple return value in `main() -> felt`.

### 01 Variables
- Declarations with `let` and initialization.
- Shadowing: re‑declare with `let x = x + 1`.
- Mutation: reassignment without `mut` (Cairo‑M allows mutation by default).
- Constants with `const` (e.g., arrays of `u32`).

### 02 Functions
- Unit functions: `fn foo() { return; }` and `return();`.
- Functions with parameters and return types.
- Calling helper functions from other functions.

### 03 If/Else
- Basic conditionals that return values.
- If/else branches with equality checks.
- Nested conditionals.

### 04 Primitive Types
- `felt` basics: arithmetic, when to use it.
- `u32` basics: 32‑bit unsigned arithmetic.
- Type inference and literal suffixes (e.g., `200u32`).

### 05 Arrays (Cairo‑M)
- Fixed‑size arrays (e.g., `[u32; 3]`).
- Indexing and summing elements.
- Using arrays in constants.

### 06 Expressions
- Expression vs statement style; returning last expression via `return`.
- Combining expressions inside functions.

### 07 Bitwise (on u32)
- Bitwise AND/OR/XOR/SHL/SHR on `u32` values.
- Emphasize that bitwise ops are on `u32` (not `felt`).

### 08 Type Casts and Conversions
- Explicit casts between numeric types where supported.
- Literals with type suffixes to avoid casts when possible.

### 09 Loops
- While loops with conditions.
- Infinite `loop {}` with `break`.
- C‑style `for (let i = start; i != end; i = i + 1)` loops.
- `continue` and `break` control flow.

### 10 Tuples
- Constructing and returning tuples.
- Accessing tuple elements and using them in expressions.

### 11 Structs
- Defining structs and initializing with `{ field: value }`.
- Field access, mutation, and nested structs.
- Passing structs as parameters and returning them.
- Copy/assignment semantics for multi‑slot aggregates (e.g., `u32` pairs).

### 12 Pointers and Heap (if enabled)
- Basics of pointers/heap as exposed by Cairo‑M tests.
- Moving data to/from heap where applicable.

### 13 Arithmetic Semantics & Panics
- Wrapping `u32` arithmetic: add/sub/mul wrap like Rust’s `Wrapping`.
- Division and remainder: panic on division by zero.
- Guidance on when to use `felt` vs `u32`.

### 14 Recursion & Multi‑Function Programs
- Simple recursion and mutual recursion examples.
- Multi‑function module organization inside a single file.

### 15 Internals (Advanced/Optional)
- Low‑level/internals as available (e.g., opcodes/instructions).
- Performance considerations of `felt` arithmetic.

## Out of Scope (for now)

The following Rustlings tracks do not currently map to Cairo‑M or are not part of these exercises:

- Enums, Strings, HashMaps, Traits, Lifetimes, Threads, Macros, Clippy, Iterators, Tests harness, Move semantics (Rust ownership model)

## Pointers for Learners

- Syntax and examples are aligned with the Cairo‑M docs and tests in `mdtest/`:
  - Basics: literals, variables, functions, primitive types, arithmetic, arrays, expressions
  - Control flow: if‑else, loops (`while`, `loop`, `for`)
  - Types: tuples, structs, pointers/heap
  - Advanced: recursion, multi‑function programs
- Many exercises revolve around “fix the code to produce the expected result/panic behavior” just like Rustlings.
- Prefer `u32` for comparisons, bitwise ops, and standard integer math; use `felt` when field arithmetic is desired.

As you progress, expect to see short `.cm` files per topic with `// TODO` comments to guide the fix.
