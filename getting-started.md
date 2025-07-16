# Getting started with Cairo-M

This guide covers how to install, configure, and use the Cairo-M Language Server
and VS Code extension, along with a comprehensive overview of the current
language features.

## 1. Installation & Configuration

### 1.1. Installing the Language Server & VS Code Extension

Currently, the recommended way to install is by building from the source.

**Prerequisites:**

- VS Code (1.75.0 or higher)
- Rust Toolchain (`rustup`)
- Node.js and `npm`

#### Step 1: Clone the Repository

```bash
git clone <repository-url>
cd cairo-m
```

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

# This creates a file like `cairo-m-0.0.1.vsix`
```

#### Step 4: Install in VS Code

1.  Open VS Code.
2.  Go to the **Extensions** view (Ctrl+Shift+X).
3.  Click the "..." menu at the top-right and select **Install from VSIX...**.
4.  Choose the `.vsix` file you just created.
5.  Restart VS Code when prompted.

### 1.2. Configuring the VS Code Extension

If the extension doesn't find the language server automatically, you must set
the path manually.

1.  Open VS Code settings (Ctrl+,).
2.  Search for "Cairo-M".
3.  Find the **Cairo-m: Language Server: Path** setting and set it to the
    absolute path of the `cairo-m-ls` binary you built.

Alternatively, you can add it to your `.vscode/settings.json` file:

```json
{
  // Example for Linux/macOS
  "cairo-m.languageServer.path": "/path/to/your/cairo-m/target/release/cairo-m-ls"
}
```

## 2. Project Setup

### 2.1. Project Structure

A Cairo-M project is defined by a `cairom.toml` manifest file at its root. The
source code resides in a `src` directory.

```text
my_project/
├── cairom.toml       # The project manifest file
└── src/              # Source code directory
    ├── main.cm       # Main entry point (or lib.cm for libraries)
    └── utils/
        └── math.cm   # A submodule
```

### 2.2. Manifest File (`cairom.toml`)

The manifest file configures your project.

```toml
# The name of your project (required)
name = "my_project"

# The version of your project (defaults to "0.1.0")
version = "0.1.0"

# The main entry point file, relative to the src/ directory.
# If not specified, the compiler will look for "src/main.cm" or "src/lib.cm".
# This is subject to future changes.
entry_point = "main.cm"
```

### 2.3. Module System

The file structure within the `src` directory directly maps to the module
hierarchy.

- `src/main.cm` -> module `main`
- `src/utils.cm` -> module `utils`
- `src/utils/math.cm` -> module `utils::math`

You can import items from other modules using the `use` keyword.

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

- **Variables**: Declared with `let`. They can be reassigned (mutable by
  default) and shadowed.
- **Constants**: Declared with `const`. The value must be a compile-time
  constant expression.

```rust
fn variables_and_constants() {
    // Variables
    let x = 10;          // Type 'felt' is inferred
    let y: felt = 20;    // With explicit type annotation
    x = 30;              // Variables can be reassigned

    // Constants
    const PI = 314;
    const MAX_VALUE = PI * 2;
    return();
}
```

- **Shadowing**: You can declare a new variable with the same name as a previous
  one, which "shadows" it.

```rust
fn shadowing() {
    let x = 5;
    {
        let x = 10; // This x shadows the outer x
        // Here, x is 10
    }
    // Here, x is 5
    return();
}
```

### 3.3. Data Types

| Type     | Description                                    | Example Literal        |
| -------- | ---------------------------------------------- | ---------------------- |
| `felt`   | The primary numeric type (a field element).    | `42`, `0`, `100`       |
| `bool`   | A boolean value.                               | `true`, `false`        |
| Tuples   | A fixed-size collection of values of any type. | `(10, true)`           |
| Pointers | A pointer to a value in memory.                | ❌ Not implemented yet |

```rust
struct Point { x: felt, y: felt }

fn data_types() {
    let num: felt = 42;
    let is_active: bool = true;
    let pair: (felt, bool) = (10, false);

    // A pointer to a Point struct
    let p_ptr: Point*;

    // Single-element tuples require a trailing comma
    let single: (felt,) = (100,);
    return();
}
```

### 3.4. Operators

**Arithmetic Operators** (for `felt`) | Operator | Description |
|----------|-------------| | `+` | Addition | | `-` | Subtraction | | `*` |
Multiplication| | `/` | Division ⚠️ Felt division ! | | `-` (unary) | Negation |

**Comparison Operators** (compares `felt`, returns `bool`) | Operator |
Description | |----------|-------------| | `==` | Equal | | `!=` | Not Equal |

❌ Not implemented yet | `<` | Less Than | | `>` | Greater Than| | `<=` | Less
or Equal| | `>=` | Greater or Equal|

**Logical Operators** (for `bool`) | Operator | Description |
|----------|-------------| | `&&` | Logical AND | | `||` | Logical OR | | `!` |
Logical NOT |

### 3.5. Functions

Functions are declared with `fn`. Types for all parameters and the return value
must be specified. A function that returns nothing has a unit type `()`, which
can be omitted from the signature.

**All functions require an explicit `return` statement, even for the unit
type.**

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
    return();
}
```

### 3.7. Control Flow

- **`if-else`**: The condition must be a `bool`.

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

- **`loop`**: Creates an infinite loop, exited with `break`.
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
- **`while`**: Loops as long as a `bool` condition is `true`.
  ```rust
  fn while_loop() {
      let x = 0;
      while (x !- 10) {
          x = x + 1;
      }
      return ();
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

## 4. Not Yet Implemented

The following common language features are not yet implemented:

- **`for` loops**: The `for` keyword is reserved but not yet semantically
  supported - Soon ✨.
- **Mutability Control**: There is no `mut` keyword. Variables are mutable by
  default, but this may change.
- **Dynamic Arrays / Slices**: No support for dynamic-sized collections or array
  literals like `[1, 2, 3]`.
- **Enums**: No `enum` type for defining variants.
- **Traits / Interfaces**: No system for defining shared behavior across types.
- **Generics**: No support for type-parameterized functions or structs.
- **Type Casting**: No explicit `as` keyword for casting between types.
- **String Literals**: No support for `"hello, world"` style strings.

## 5. Other Notable Things

- **Explicit `return` is Mandatory**: All functions must end with an explicit
  `return` statement, even if they return the unit type `()`. There is no
  implicit return of the last expression.
- **`bool` vs `felt` for Conditions**: Conditions for `if` and `while` must
  evaluate to a `bool`. You cannot use a `felt` like `0` or `1` directly. Use
  comparison operators like `x == 0` to produce a `bool`. In the future this
  will be doable with the `as` keyword.
