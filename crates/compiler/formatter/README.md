# Cairo-M Formatter

A code formatter for the Cairo-M language, inspired by rustfmt and designed to
automatically format Cairo-M code for consistency and readability.

## Features

- **Full AST-based formatting**: Parses and formats Cairo-M code using the
  compiler's AST for accurate representation
- **Comment preservation**: Maintains all comments (file-level, inline, and
  end-of-line) in their proper positions
- **Idempotent**: Running the formatter multiple times produces the same result
- **LSP integration**: Seamlessly integrated with the Cairo-M Language Server
- **VSCode integration**: Available via standard formatting commands and
  keybindings
- **Configurable**: Customizable formatting options (line width, indent width,
  trailing commas)

## Usage

### Via VSCode (Recommended)

The formatter is fully integrated into the Cairo-M VSCode extension:

- **Format Document**: Press `Shift+Alt+F` (Windows/Linux) or `Shift+Option+F`
  (macOS)
- **Format Selection**: Select code and use Command Palette → "Cairo-M: Format
  Selection"
- **Format on Save**: Command Palette → "Cairo-M: Toggle Format On Save"
- **Command Palette**: Access all formatter commands via `Ctrl+Shift+P`

### Via Language Server

The formatter is available through any LSP-compatible editor:

- Send a `textDocument/formatting` request to the Cairo-M language server
- The server will return formatted text as LSP `TextEdit` objects

### Programmatic Usage

```rust
use cairo_m_compiler_parser::{ParserDatabaseImpl, SourceFile};
use cairo_m_formatter::{format_source_file, FormatterConfig};

// Create a parser database
let db = ParserDatabaseImpl::default();

// Load your source code
let source = SourceFile::new(&db, code.to_string(), "example.cm".to_string());

// Configure formatting options (or use defaults)
let config = FormatterConfig::default();

// Format the code
let formatted = format_source_file(&db, source, &config);
println!("{}", formatted);
```

## Configuration

### VSCode Settings

Configure the formatter through VSCode settings:

```json
{
  // Maximum line width before wrapping
  "cairo-m.format.maxWidth": 100,

  // Number of spaces per indentation level
  "cairo-m.format.indentWidth": 4,

  // Add trailing commas to multi-line constructs
  "cairo-m.format.trailingComma": false
}
```

### Default Configuration

```rust
FormatterConfig {
    max_width: 100,      // Maximum line length
    indent_width: 4,     // Spaces per indent level
    trailing_comma: false, // Trailing commas in multi-line constructs
}
```

## Formatting Examples

### Functions

**Before:**

```cairo
fn   badly_formatted(x:felt,y:felt)->felt{
let result=x+y;
return result;}
```

**After:**

```cairo
fn badly_formatted(x: felt, y: felt) -> felt {
    let result = x + y;
    return result;
}
```

### Structs

**Before:**

```cairo
struct   Point{x:felt,y:felt,}
```

**After:**

```cairo
struct Point {
    x: felt,
    y: felt,
}
```

### Comments

**Before:**

```cairo
// File comment
fn main(){
// Function comment
let x=1;// Inline comment
return x;}
```

**After:**

```cairo
// File comment
fn main() {
    // Function comment
    let x = 1; // Inline comment
    return x;
}
```

## Supported Constructs

The formatter handles all Cairo-M language constructs:

### Top-Level Items

- **Functions**: Parameter lists, return types, bodies with proper indentation
- **Structs**: Field formatting with optional trailing commas
- **Constants**: Value alignment and type annotations
- **Use statements**: Import path formatting

### Statements

- **Let bindings**: Pattern destructuring, type annotations, initializers
- **Assignments**: Proper spacing around `=`
- **Control flow**: `if/else`, `while`, `loop`, `for` with consistent bracing
- **Returns**: `return` statement formatting
- **Break/Continue**: Loop control statements

### Expressions

- **Literals**: Numbers, booleans, with proper spacing
- **Binary operations**: Consistent spacing around operators
- **Unary operations**: Prefix operator formatting
- **Function calls**: Argument list formatting
- **Member access**: Dot notation spacing
- **Tuples**: Multi-line tuple formatting
- **Struct literals**: Field formatting with line breaks

### Type Expressions

- **Named types**: Simple type references
- **Pointers**: `Type*` formatting
- **Tuples**: `(Type1, Type2)` formatting

### Patterns

- **Identifiers**: Variable binding patterns
- **Tuple destructuring**: `(a, b)` pattern formatting

## Architecture

The formatter uses a three-phase approach:

### 1. Parsing Phase

- Leverages the Cairo-M parser to build a complete AST
- Extracts all comments from the source text
- Associates comments with AST nodes using span information

### 2. Formatting Phase

- Converts AST nodes to an intermediate `Doc` representation
- Each AST node implements the `Format` trait
- Comments are attached to their associated nodes
- The `Doc` tree represents formatting decisions abstractly

### 3. Rendering Phase

- Renders the `Doc` tree to a string
- Applies line breaking decisions based on available width
- Handles indentation consistently
- Preserves comment positions accurately

### Key Components

- **`Doc`**: Abstract document representation supporting composition
- **`Format` trait**: Implemented by all formattable AST nodes
- **`FormatterCtx`**: Maintains formatting state and configuration
- **`CommentAttachment`**: Associates comments with AST nodes using spans
- **`Renderer`**: Converts `Doc` trees to formatted strings

## Testing

The formatter includes comprehensive test coverage:

```bash
# Run all formatter tests
cargo test -p cairo-m-formatter

# Run specific test categories
cargo test -p cairo-m-formatter comment_tests
cargo test -p cairo-m-formatter formatter_tests

# Review snapshot changes
cargo insta review
```

### Test Categories

- **Snapshot tests**: Compare formatted output against expected results
- **Idempotence tests**: Ensure formatting is stable
- **Comment preservation tests**: Verify comments remain in correct positions
- **Edge case tests**: Handle unusual but valid code patterns

## Future Enhancements

- **Configuration file**: Support `.cairo-m-fmt.toml` for project-specific
  settings
- **Import grouping**: Intelligent organization of `use` statements
- **Alignment options**: Align similar constructs (e.g., struct fields)
- **Style options**: Alternative formatting styles (K&R, Allman, etc.)
- **Performance optimizations**: Faster formatting for large files
- **Partial formatting**: More efficient range-based formatting

## Contributing

When contributing to the formatter:

1. Add tests for new formatting rules
2. Ensure all existing tests pass
3. Run the formatter on the Cairo-M codebase to verify behavior
4. Update snapshots if formatting changes are intentional
5. Document any new configuration options
