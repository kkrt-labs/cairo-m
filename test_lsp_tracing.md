# Cairo-M LSP Tracing Guide

## Overview

The Cairo-M language server now includes focused tracing that shows compilation
flow without overwhelming detail.

## Viewing Logs

1. **Open VSCode with the Cairo-M extension**
2. **Open the Output panel** (View → Output)
3. **Select "Cairo-M Language Server" from the dropdown**

## Log Messages You'll See

### When opening a file:

```
[DIAGNOSTICS] Starting validation for /path/to/main.cm
[PROJECT] Creating new project for /path/to/project
[PARSER] Parsing file: /path/to/main.cm
[PARSER] Parse complete for /path/to/main.cm: 5 items
[PARSER] Parsing file: /path/to/math.cm
[PARSER] Parse complete for /path/to/math.cm: 3 items
[SEMANTIC] Starting project validation
[SEMANTIC] Building project semantic index for 2 modules
[SEMANTIC] Building semantic index for module: main
[SEMANTIC] Semantic index built for module 'main': 8 definitions, 12 identifier usages
[SEMANTIC] Building semantic index for module: math
[SEMANTIC] Semantic index built for module 'math': 3 definitions, 4 identifier usages
[SEMANTIC] Project semantic index complete
[SEMANTIC] Validating module: main
[SEMANTIC] Module 'main' validation complete: 0 diagnostics
[SEMANTIC] Validating module: math
[SEMANTIC] Module 'math' validation complete: 1 diagnostics
[SEMANTIC] Project validation complete: 1 total diagnostics
[DIAGNOSTICS] Validated module 'main' (/path/to/main.cm): 1 diagnostics
```

### When typing in a file:

```
[DIAGNOSTICS] Starting validation for /path/to/main.cm
[PROJECT] Using cached project for /path/to/project  <-- Project is reused!
[PARSER] Parsing file: /path/to/main.cm             <-- Only the changed file is reparsed
[PARSER] Parse complete for /path/to/main.cm: 5 items
[SEMANTIC] Starting project validation
... (only rebuilds what changed)
```

## What This Tells You

1. **Incremental Compilation Working**:

   - If you see "[PROJECT] Using cached project", the project structure is
     reused
   - Only modified files show "[PARSER] Parsing file"
   - Unchanged modules don't rebuild their semantic index

2. **Performance Issues**:

   - If every keystroke causes full project rebuild, something's wrong
   - Multiple "[PROJECT] Creating new project" indicates project cache issues
   - All modules being reparsed on each change suggests Salsa caching problem

3. **Compilation Flow**:
   - Parser → Semantic Index → Validation → Diagnostics
   - Number of definitions and usages helps track code complexity
   - Diagnostic counts show where errors are occurring

## Adjusting Log Verbosity

The default shows only Cairo-M info logs. To see more detail:

```bash
# Show debug-level Cairo-M logs (more detail)
export RUST_LOG=cairo_m=debug

# Show only warnings and errors (less noise)
export RUST_LOG=cairo_m=warn

# Default (recommended)
export RUST_LOG=cairo_m=info
```

Note: Salsa's internal logs are filtered out to reduce noise.
