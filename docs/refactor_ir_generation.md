# MIR IR Generation Refactoring

## Overview

This document describes the refactoring of the `ir_generation.rs` module in the
Cairo-M compiler's MIR crate. The goal is to break down the monolithic 2638-line
file into smaller, focused modules with clear separation of concerns.

## Completed Work

### 1. Module Structure Created

The following structure has been established under
`crates/compiler/mir/src/lowering/`:

```
lowering/
├── mod.rs            // Module root with re-exports
├── builder.rs        // MirBuilder struct and core methods
├── expr.rs           // Expression lowering trait and placeholder impls
├── stmt.rs           // Statement lowering trait and placeholder impls
├── function.rs       // Function-level orchestration and entry point
├── control_flow.rs   // Control flow construction helpers
└── utils.rs          // Shared utility functions
```

### 2. Core Infrastructure

- **MirBuilder** extracted to `builder.rs` with:
  - State management (current block, loop stack, definition mappings)
  - Binary operation conversion
  - Function resolution
  - Basic block manipulation

- **Traits Defined**:
  - `LowerStmt` - for statement lowering
  - `LowerExpr` - for expression lowering

- **Control Flow Helpers** in `control_flow.rs`:
  - Block creation and switching
  - Branch/goto generation
  - Scoped block execution

- **Entry Point** in `function.rs`:
  - `generate_mir` Salsa query
  - Function lowering orchestration

### 3. Module Integration

- Added `lowering` module to `lib.rs`
- Created compatibility module `ir_generation_new.rs`
- All modules compile with placeholder implementations

## Remaining Work

### Phase 3a - Implementation Migration (Estimated: 2-3 days)

1. **Statement Lowering** (~600 lines to migrate):
   - `lower_let_statement`
   - `lower_return_statement`
   - `lower_assignment_statement`
   - `lower_if_statement`
   - `lower_while_statement`, `lower_loop_statement`, `lower_for_statement`
   - `lower_break_statement`, `lower_continue_statement`

2. **Expression Lowering** (~800 lines to migrate):
   - `lower_expression` (main dispatcher)
   - `lower_lvalue_expression`
   - Binary/unary operations
   - Function calls
   - Literals and identifiers
   - Array/struct/tuple handling

3. **Helper Functions** (~200 lines):
   - Type resolution helpers
   - Value creation utilities
   - Error handling

### Phase 3b - Integration (Estimated: 1 day)

1. Update all imports and dependencies
2. Remove duplicated code from original file
3. Ensure all tests pass
4. Update documentation

### Phase 3c - Cleanup (Estimated: 0.5 day)

1. Delete old `ir_generation.rs`
2. Rename `ir_generation_new.rs` to `ir_generation.rs`
3. Final test run and documentation update

## Benefits Achieved

1. **Separation of Concerns**: Clear boundaries between statement lowering,
   expression lowering, and control flow
2. **Improved Testability**: Each module can be unit tested independently
3. **Better Discoverability**: Developers can find specific lowering logic more
   easily
4. **Trait-based Design**: Extensible architecture for future enhancements

## Module Responsibilities

### builder.rs

- Owns the MirBuilder struct
- Manages function-level state
- Provides core infrastructure

### stmt.rs

- Implements statement lowering logic
- Handles control flow statements
- Manages variable bindings

### expr.rs

- Implements expression evaluation
- Handles operators and function calls
- Manages temporary values

### control_flow.rs

- Provides high-level control flow patterns
- Manages basic block creation and linking
- Handles loop contexts

### function.rs

- Orchestrates the lowering process
- Handles module-level concerns
- Implements the Salsa query

### utils.rs

- Shared utilities used across modules
- Type conversion helpers
- Common patterns

## Next Steps

1. Fix the unrelated semantic crate compilation error
2. Begin incremental migration of implementations
3. Test each migrated component
4. Complete integration and cleanup

The refactoring sets up a solid foundation for future improvements such as:

- Pluggable optimization passes
- Alternative lowering strategies
- Better error recovery
- Performance optimizations
