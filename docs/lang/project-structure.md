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

#### Example Structure:

```
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

#### Import Syntax

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

The CairoM compiler automatically discovers all `.cm` files within a project and
compiles them together. The entry point is typically `main.cm`.

To compile, run the compiler and point it to your project's root directory.

```bash
cairo-m-compiler --input /path/to/my_project
```

Alternatively, you can point to any file within the project, and the compiler
will find the project root by looking for `cairom.toml` in parent directories.

```bash
cairo-m-compiler --input /path/to/my_project/src/main.cm
```

The compiler will produce a single JSON output file containing the compiled
program, including all code from every module in the project. Any compilation
errors or warnings from any file will be reported.
