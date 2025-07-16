# Working with CairoM Projects

This guide explains the structure of a CairoM project (a "crate"), how to define
and use modules, and how to compile a multi-file project.

## 1. Crate & File Structure

A CairoM project, or "crate", is organized in a directory. While the compiler
can discover files in a flat structure, the recommended layout uses a manifest
file and a `src` directory.

- **`cairom.toml`**: An empty file at the project's root that acts as a
  manifest. Its presence defines the project root.
- **`src/`**: A directory containing all your CairoM source files (`.cm`).

Each `.cm` file inside the `src` directory is treated as a separate **module**.

While it's possible to compile single files outside of a crate, a crate **must**
have a root module: either `main.cm` or `lib.cm`.

## The Crate Manifest

The `cairom.toml` file is a manifest file that defines the project. It is used
to define the project name, version, and entry point.

```toml
name = "cairo-m-project"
version = "0.1.0"
entry_point = "main.cm"
```

The `name` field is the name of the project. The `version` field is the version
of the project. The `entry_point` field is the entry point of the project.

> Note: The `entry_point` might be removed in the future.

## Example Structure

```text
my_project/
├── cairom.toml       (Project manifest)
└── src/
    ├── main.cm       (Main crate entry point)
    └── math.cm       (A library module)
```

## 2. Module System & Resolution

CairoM uses a file-based module system. Each source file is a module, and its
name is derived directly from the filename without the `.cm` extension.

- `src/main.cm` becomes the `main` module.
- `src/math.cm` becomes the `math` module.

You can import items (like functions and structs) from other modules using the
`use` statement.

### Import Syntax

To import the `add` function from the `math` module:

```cairo
// In src/main.cm
use math::add;

func main() -> felt {
    return add(1, 2);
}
```

To import multiple items from the same module, use curly braces `{}`:

```cairo
// In src/main.cm
use math::{add, sub};

func main() -> felt {
    let a = add(5, 3);
    return sub(a, 1);
}
```

**Module Resolution:** The compiler maps module paths to file paths:

- `use math::add` → looks for `math.cm` in the project root
- `use utils::helpers::format` → looks for `utils/helpers/format.cm`
- `use core::ops::add` → looks for `core/ops/add.cm`

If the file doesn't exist at the expected path, you'll get an "unresolved
import" error.

## 3. How to Start a New Project

Follow these steps to create a new project.

1.  **Create the project directory and manifest:**

    ```bash
    mkdir my_project
    cd my_project
    touch cairom.toml
    ```

2.  **Create the source directory and files:**

    ```bash
    mkdir src
    touch src/main.cm
    touch src/math.cm
    ```

3.  **Add code to your modules.**

    **`src/math.cm`**:

    ```cairo
    func add(a: felt, b: felt) -> felt {
        return a + b;
    }

    func sub(a: felt, b: felt) -> felt {
        return a - b;
    }
    ```

    **`src/main.cm`**:

    ```cairo
    use math::{add, sub};

    func main() -> felt {
        let x = add(10, 5);
        let y = sub(x, 3);
        return y;
    }
    ```

## 4. How to Compile a Project

The CairoM compiler automatically discovers and compiles all `.cm` files within
a project. The entry point is typically `main.cm` or `lib.cm`.

To compile, run the compiler and point it to your project's root directory:

```bash
cairo-m-compiler --input /path/to/my_project
```

Alternatively, you can point to any file within the project, and the compiler
will find the project root by looking for `cairom.toml` in parent directories:

```bash
cairo-m-compiler --input /path/to/my_project/src/main.cm
```

The compiler will:

1. Discover all `.cm` files in the project
2. Identify the entry point (`main.cm` or `lib.cm`)
3. Parse and analyze all modules
4. Report errors for any issues found (including unused modules)
5. Produce a single JSON output file containing the compiled program

All modules in the project are validated and included in the compilation,
ensuring comprehensive error checking across your entire codebase.
