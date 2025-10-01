# Getting started with Cairo-M

This guide covers how to install, configure, and use the Cairo-M Language
Server, the VS Code extension, and the current language features as implemented
today.

## 1. Installation & Configuration

### 1.1. Installing the Language Server & VS Code Extension

Currently, the recommended way to install is by building from the source.

**Prerequisites:**

- VS Code (1.75.0 or higher)
- Rust Toolchain (`rustup`)
- Node.js and `npm`

#### Step 1: Clone the Repository

```bash
git clone https://github.com/kkrt-labs/cairo-m.git
cd cairo-m
```

Follow instructions in [README.md](../README.md) to install the project.

#### Step 2: Build the Language Server

The language server is a binary that provides all the language intelligence.

```bash
# Build in release mode for performance
cargo build --release -p cairo-m-ls

# The binary will be located at:
# - target/release/cairo-m-ls (Linux/macOS)
```

#### Step 3: Build and Install the VS Code Extension

```bash
# Navigate to the extension's directory
cd vscode-cairo-m

# Install dependencies
npm install

# Package the extension into a .vsix file
npm run package

# This creates a file like `cairo-m.vsix`
```

#### Step 4: Install in VS Code

1.  Open VS Code.
2.  Go to the **Extensions** view (Ctrl+Shift+X).
3.  Click the "..." menu at the top-right and select **Install from VSIX...**.
4.  Choose the `.vsix` file you just created.
5.  Restart VS Code when prompted.

### 1.2. Configuring the VS Code Extension

You will need to set the path to the language server manually.

1.  Open VS Code settings (Ctrl+,).
2.  Search for "Cairo-M".
3.  Find the **Cairo-m: Language Server: Path** setting and set it to the
    absolute path of the `cairo-m-ls` binary you built.

Alternatively, add it to your `.vscode/settings.json` file:

```json
{
  // Example for Linux/macOS
  "cairo-m.languageServer.path": "/path/to/your/cairo-m/target/release/cairo-m-ls"
}
```

## 2. Project Setup

### 2.1. Create a Project

You can scaffold a new Cairo‑M project with the helper CLI:

```bash
cargo install --path crates/cargo-cairo-m
cargo-cairo-m init my_project
```

This creates a project with `cairom.toml` and a `src/` folder ready to build and
test.

A minimal project layout looks like:

```text
my_project/
├── cairom.toml       # Project manifest
└── src/
    └── main.cm       # Entry point (default is main.cm)
```

### 2.2. Manifest File (`cairom.toml`)

The manifest configures your project. The `entry_point` is relative to `src/`.

```toml
name = "my_project"
version = "0.1.0"   # default if omitted
entry_point = "main.cm"
```

### 2.3. Module System

The file structure within the `src` directory directly maps to the module
hierarchy.

- `src/main.cm` -> module `main`
- `src/utils.cm` -> module `utils`
- `src/utils/math.cm` -> module `utils::math`

You can import items from other modules using the `use` keyword. Grouped imports
are supported (e.g., `use utils::{math, io};`). The semantic layer validates
imported items exist in target modules, and the language server supports
go‑to‑definition for imports.

```rust
// In src/main.cm
use utils::math::add; // Imports the 'add' function from the 'utils::math' module

fn main() {
    add(1, 2);
    return;
}
```

## 3. Language Features

### 3.1. Comments

```rust
// This is a single-line comment.
```

### 3.2. Variables and Constants

- Variables: Declared with `let`, initialized on declaration, mutable by
  default, shadowing allowed.
- Constants: Declared with `const`, must be compile‑time constant expressions.

```rust
fn variables_and_constants() {
    let x = 10;        // inferred felt
    let y: felt = 20;  // explicit type
    x = 30;            // mutation is allowed

    const POW2: [u32; 3] = [1, 2, 4];
    return;
}

fn shadowing() {
    let x = 5;
    let x = x + 1;  // shadows previous x
    return;
}
```

### 3.3. Data Types

- felt: Field element in M31 (2^31 − 1). Default numeric literal type.
- u32: 32‑bit unsigned integer with wrapping arithmetic.
- bool: Boolean literal type (`true`, `false`).
- Tuples: Fixed‑size heterogenous values, e.g. `(felt, bool)`.
- Structs: User‑defined aggregates with named fields.
- Arrays: Fixed‑size arrays `[T; N]` of `felt` or `u32` elements.

Notes:

- Single‑element tuples require a trailing comma in type and value positions:
  `(felt,)` and `(x,)`. Without a trailing comma, `(T)`/`(expr)` are just
  parenthesized type/expressions.
- Numeric literal suffixes: append `u32` to force a `u32` literal (e.g.,
  `200u32`). Unsuffixed numeric literals default to `felt`, unless the context
  allows inferring the type.

### 3.4. Operators

- Arithmetic (felt): `+`, `-`, `*`, `/`, unary `-`.
  - Division is field division: when not divisible, uses the multiplicative
    inverse.
- Arithmetic (u32): `+`, `-`, `*`, `/`, `%` with 32‑bit wrapping semantics.
- Comparison (felt): `==`, `!=` only.
- Comparison (u32): `==`, `!=`, `<`, `>`, `<=`, `>=`.
- Bitwise (u32): `&`, `|`, `^` on u32 values and immediates.
- Logical (bool): `&&`, `||`, `!`.

Operator precedence and associativity follow conventional math rules;
parentheses control grouping.

### 3.5. Functions

Functions are declared with `fn`. Types for all parameters and the return value
must be specified. A function that returns nothing has a unit type `()`, which
can be omitted from the signature.

All functions require an explicit `return`, even for the unit `()`.

```rust
// Function with parameters and a return value
fn add(a: felt, b: felt) -> felt {
    return a + b;
}

// Function with no parameters
fn get_answer() -> felt {
    return 42;
}

// Function that returns a tuple
fn get_pair() -> (felt, bool) {
    return (10, true);
}

// Function with no return value (implicitly returns unit type `()`)
fn log_message() {
    // ... do something ...
    return ();
}
```

### 3.6. Structs

Structs are custom data types that group related values.

```rust
// Struct definition
struct Point {
    x: felt,
    y: felt,
}

fn use_structs() {
    // Struct instantiation (literal)
    let p1 = Point { x: 10, y: 20 };

    // Accessing fields
    let x_coord = p1.x;

    // Assigning to a struct field
    p1.y = 30;
    return;
}
```

### 3.7. Control Flow

- if / else: The condition must be `bool`.

  ```rust
  fn check_value(x: felt) -> felt {
      let x = 1;
      if (x == 11) {
          let x = 10;
      } else if (x == 10) {
          let x = 2;
      } else {
          x = 3;
      }
      // 3 !
      return x;
  }

  ```

- loop: Infinite loop, exit with `break`.
  ```rust
  fn infinite_loop() {
      loop {
          // This will run forever unless a `break` is encountered.
          if (some_condition()) {
              break; // Exits the loop
          }
          continue; // Skips to the next iteration
      }
      return ();
  }
  ```
- while: Loops while a `bool` condition holds.

  ```rust
  fn while_loop() {
      let x = 0;
      while x != 10 {
          x = x + 1;
      }
      return ();
  }
  ```

- for: C‑style headers: `for (init; condition; step) { ... }`.
  ```cairo
  fn for_loop() {
      for (let i = 0; i != 10; i = i + 1) {
          // ...
      }
      return;
  }
  ```

### 3.8. Scoping

Cairo-M uses lexical block scoping with curly braces `{}`.

```rust
fn scoping_example() {
    let outer = 1;
    {
        let inner = 2;
        // 'outer' and 'inner' are both visible here.
    }
    // 'inner' is not visible here. 'outer' is.
    return ();
}
```

### 3.9. Tuple Destructuring

You can unpack tuples into variables using `let`.

```rust
fn destructuring() {
    // Unpack a tuple literal
    let (a, b) = (10, 20);

    // Unpack from a function call
    let (c, d) = get_pair();
}
```

Member access and assignment use positional fields (`.0`, `.1`, ...):

```cairo
fn tuple_access_and_assign() -> felt {
    let t = (1, 2, 3);
    t.0 = 4;
    return t.0 + t.1 + t.2; // 4 + 2 + 3
}
```

### 3.10. Arrays (Fixed-Size)

- Declaration: `[T; N]` for `felt` and `u32` elements.
- Repetition: `[elem; N]` (e.g. `[0u32; 3]`).
- Indexing: `arr[index]` where `index` is a `felt` expression.
- Assignment: `arr[i] = value;` supported.
- In parameters: arrays are passed by pointer (mutations affect caller).
- Const arrays: constant indices are folded at compile time.
- Bounds: no runtime bounds checks.

```cairo
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

- Arrays with aggregates: arrays can contain tuples and structs, and be nested.

```cairo
struct Point { x: u32, y: u32 }

fn arrays_with_aggregates() -> u32 {
    let points: [Point; 2] = [
        Point { x: 1, y: 2 },
        Point { x: 3, y: 4 },
    ];
    // Update and access nested fields through indexing
    points[0].x = 10;
    return points[0].x + points[1].y; // 10 + 4
}
```

### 3.11. Type Casts

- Supported: `u32` to `felt` via `as`.
- Not supported: other casts (e.g. `felt` to `u32`).
- Safety: casting checks the value is strictly less than `P = 2^31 - 1`. If not,
  it will panic.

```cairo
fn u32_to_felt_ok() -> felt {
    let x: u32 = 2147483646; // P - 1
    let y: felt = x as felt;
    return y;
}

//! error: compilation
fn felt_to_u32_error() -> u32 {  // not supported yet
    let x: felt = 10;
    let y: u32 = x as u32;
    return y;
}
```

### 3.12. Pointers and Heap Allocation

Pointer types `T*` and heap allocation via `new` are supported. Indexing through
pointers uses the same `p[i]` syntax as arrays.

- Allocate felt/u32 buffers: `let p: felt* = new felt[N];`
- Read/write via indexing: `p[i] = ...; let x = p[i];`
- Struct pointers: `let ps: Point* = new Point[N]; ps[0] = Point { ... };`

```cairo-m
// Allocate and use a felt buffer
fn alloc_felt_sum() -> felt {
    let p: felt* = new felt[3];
    p[0] = 7;
    p[1] = 8;
    p[2] = 9;
    return p[0] + p[1] + p[2];
}

// Allocate an array of structs and access fields
struct Point { x: u32, y: u32 }

fn alloc_points_total() -> u32 {
    let ps: Point* = new Point[2];
    ps[0] = Point { x: 1, y: 2 };
    ps[1] = Point { x: 3, y: 4 };
    return ps[0].x + ps[1].y; // 1 + 4
}
```

## 4. Not Yet Implemented

The following common language features are not yet implemented:

- Dynamic arrays/slices: no variable‑length arrays; only `[T; N]`.
- Type casting: only `u32 -> felt` is supported.
- Felt relational operators: `<`, `>`, `<=`, `>=` on `felt` are not enabled.

## 5. Other Notable Things

- Explicit `return`: Required in all functions, including unit `()`.
- Conditions are bool: `if`, `while`, `for` conditions must be `bool`. Use
  comparisons (e.g., `x == 0`) rather than numeric truthiness.
- Field division: division on `felt` is field division; division by zero panics.
- u32 math: wraps on overflow for all operations.
- Assertions: `assert(condition)` checks conditions at runtime; use with `bool`
  expressions.

## 6. Code Formatting

Cairo-M includes an integrated code formatter that automatically formats your
code for consistency and readability.

### 6.1. Using the Formatter in VS Code

The formatter is fully integrated into the VS Code extension:

- **Format Document**: Press `Shift+Alt+F` (Windows/Linux) or `Shift+Option+F`
  (macOS)
- **Format Selection**: Select code and use Command Palette → "Cairo-M: Format
  Selection"
- **Format on Save**: Enable automatic formatting when saving files via Command
  Palette → "Cairo-M: Toggle Format On Save"

### 6.2. What the Formatter Does

The formatter automatically:

- Adds consistent spacing around operators and punctuation
- Properly indents code blocks and nested structures
- Formats function signatures and parameter lists
- Aligns struct fields and maintains consistent brace style
- Preserves all comments in their original positions
- Ensures consistent line breaks and wrapping

Example transformation:

```cairo
// Before formatting
fn   calculate(x:felt,y:felt)->felt{
let result=x+y;
return result;}

// After formatting
fn calculate(x: felt, y: felt) -> felt {
    let result = x + y;
    return result;
}
```

## 7. Compiling your programs

To compile your program, run the compiler and point it to your project's root
directory:

```bash
cargo run --release -p cairo-m-compiler -- --input /path/to/my_project -o project_compiled.json
```

Once you have your compiled program, you can run it with the runner:

```bash
cargo run --release -p cairo-m-runner -- project_compiled.json --entrypoint main [-a <arguments>]
```

And prove it with the prover:

```bash
cargo run --release -p cairo-m-prover -- --input project_compiled.json --entrypoint main [-a <arguments>] --output proof.json
```

Notes for runner arguments:

- Supported input types: numbers (felt), booleans, tuples, and structs.
- Fixed‑size arrays are not yet supported as CLI inputs.
