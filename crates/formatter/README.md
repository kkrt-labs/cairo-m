# Cairo-M Formatter

A code formatter for the Cairo-M language, inspired by rustfmt.

## Features

- **Full AST-based formatting**: Parses and formats Cairo-M code using the
  compiler's AST
- **Idempotent**: Running the formatter multiple times produces the same result
- **LSP integration**: Available via "Format Document" in VSCode
- **Configurable**: Line width, indent width, and other options (currently uses
  defaults)

## Usage

### Via Language Server (Recommended)

The formatter is integrated into `cairo-m-ls` and available through your
editor's formatting commands:

- **VSCode**: Use `Format Document` (usually Shift+Alt+F)
- **Other editors**: Use your editor's standard formatting command

### Programmatic Usage

```rust
use cairo_m_compiler_parser::{ParserDatabaseImpl, SourceFile};
use cairo_m_formatter::{format_source_file, FormatterConfig};

let db = ParserDatabaseImpl::default();
let source = SourceFile::new(&db, code.to_string(), "example.cm".to_string());
let config = FormatterConfig::default();
let formatted = format_source_file(&db, source, &config);
```

## Configuration

The formatter uses these default settings:

- **Max line width**: 100 characters
- **Indent width**: 4 spaces
- **Trailing commas**: Disabled
- **Line endings**: Auto-detect

## Supported Constructs

The formatter handles all Cairo-M language constructs:

- **Items**: Functions, structs, namespaces, constants, use statements
- **Statements**: Let bindings, assignments, if/else, loops, for loops, return,
  break, continue
- **Expressions**: Literals, identifiers, binary/unary operations, function
  calls, member access, tuples, struct literals
- **Types**: Named types, pointers, tuples
- **Patterns**: Identifiers, tuple destructuring

## Architecture

The formatter uses a pretty-printing approach based on Wadler's "A Prettier
Printer" algorithm:

1. **Parse**: Uses the Cairo-M parser to build an AST
2. **Format**: Converts AST nodes to an intermediate `Doc` representation
3. **Render**: Renders the `Doc` tree to a string with proper line breaks and
   indentation

## Future Enhancements

- **Comment preservation**: Currently, comments are not preserved during
  formatting
- **Configuration file**: Support `.cairo-m-fmt.toml` for project-specific
  settings
- **Range formatting**: More sophisticated partial formatting
- **Style options**: Alternative formatting styles (e.g., different brace
  styles)

## Testing

The formatter includes comprehensive snapshot tests using `insta`:

```bash
cargo test -p cairo-m-formatter
```

To review snapshot changes:

```bash
cargo insta review
```
